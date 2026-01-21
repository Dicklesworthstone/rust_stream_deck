//! SQLite database operations for snapshot storage.
//!
//! Provides persistent storage for device state snapshots with
//! content-addressable image caching.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use tracing::{debug, info, instrument, trace};

use super::schema::{CachedImage, KeyState, Snapshot, SnapshotKey, SnapshotSummary};
use crate::error::{Result, SdError};

/// SQLite schema for snapshot storage.
const SCHEMA_SQL: &str = r#"
-- Snapshot metadata
CREATE TABLE IF NOT EXISTS snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    device_model TEXT NOT NULL,
    device_serial TEXT,
    key_count INTEGER NOT NULL,
    key_width INTEGER NOT NULL,
    key_height INTEGER NOT NULL,
    brightness INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER DEFAULT 1
);

-- Per-key state within snapshot
CREATE TABLE IF NOT EXISTS snapshot_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    key_index INTEGER NOT NULL,
    state_type TEXT NOT NULL,
    source_path TEXT,
    image_hash TEXT,
    color_hex TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(snapshot_id, key_index)
);

-- Image cache metadata
CREATE TABLE IF NOT EXISTS images (
    hash TEXT PRIMARY KEY,
    original_path TEXT,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    format TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    access_count INTEGER DEFAULT 1
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_snapshot_keys_snapshot ON snapshot_keys(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_snapshot_keys_hash ON snapshot_keys(image_hash);
CREATE INDEX IF NOT EXISTS idx_images_accessed ON images(last_accessed_at);
"#;

/// Database wrapper for snapshot storage.
pub struct SnapshotDb {
    conn: Connection,
}

impl SnapshotDb {
    /// Opens or creates a database at the standard location.
    ///
    /// Location: `~/.local/share/sd/snapshots/snapshots.db`
    #[instrument]
    pub fn open_default() -> Result<Self> {
        let path = default_db_path()?;
        Self::open(&path)
    }

    /// Opens or creates a database at the given path.
    #[instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SdError::Other(format!(
                    "Failed to create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

        debug!(path = %path.display(), "Opening snapshot database");
        let conn = Connection::open(path).map_err(|e| {
            SdError::Other(format!("Failed to open database: {e}"))
        })?;

        let db = Self { conn };
        db.init_schema()?;
        info!(path = %path.display(), "Snapshot database ready");
        Ok(db)
    }

    /// Creates an in-memory database (useful for testing).
    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            SdError::Other(format!("Failed to create in-memory database: {e}"))
        })?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initializes the database schema.
    fn init_schema(&self) -> Result<()> {
        // Enable foreign keys
        self.conn.execute("PRAGMA foreign_keys = ON", []).map_err(|e| {
            SdError::Other(format!("Failed to enable foreign keys: {e}"))
        })?;

        self.conn.execute_batch(SCHEMA_SQL).map_err(|e| {
            SdError::Other(format!("Failed to initialize schema: {e}"))
        })?;
        Ok(())
    }

    /// Saves a snapshot to the database.
    ///
    /// If a snapshot with the same name exists, it will be updated.
    #[instrument(skip(self, snapshot), fields(name = %snapshot.name))]
    pub fn save_snapshot(&mut self, snapshot: &Snapshot) -> Result<i64> {
        let tx = self.conn.transaction().map_err(|e| {
            SdError::Other(format!("Failed to start transaction: {e}"))
        })?;

        let now = Utc::now().to_rfc3339();

        // Check if snapshot exists
        let existing_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM snapshots WHERE name = ?1",
                params![snapshot.name],
                |row| row.get(0),
            )
            .ok();

        let snapshot_id = if let Some(id) = existing_id {
            // Update existing snapshot
            debug!(id, "Updating existing snapshot");
            tx.execute(
                "UPDATE snapshots SET
                    description = ?1,
                    device_model = ?2,
                    device_serial = ?3,
                    key_count = ?4,
                    key_width = ?5,
                    key_height = ?6,
                    brightness = ?7,
                    updated_at = ?8
                 WHERE id = ?9",
                params![
                    snapshot.description,
                    snapshot.device_model,
                    snapshot.device_serial,
                    snapshot.key_count,
                    snapshot.key_width,
                    snapshot.key_height,
                    snapshot.brightness,
                    now,
                    id,
                ],
            )
            .map_err(|e| SdError::Other(format!("Failed to update snapshot: {e}")))?;

            // Delete existing keys
            tx.execute("DELETE FROM snapshot_keys WHERE snapshot_id = ?1", params![id])
                .map_err(|e| SdError::Other(format!("Failed to delete old keys: {e}")))?;

            id
        } else {
            // Insert new snapshot
            debug!("Inserting new snapshot");
            tx.execute(
                "INSERT INTO snapshots (name, description, device_model, device_serial, key_count, key_width, key_height, brightness, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    snapshot.name,
                    snapshot.description,
                    snapshot.device_model,
                    snapshot.device_serial,
                    snapshot.key_count,
                    snapshot.key_width,
                    snapshot.key_height,
                    snapshot.brightness,
                    now,
                    now,
                ],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert snapshot: {e}")))?;

            tx.last_insert_rowid()
        };

        // Insert keys
        for key in &snapshot.keys {
            let (state_type, source_path, image_hash, color_hex) = match &key.state {
                KeyState::Image { source_path, image_hash } => (
                    "image",
                    source_path.as_ref().map(|p| p.display().to_string()),
                    Some(image_hash.clone()),
                    None,
                ),
                KeyState::Color { hex } => ("color", None, None, Some(hex.clone())),
                KeyState::Clear => ("clear", None, None, None),
            };

            trace!(key_index = key.key_index, state_type, "Inserting snapshot key");
            tx.execute(
                "INSERT INTO snapshot_keys (snapshot_id, key_index, state_type, source_path, image_hash, color_hex, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![snapshot_id, key.key_index, state_type, source_path, image_hash, color_hex, now],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert snapshot key: {e}")))?;
        }

        tx.commit().map_err(|e| {
            SdError::Other(format!("Failed to commit transaction: {e}"))
        })?;

        info!(name = %snapshot.name, id = snapshot_id, keys = snapshot.keys.len(), "Snapshot saved");
        Ok(snapshot_id)
    }

    /// Loads a snapshot by name.
    #[instrument(skip(self))]
    pub fn load_snapshot(&self, name: &str) -> Result<Option<Snapshot>> {
        // Load snapshot metadata
        let row: Option<(i64, String, Option<String>, String, Option<String>, u8, u32, u32, Option<u8>, String, String)> = self
            .conn
            .query_row(
                "SELECT id, name, description, device_model, device_serial, key_count, key_width, key_height, brightness, created_at, updated_at
                 FROM snapshots WHERE name = ?1",
                params![name],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                    ))
                },
            )
            .ok();

        let Some((id, name, description, device_model, device_serial, key_count, key_width, key_height, brightness, created_at, updated_at)) = row else {
            debug!(name, "Snapshot not found");
            return Ok(None);
        };

        // Parse timestamps
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| SdError::Other(format!("Invalid created_at timestamp: {e}")))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| SdError::Other(format!("Invalid updated_at timestamp: {e}")))?
            .with_timezone(&Utc);

        // Load keys
        let mut stmt = self
            .conn
            .prepare("SELECT key_index, state_type, source_path, image_hash, color_hex FROM snapshot_keys WHERE snapshot_id = ?1 ORDER BY key_index")
            .map_err(|e| SdError::Other(format!("Failed to prepare statement: {e}")))?;

        let keys: Vec<SnapshotKey> = stmt
            .query_map(params![id], |row| {
                let key_index: u8 = row.get(0)?;
                let state_type: String = row.get(1)?;
                let source_path: Option<String> = row.get(2)?;
                let image_hash: Option<String> = row.get(3)?;
                let color_hex: Option<String> = row.get(4)?;

                let state = match state_type.as_str() {
                    "image" => KeyState::Image {
                        source_path: source_path.map(PathBuf::from),
                        image_hash: image_hash.unwrap_or_default(),
                    },
                    "color" => KeyState::Color {
                        hex: color_hex.unwrap_or_default(),
                    },
                    _ => KeyState::Clear,
                };

                Ok(SnapshotKey { key_index, state })
            })
            .map_err(|e| SdError::Other(format!("Failed to query keys: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SdError::Other(format!("Failed to collect keys: {e}")))?;

        debug!(name = %name, keys = keys.len(), "Snapshot loaded");
        Ok(Some(Snapshot {
            id: Some(id),
            name,
            description,
            device_model,
            device_serial,
            key_count,
            key_width,
            key_height,
            brightness,
            created_at,
            updated_at,
            keys,
        }))
    }

    /// Lists all snapshots with summary information.
    #[instrument(skip(self))]
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotSummary>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, description, device_model, key_count, brightness, created_at, updated_at
                 FROM snapshots ORDER BY updated_at DESC",
            )
            .map_err(|e| SdError::Other(format!("Failed to prepare statement: {e}")))?;

        let summaries: Vec<SnapshotSummary> = stmt
            .query_map([], |row| {
                let created_at: String = row.get(6)?;
                let updated_at: String = row.get(7)?;

                Ok(SnapshotSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    device_model: row.get(3)?,
                    key_count: row.get(4)?,
                    brightness: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| SdError::Other(format!("Failed to query snapshots: {e}")))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| SdError::Other(format!("Failed to collect snapshots: {e}")))?;

        debug!(count = summaries.len(), "Listed snapshots");
        Ok(summaries)
    }

    /// Deletes a snapshot by name.
    ///
    /// Returns true if a snapshot was deleted, false if not found.
    #[instrument(skip(self))]
    pub fn delete_snapshot(&mut self, name: &str) -> Result<bool> {
        let deleted = self
            .conn
            .execute("DELETE FROM snapshots WHERE name = ?1", params![name])
            .map_err(|e| SdError::Other(format!("Failed to delete snapshot: {e}")))?;

        if deleted > 0 {
            info!(name, "Snapshot deleted");
            Ok(true)
        } else {
            debug!(name, "Snapshot not found for deletion");
            Ok(false)
        }
    }

    /// Checks if a snapshot exists by name.
    #[instrument(skip(self))]
    pub fn snapshot_exists(&self, name: &str) -> Result<bool> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM snapshots WHERE name = ?1",
                params![name],
                |_| Ok(true),
            )
            .unwrap_or(false);
        Ok(exists)
    }

    // === Image Cache Operations ===

    /// Saves image metadata to the cache.
    #[instrument(skip(self, image), fields(hash = %image.hash))]
    pub fn save_image(&self, image: &CachedImage) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO images (hash, original_path, width, height, format, size_bytes, created_at, last_accessed_at, access_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    image.hash,
                    image.original_path.as_ref().map(|p| p.display().to_string()),
                    image.width,
                    image.height,
                    image.format,
                    image.size_bytes,
                    image.created_at.to_rfc3339(),
                    now,
                    image.access_count,
                ],
            )
            .map_err(|e| SdError::Other(format!("Failed to save image: {e}")))?;

        trace!(hash = %image.hash, "Image metadata saved");
        Ok(())
    }

    /// Loads image metadata by hash.
    #[instrument(skip(self))]
    pub fn load_image(&self, hash: &str) -> Result<Option<CachedImage>> {
        let row: Option<(String, Option<String>, u32, u32, String, u64, String, String, u32)> = self
            .conn
            .query_row(
                "SELECT hash, original_path, width, height, format, size_bytes, created_at, last_accessed_at, access_count
                 FROM images WHERE hash = ?1",
                params![hash],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .ok();

        let Some((hash, original_path, width, height, format, size_bytes, created_at, last_accessed_at, access_count)) = row else {
            return Ok(None);
        };

        // Update access tracking
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE images SET last_accessed_at = ?1, access_count = access_count + 1 WHERE hash = ?2",
                params![now, hash],
            )
            .ok();

        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let last_accessed_at = DateTime::parse_from_rfc3339(&last_accessed_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some(CachedImage {
            hash,
            original_path: original_path.map(PathBuf::from),
            width,
            height,
            format,
            size_bytes,
            created_at,
            last_accessed_at,
            access_count,
        }))
    }

    /// Deletes orphaned images not referenced by any snapshot.
    #[instrument(skip(self))]
    pub fn cleanup_orphaned_images(&mut self) -> Result<usize> {
        let deleted = self
            .conn
            .execute(
                "DELETE FROM images WHERE hash NOT IN (SELECT DISTINCT image_hash FROM snapshot_keys WHERE image_hash IS NOT NULL)",
                [],
            )
            .map_err(|e| SdError::Other(format!("Failed to cleanup images: {e}")))?;

        if deleted > 0 {
            info!(deleted, "Orphaned images cleaned up");
        }
        Ok(deleted)
    }
}

