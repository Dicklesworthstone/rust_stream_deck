//! Snapshot data types for device state persistence.
//!
//! These types represent the structure of saved device snapshots,
//! including metadata, key states, and cached image information.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A saved device state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Database ID (set after loading from DB).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Unique name for this snapshot.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Device model this snapshot was taken from (e.g., "StreamDeckXL").
    pub device_model: String,
    /// Optional device serial to bind to specific device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_serial: Option<String>,
    /// Number of keys on the device when snapshot was taken.
    pub key_count: u8,
    /// Width of key images in pixels.
    pub key_width: u32,
    /// Height of key images in pixels.
    pub key_height: u32,
    /// Brightness level (0-100) if captured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    /// When the snapshot was created.
    pub created_at: DateTime<Utc>,
    /// When the snapshot was last updated.
    pub updated_at: DateTime<Utc>,
    /// Per-key state data.
    #[serde(default)]
    pub keys: Vec<SnapshotKey>,
}

impl Snapshot {
    /// Create a new snapshot with current timestamp.
    #[must_use]
    pub fn new(name: String, device_model: String, key_count: u8, key_width: u32, key_height: u32) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            name,
            description: None,
            device_model,
            device_serial: None,
            key_count,
            key_width,
            key_height,
            brightness: None,
            created_at: now,
            updated_at: now,
            keys: Vec::new(),
        }
    }

    /// Set the brightness level.
    pub fn with_brightness(mut self, brightness: u8) -> Self {
        self.brightness = Some(brightness);
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a key to the snapshot.
    pub fn add_key(&mut self, key: SnapshotKey) {
        self.keys.push(key);
    }
}

/// State of a single key in a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotKey {
    /// Key index (0-based).
    pub key_index: u8,
    /// State of the key.
    pub state: KeyState,
}

impl SnapshotKey {
    /// Create a new snapshot key with an image.
    #[must_use]
    pub fn image(key_index: u8, source_path: Option<PathBuf>, image_hash: String) -> Self {
        Self {
            key_index,
            state: KeyState::Image {
                source_path,
                image_hash,
            },
        }
    }

    /// Create a new snapshot key with a color.
    #[must_use]
    pub fn color(key_index: u8, hex: String) -> Self {
        Self {
            key_index,
            state: KeyState::Color { hex },
        }
    }

    /// Create a new snapshot key that is cleared.
    #[must_use]
    pub fn cleared(key_index: u8) -> Self {
        Self {
            key_index,
            state: KeyState::Clear,
        }
    }
}

/// State type for a key in a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyState {
    /// Key has an image.
    Image {
        /// Original source path (for reference/portability).
        #[serde(skip_serializing_if = "Option::is_none")]
        source_path: Option<PathBuf>,
        /// SHA256 hash of the processed image content (content-addressable).
        image_hash: String,
    },
    /// Key filled with solid color.
    Color {
        /// Hex color string (e.g., "#ff0000").
        hex: String,
    },
    /// Key is cleared (black).
    Clear,
}

/// Metadata for a cached image in content-addressable storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedImage {
    /// SHA256 hash (primary key, content-addressable).
    pub hash: String,
    /// First source path seen for this image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<PathBuf>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Image format (e.g., "webp", "png").
    pub format: String,
    /// Size of the cached file in bytes.
    pub size_bytes: u64,
    /// When the image was first cached.
    pub created_at: DateTime<Utc>,
    /// When the image was last accessed.
    pub last_accessed_at: DateTime<Utc>,
    /// Number of times this image has been accessed.
    pub access_count: u32,
}

impl CachedImage {
    /// Create a new cached image entry.
    #[must_use]
    pub fn new(
        hash: String,
        original_path: Option<PathBuf>,
        width: u32,
        height: u32,
        format: String,
        size_bytes: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            hash,
            original_path,
            width,
            height,
            format,
            size_bytes,
            created_at: now,
            last_accessed_at: now,
            access_count: 1,
        }
    }
}

/// Summary information about a snapshot (for listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    /// Database ID.
    pub id: i64,
    /// Snapshot name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Device model.
    pub device_model: String,
    /// Number of keys with saved state.
    pub key_count: u8,
    /// Brightness level if saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_new() {
        let snap = Snapshot::new(
            "test".to_string(),
            "StreamDeckXL".to_string(),
            32,
            96,
            96,
        );
        assert_eq!(snap.name, "test");
        assert_eq!(snap.device_model, "StreamDeckXL");
        assert_eq!(snap.key_count, 32);
        assert!(snap.keys.is_empty());
        assert!(snap.id.is_none());
    }

    #[test]
    fn test_snapshot_with_brightness() {
        let snap = Snapshot::new(
            "test".to_string(),
            "StreamDeckMK2".to_string(),
            15,
            72,
            72,
        )
        .with_brightness(80);
        assert_eq!(snap.brightness, Some(80));
    }

    #[test]
    fn test_snapshot_key_types() {
        let image_key = SnapshotKey::image(0, Some(PathBuf::from("/tmp/icon.png")), "abc123".to_string());
        assert!(matches!(image_key.state, KeyState::Image { .. }));

        let color_key = SnapshotKey::color(1, "#ff0000".to_string());
        assert!(matches!(color_key.state, KeyState::Color { .. }));

        let clear_key = SnapshotKey::cleared(2);
        assert!(matches!(clear_key.state, KeyState::Clear));
    }

    #[test]
    fn test_cached_image_new() {
        let img = CachedImage::new(
            "deadbeef".to_string(),
            Some(PathBuf::from("/tmp/test.png")),
            72,
            72,
            "webp".to_string(),
            1024,
        );
        assert_eq!(img.hash, "deadbeef");
        assert_eq!(img.width, 72);
        assert_eq!(img.access_count, 1);
    }

    #[test]
    fn test_key_state_serialization() {
        let state = KeyState::Image {
            source_path: Some(PathBuf::from("/tmp/icon.png")),
            image_hash: "abc123".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"type\":\"image\""));

        let state = KeyState::Color { hex: "#ff0000".to_string() };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"type\":\"color\""));

        let state = KeyState::Clear;
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"type\":\"clear\""));
    }
}
