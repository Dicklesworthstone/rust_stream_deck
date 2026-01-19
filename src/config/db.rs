//! `SQLite` database operations for Stream Deck profile storage.
#![allow(dead_code)] // Database types are for future use

use std::path::Path;

use rusqlite::{Connection, Result as SqliteResult, params};

use crate::error::{Result, SdError};

/// Database wrapper for profile storage.
pub struct ProfileDb {
    conn: Connection,
}

impl ProfileDb {
    /// Opens or creates a database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref()).map_err(|e| {
            SdError::Other(format!("Failed to open database: {e}"))
        })?;

        let db = Self { conn };
        db.init_schema()?;
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
        self.conn.execute_batch(SCHEMA_SQL).map_err(|e| {
            SdError::Other(format!("Failed to initialize schema: {e}"))
        })?;
        Ok(())
    }

    /// Inserts a new profile package and returns its ID.
    pub fn insert_package(
        &self,
        name: &str,
        app_version: &str,
        device_model: &str,
        format_version: u32,
        os_type: Option<&str>,
        os_version: Option<&str>,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO profile_package (name, app_version, device_model, format_version, os_type, os_version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![name, app_version, device_model, format_version, os_type, os_version],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert package: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Inserts a required plugin for a package.
    pub fn insert_required_plugin(&self, package_id: i64, plugin_uuid: &str) -> Result<i64> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO required_plugin (package_id, plugin_uuid) VALUES (?1, ?2)",
                params![package_id, plugin_uuid],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert required plugin: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Inserts a device and returns its ID.
    pub fn insert_device(
        &self,
        package_id: i64,
        model: &str,
        uuid: &str,
        serial: Option<&str>,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO device (package_id, model, uuid, serial) VALUES (?1, ?2, ?3, ?4)",
                params![package_id, model, uuid, serial],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert device: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Inserts a profile and returns its ID.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_profile(
        &self,
        package_id: i64,
        uuid: &str,
        name: &str,
        version: &str,
        device_id: Option<i64>,
        current_page_uuid: Option<&str>,
        default_page_uuid: Option<&str>,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO profile (package_id, uuid, name, version, device_id, current_page_uuid, default_page_uuid)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![package_id, uuid, name, version, device_id, current_page_uuid, default_page_uuid],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert profile: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Inserts a page and returns its ID.
    pub fn insert_page(
        &self,
        profile_id: i64,
        uuid: &str,
        name: Option<&str>,
        is_default: bool,
        sort_order: i32,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO page (profile_id, uuid, name, is_default, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![profile_id, uuid, name, is_default, sort_order],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert page: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Inserts an action and returns its ID.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_action(
        &self,
        page_id: i64,
        action_uuid: &str,
        row: i32,
        col: i32,
        name: &str,
        plugin_uuid: &str,
        plugin_name: Option<&str>,
        plugin_version: Option<&str>,
        linked_title: bool,
        current_state: i32,
        settings_json: Option<&str>,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO action (page_id, action_uuid, row, col, name, plugin_uuid, plugin_name, plugin_version, linked_title, current_state, settings_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![page_id, action_uuid, row, col, name, plugin_uuid, plugin_name, plugin_version, linked_title, current_state, settings_json],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert action: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Inserts an action state and returns its ID.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_action_state(
        &self,
        action_id: i64,
        state_index: i32,
        title: Option<&str>,
        title_alignment: Option<&str>,
        title_color: Option<&str>,
        show_title: bool,
        font_family: Option<&str>,
        font_size: Option<i32>,
        font_style: Option<&str>,
        font_underline: bool,
        outline_thickness: Option<i32>,
        image_id: Option<i64>,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO action_state (action_id, state_index, title, title_alignment, title_color, show_title, font_family, font_size, font_style, font_underline, outline_thickness, image_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![action_id, state_index, title, title_alignment, title_color, show_title, font_family, font_size, font_style, font_underline, outline_thickness, image_id],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert action state: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Inserts an image and returns its ID.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_image(
        &self,
        package_id: i64,
        original_filename: &str,
        content_hash: &str,
        format: &str,
        width: Option<u32>,
        height: Option<u32>,
        data_base64: &str,
    ) -> Result<i64> {
        // Use INSERT OR IGNORE to handle duplicate hashes
        self.conn
            .execute(
                "INSERT OR IGNORE INTO image (package_id, original_filename, content_hash, format, width, height, data_base64)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![package_id, original_filename, content_hash, format, width, height, data_base64],
            )
            .map_err(|e| SdError::Other(format!("Failed to insert image: {e}")))?;

        // If row was ignored due to duplicate, find the existing ID
        let id: i64 = self.conn
            .query_row(
                "SELECT id FROM image WHERE package_id = ?1 AND content_hash = ?2",
                params![package_id, content_hash],
                |row| row.get(0),
            )
            .map_err(|e| SdError::Other(format!("Failed to get image ID: {e}")))?;

        Ok(id)
    }

    /// Looks up an image by its content hash within a package.
    pub fn find_image_by_hash(&self, package_id: i64, content_hash: &str) -> Result<Option<i64>> {
        let result: SqliteResult<i64> = self.conn.query_row(
            "SELECT id FROM image WHERE package_id = ?1 AND content_hash = ?2",
            params![package_id, content_hash],
            |row| row.get(0),
        );

        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SdError::Other(format!("Failed to find image: {e}"))),
        }
    }

    /// Lists all profile packages.
    pub fn list_packages(&self) -> Result<Vec<PackageRow>> {
        let mut stmt = self.conn
            .prepare("SELECT id, name, app_version, device_model, format_version, created_at FROM profile_package ORDER BY created_at DESC")
            .map_err(|e| SdError::Other(format!("Failed to prepare statement: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(PackageRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    app_version: row.get(2)?,
                    device_model: row.get(3)?,
                    format_version: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| SdError::Other(format!("Failed to query packages: {e}")))?;

        let mut packages = Vec::new();
        for row in rows {
            packages.push(row.map_err(|e| SdError::Other(format!("Failed to read row: {e}")))?);
        }

        Ok(packages)
    }

    /// Deletes a package and all its related data (cascades).
    pub fn delete_package(&self, package_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM profile_package WHERE id = ?1", params![package_id])
            .map_err(|e| SdError::Other(format!("Failed to delete package: {e}")))?;
        Ok(())
    }
}

/// Row data for a profile package.
#[derive(Debug, Clone)]
pub struct PackageRow {
    pub id: i64,
    pub name: String,
    pub app_version: String,
    pub device_model: String,
    pub format_version: u32,
    pub created_at: String,
}

/// SQL schema for the database.
const SCHEMA_SQL: &str = r"
-- Core profile package (top-level archive info)
CREATE TABLE IF NOT EXISTS profile_package (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    app_version TEXT NOT NULL,
    device_model TEXT NOT NULL,
    format_version INTEGER NOT NULL DEFAULT 1,
    os_type TEXT,
    os_version TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Required plugins for the profile
CREATE TABLE IF NOT EXISTS required_plugin (
    id INTEGER PRIMARY KEY,
    package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
    plugin_uuid TEXT NOT NULL,
    UNIQUE(package_id, plugin_uuid)
);

-- Device info
CREATE TABLE IF NOT EXISTS device (
    id INTEGER PRIMARY KEY,
    package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    uuid TEXT NOT NULL,
    serial TEXT,
    UNIQUE(package_id, uuid)
);

-- Profiles (can be nested within packages)
CREATE TABLE IF NOT EXISTS profile (
    id INTEGER PRIMARY KEY,
    package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
    uuid TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '3.0',
    device_id INTEGER REFERENCES device(id) ON DELETE SET NULL,
    current_page_uuid TEXT,
    default_page_uuid TEXT,
    UNIQUE(package_id, uuid)
);

-- Pages within profiles (each page is a screen of buttons)
CREATE TABLE IF NOT EXISTS page (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES profile(id) ON DELETE CASCADE,
    uuid TEXT NOT NULL,
    name TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    UNIQUE(profile_id, uuid)
);

-- Actions assigned to keys on a page
CREATE TABLE IF NOT EXISTS action (
    id INTEGER PRIMARY KEY,
    page_id INTEGER NOT NULL REFERENCES page(id) ON DELETE CASCADE,
    action_uuid TEXT NOT NULL,
    row INTEGER NOT NULL,
    col INTEGER NOT NULL,
    name TEXT NOT NULL,
    plugin_uuid TEXT NOT NULL,
    plugin_name TEXT,
    plugin_version TEXT,
    linked_title BOOLEAN NOT NULL DEFAULT TRUE,
    current_state INTEGER NOT NULL DEFAULT 0,
    settings_json TEXT,
    UNIQUE(page_id, row, col)
);

-- Action states (multi-state actions like toggles)
CREATE TABLE IF NOT EXISTS action_state (
    id INTEGER PRIMARY KEY,
    action_id INTEGER NOT NULL REFERENCES action(id) ON DELETE CASCADE,
    state_index INTEGER NOT NULL DEFAULT 0,
    title TEXT,
    title_alignment TEXT DEFAULT 'top',
    title_color TEXT DEFAULT '#ffffff',
    show_title BOOLEAN NOT NULL DEFAULT TRUE,
    font_family TEXT,
    font_size INTEGER DEFAULT 9,
    font_style TEXT,
    font_underline BOOLEAN NOT NULL DEFAULT FALSE,
    outline_thickness INTEGER DEFAULT 2,
    image_id INTEGER REFERENCES image(id) ON DELETE SET NULL,
    UNIQUE(action_id, state_index)
);

-- Images stored as base64-encoded data
CREATE TABLE IF NOT EXISTS image (
    id INTEGER PRIMARY KEY,
    package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
    original_filename TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    format TEXT NOT NULL DEFAULT 'png',
    width INTEGER,
    height INTEGER,
    data_base64 TEXT NOT NULL,
    UNIQUE(package_id, content_hash)
);

-- Multi-actions (sequences of actions)
CREATE TABLE IF NOT EXISTS multi_action (
    id INTEGER PRIMARY KEY,
    parent_action_id INTEGER NOT NULL REFERENCES action(id) ON DELETE CASCADE,
    child_action_id INTEGER NOT NULL REFERENCES action(id) ON DELETE CASCADE,
    execution_order INTEGER NOT NULL DEFAULT 0,
    delay_ms INTEGER DEFAULT 0,
    UNIQUE(parent_action_id, execution_order)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_profile_package_id ON profile(package_id);
CREATE INDEX IF NOT EXISTS idx_page_profile_id ON page(profile_id);
CREATE INDEX IF NOT EXISTS idx_action_page_id ON action(page_id);
CREATE INDEX IF NOT EXISTS idx_action_position ON action(page_id, row, col);
CREATE INDEX IF NOT EXISTS idx_image_hash ON image(content_hash);

-- Enable foreign key enforcement
PRAGMA foreign_keys = ON;
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_database() {
        let db = ProfileDb::in_memory().unwrap();
        let packages = db.list_packages().unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_insert_package() {
        let db = ProfileDb::in_memory().unwrap();

        let id = db.insert_package(
            "Test Profile",
            "7.1.1.22340",
            "20GAT9901",
            1,
            Some("macOS"),
            Some("26.2.0"),
        ).unwrap();

        assert_eq!(id, 1);

        let packages = db.list_packages().unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "Test Profile");
    }

    #[test]
    fn test_insert_image_dedup() {
        let db = ProfileDb::in_memory().unwrap();

        let pkg_id = db.insert_package("Test", "1.0", "DEV", 1, None, None).unwrap();

        let id1 = db.insert_image(pkg_id, "icon1.png", "abc123", "png", Some(72), Some(72), "base64data").unwrap();
        let id2 = db.insert_image(pkg_id, "icon2.png", "abc123", "png", Some(72), Some(72), "base64data").unwrap();

        // Same hash should return same ID
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_cascade_delete() {
        let db = ProfileDb::in_memory().unwrap();

        let pkg_id = db.insert_package("Test", "1.0", "DEV", 1, None, None).unwrap();
        let _device_id = db.insert_device(pkg_id, "DEV", "uuid-123", None).unwrap();
        let profile_id = db.insert_profile(pkg_id, "prof-uuid", "Profile", "3.0", None, None, None).unwrap();
        let page_id = db.insert_page(profile_id, "page-uuid", None, true, 0).unwrap();
        let _action_id = db.insert_action(page_id, "action-uuid", 0, 0, "Action", "plugin-uuid", None, None, true, 0, None).unwrap();

        // Delete package should cascade
        db.delete_package(pkg_id).unwrap();

        let packages = db.list_packages().unwrap();
        assert!(packages.is_empty());
    }
}