/// Returns the default database path.
///
/// Location: `~/.local/share/sd/snapshots/snapshots.db`
pub fn default_db_path() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir().ok_or_else(|| {
        SdError::Other("Could not determine local data directory".to_string())
    })?;
    Ok(data_dir.join("sd").join("snapshots").join("snapshots.db"))
}

/// Returns the default image cache directory.
///
/// Location: `~/.local/share/sd/snapshots/images/`
pub fn default_image_cache_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir().ok_or_else(|| {
        SdError::Other("Could not determine local data directory".to_string())
    })?;
    Ok(data_dir.join("sd").join("snapshots").join("images"))
}

/// Returns the storage path for an image hash.
///
/// Uses first 2 characters as subdirectory for distribution.
pub fn image_cache_path(hash: &str) -> Result<PathBuf> {
    let cache_dir = default_image_cache_dir()?;
    let subdir = &hash[0..2.min(hash.len())];
    Ok(cache_dir.join(subdir).join(format!("{hash}.webp")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_database() {
        let db = SnapshotDb::in_memory().unwrap();
        assert!(db.list_snapshots().unwrap().is_empty());
    }

    #[test]
    fn test_save_and_load_snapshot() {
        let mut db = SnapshotDb::in_memory().unwrap();

        let mut snap = Snapshot::new(
            "test-layout".to_string(),
            "StreamDeckXL".to_string(),
            32,
            96,
            96,
        )
        .with_brightness(80);

        snap.add_key(SnapshotKey::image(0, Some(PathBuf::from("/tmp/icon.png")), "abc123".to_string()));
        snap.add_key(SnapshotKey::color(1, "#ff0000".to_string()));
        snap.add_key(SnapshotKey::cleared(2));

        let id = db.save_snapshot(&snap).unwrap();
        assert!(id > 0);

        let loaded = db.load_snapshot("test-layout").unwrap().unwrap();
        assert_eq!(loaded.name, "test-layout");
        assert_eq!(loaded.device_model, "StreamDeckXL");
        assert_eq!(loaded.brightness, Some(80));
        assert_eq!(loaded.keys.len(), 3);
    }

    #[test]
    fn test_update_existing_snapshot() {
        let mut db = SnapshotDb::in_memory().unwrap();

        let snap1 = Snapshot::new(
            "my-layout".to_string(),
            "StreamDeckMK2".to_string(),
            15,
            72,
            72,
        )
        .with_brightness(50);

        db.save_snapshot(&snap1).unwrap();

        let snap2 = Snapshot::new(
            "my-layout".to_string(),
            "StreamDeckMK2".to_string(),
            15,
            72,
            72,
        )
        .with_brightness(100);

        db.save_snapshot(&snap2).unwrap();

        let loaded = db.load_snapshot("my-layout").unwrap().unwrap();
        assert_eq!(loaded.brightness, Some(100));

        // Should still be just one snapshot
        let list = db.list_snapshots().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_list_snapshots() {
        let mut db = SnapshotDb::in_memory().unwrap();

        db.save_snapshot(&Snapshot::new("layout-1".to_string(), "XL".to_string(), 32, 96, 96)).unwrap();
        db.save_snapshot(&Snapshot::new("layout-2".to_string(), "MK2".to_string(), 15, 72, 72)).unwrap();

        let list = db.list_snapshots().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_delete_snapshot() {
        let mut db = SnapshotDb::in_memory().unwrap();

        db.save_snapshot(&Snapshot::new("to-delete".to_string(), "XL".to_string(), 32, 96, 96)).unwrap();
        assert!(db.snapshot_exists("to-delete").unwrap());

        let deleted = db.delete_snapshot("to-delete").unwrap();
        assert!(deleted);
        assert!(!db.snapshot_exists("to-delete").unwrap());

        // Second delete should return false
        let deleted = db.delete_snapshot("to-delete").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_image_cache() {
        let db = SnapshotDb::in_memory().unwrap();

        let image = CachedImage::new(
            "deadbeef1234".to_string(),
            Some(PathBuf::from("/tmp/icon.png")),
            72,
            72,
            "webp".to_string(),
            1024,
        );

        db.save_image(&image).unwrap();

        let loaded = db.load_image("deadbeef1234").unwrap().unwrap();
        assert_eq!(loaded.hash, "deadbeef1234");
        assert_eq!(loaded.width, 72);
        assert_eq!(loaded.format, "webp");
    }

    #[test]
    fn test_image_cache_path() {
        let path = image_cache_path("aabbccdd1234").unwrap();
        assert!(path.ends_with("aa/aabbccdd1234.webp"));
    }
}
