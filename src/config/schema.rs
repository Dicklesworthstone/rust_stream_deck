//! Data types representing Stream Deck profile configuration.
#![allow(dead_code)] // Schema types are for future use
//!
//! These types map to both:
//! - The official Elgato `.streamDeckProfile` ZIP format (JSON manifests + PNG images)
//! - Our internal `SQLite` storage format with base64-encoded images
//!
//! # `SQLite` Schema
//!
//! ```sql
//! -- Core profile package (top-level archive info)
//! CREATE TABLE profile_package (
//!     id INTEGER PRIMARY KEY,
//!     name TEXT NOT NULL,
//!     app_version TEXT NOT NULL,
//!     device_model TEXT NOT NULL,
//!     format_version INTEGER NOT NULL DEFAULT 1,
//!     os_type TEXT,
//!     os_version TEXT,
//!     created_at TEXT NOT NULL DEFAULT (datetime('now')),
//!     updated_at TEXT NOT NULL DEFAULT (datetime('now'))
//! );
//!
//! -- Required plugins for the profile
//! CREATE TABLE required_plugin (
//!     id INTEGER PRIMARY KEY,
//!     package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
//!     plugin_uuid TEXT NOT NULL,
//!     UNIQUE(package_id, plugin_uuid)
//! );
//!
//! -- Device info
//! CREATE TABLE device (
//!     id INTEGER PRIMARY KEY,
//!     package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
//!     model TEXT NOT NULL,
//!     uuid TEXT NOT NULL,
//!     serial TEXT,
//!     UNIQUE(package_id, uuid)
//! );
//!
//! -- Profiles (can be nested within packages)
//! CREATE TABLE profile (
//!     id INTEGER PRIMARY KEY,
//!     package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
//!     uuid TEXT NOT NULL,
//!     name TEXT NOT NULL,
//!     version TEXT NOT NULL DEFAULT '3.0',
//!     device_id INTEGER REFERENCES device(id) ON DELETE SET NULL,
//!     current_page_uuid TEXT,
//!     default_page_uuid TEXT,
//!     UNIQUE(package_id, uuid)
//! );
//!
//! -- Pages within profiles (each page is a screen of buttons)
//! CREATE TABLE page (
//!     id INTEGER PRIMARY KEY,
//!     profile_id INTEGER NOT NULL REFERENCES profile(id) ON DELETE CASCADE,
//!     uuid TEXT NOT NULL,
//!     name TEXT,
//!     is_default BOOLEAN NOT NULL DEFAULT FALSE,
//!     sort_order INTEGER NOT NULL DEFAULT 0,
//!     UNIQUE(profile_id, uuid)
//! );
//!
//! -- Actions assigned to keys on a page
//! CREATE TABLE action (
//!     id INTEGER PRIMARY KEY,
//!     page_id INTEGER NOT NULL REFERENCES page(id) ON DELETE CASCADE,
//!     action_uuid TEXT NOT NULL,
//!     row INTEGER NOT NULL,
//!     col INTEGER NOT NULL,
//!     name TEXT NOT NULL,
//!     plugin_uuid TEXT NOT NULL,
//!     plugin_name TEXT,
//!     plugin_version TEXT,
//!     linked_title BOOLEAN NOT NULL DEFAULT TRUE,
//!     current_state INTEGER NOT NULL DEFAULT 0,
//!     settings_json TEXT, -- JSON blob for plugin-specific settings
//!     UNIQUE(page_id, row, col)
//! );
//!
//! -- Action states (multi-state actions like toggles)
//! CREATE TABLE action_state (
//!     id INTEGER PRIMARY KEY,
//!     action_id INTEGER NOT NULL REFERENCES action(id) ON DELETE CASCADE,
//!     state_index INTEGER NOT NULL DEFAULT 0,
//!     title TEXT,
//!     title_alignment TEXT DEFAULT 'top',
//!     title_color TEXT DEFAULT '#ffffff',
//!     show_title BOOLEAN NOT NULL DEFAULT TRUE,
//!     font_family TEXT,
//!     font_size INTEGER DEFAULT 9,
//!     font_style TEXT,
//!     font_underline BOOLEAN NOT NULL DEFAULT FALSE,
//!     outline_thickness INTEGER DEFAULT 2,
//!     image_id INTEGER REFERENCES image(id) ON DELETE SET NULL,
//!     UNIQUE(action_id, state_index)
//! );
//!
//! -- Images stored as base64-encoded data
//! CREATE TABLE image (
//!     id INTEGER PRIMARY KEY,
//!     package_id INTEGER NOT NULL REFERENCES profile_package(id) ON DELETE CASCADE,
//!     original_filename TEXT NOT NULL,
//!     content_hash TEXT NOT NULL, -- SHA256 of raw image data
//!     format TEXT NOT NULL DEFAULT 'png',
//!     width INTEGER,
//!     height INTEGER,
//!     data_base64 TEXT NOT NULL, -- Base64-encoded image data
//!     UNIQUE(package_id, content_hash)
//! );
//!
//! -- Multi-actions (sequences of actions)
//! CREATE TABLE multi_action (
//!     id INTEGER PRIMARY KEY,
//!     parent_action_id INTEGER NOT NULL REFERENCES action(id) ON DELETE CASCADE,
//!     child_action_id INTEGER NOT NULL REFERENCES action(id) ON DELETE CASCADE,
//!     execution_order INTEGER NOT NULL DEFAULT 0,
//!     delay_ms INTEGER DEFAULT 0,
//!     UNIQUE(parent_action_id, execution_order)
//! );
//!
//! -- Indexes for common queries
//! CREATE INDEX idx_profile_package_id ON profile(package_id);
//! CREATE INDEX idx_page_profile_id ON page(profile_id);
//! CREATE INDEX idx_action_page_id ON action(page_id);
//! CREATE INDEX idx_action_position ON action(page_id, row, col);
//! CREATE INDEX idx_image_hash ON image(content_hash);
//! ```

use serde::{Deserialize, Serialize};

/// Top-level package metadata from the ZIP archive's `package.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProfilePackage {
    /// Application version that created this profile (e.g., "7.1.1.22340")
    pub app_version: String,
    /// Device model identifier (e.g., "20GAT9901")
    pub device_model: String,
    /// Device settings (usually null)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_settings: Option<serde_json::Value>,
    /// Format version (usually 1)
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    /// OS type (e.g., "macOS", "Windows")
    #[serde(rename = "OSType", skip_serializing_if = "Option::is_none")]
    pub os_type: Option<String>,
    /// OS version
    #[serde(rename = "OSVersion", skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    /// List of required plugin UUIDs
    #[serde(default)]
    pub required_plugins: Vec<String>,
}

const fn default_format_version() -> u32 {
    1
}

/// Device information from profile manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Device {
    /// Device model identifier
    pub model: String,
    /// Device UUID
    #[serde(rename = "UUID")]
    pub uuid: String,
}

/// Profile metadata from `.sdProfile/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Profile {
    /// Device info
    pub device: Device,
    /// Profile name
    pub name: String,
    /// Page navigation info
    pub pages: ProfilePages,
    /// Format version (e.g., "3.0")
    #[serde(default = "default_profile_version")]
    pub version: String,
}

fn default_profile_version() -> String {
    "3.0".to_string()
}

/// Page navigation info within a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProfilePages {
    /// Currently active page UUID
    pub current: String,
    /// Default page UUID
    pub default: String,
    /// List of page UUIDs
    #[serde(default)]
    pub pages: Vec<String>,
}

/// Page configuration (controller manifest).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Page {
    /// Controller configurations
    #[serde(default)]
    pub controllers: Vec<Controller>,
}

/// Controller configuration (typically one per page).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Controller {
    /// Actions mapped by "row,col" position
    #[serde(default)]
    pub actions: std::collections::HashMap<String, Action>,
}

/// Action configuration for a single key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(clippy::struct_field_names)] // Field names match the JSON schema
pub struct Action {
    /// Unique action ID
    #[serde(rename = "ActionID")]
    pub action_id: String,
    /// Whether title is linked to plugin
    #[serde(default = "default_true")]
    pub linked_title: bool,
    /// Action name
    pub name: String,
    /// Plugin info
    pub plugin: Plugin,
    /// Resources (usually null)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<serde_json::Value>,
    /// Plugin-specific settings
    #[serde(default)]
    pub settings: ActionSettings,
    /// Current state index
    #[serde(default)]
    pub state: u32,
    /// State configurations
    #[serde(default)]
    pub states: Vec<ActionState>,
    /// Plugin UUID
    #[serde(rename = "UUID")]
    pub uuid: String,
    /// Nested actions (for multi-actions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<MultiActionEntry>>,
}

const fn default_true() -> bool {
    true
}

/// Multi-action entry (for grouped action sequences).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MultiActionEntry {
    /// Nested actions in this group
    #[serde(default)]
    pub actions: Vec<Action>,
}

/// Plugin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Plugin {
    /// Plugin display name
    pub name: String,
    /// Plugin UUID (e.g., "com.elgato.streamdeck.system.hotkey")
    #[serde(rename = "UUID")]
    pub uuid: String,
    /// Plugin version
    #[serde(default = "default_plugin_version")]
    pub version: String,
}

fn default_plugin_version() -> String {
    "1.0".to_string()
}

/// Plugin-specific settings (stored as flexible JSON).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionSettings {
    /// Raw JSON settings
    #[serde(flatten)]
    pub inner: serde_json::Value,
}

/// Action state configuration (for multi-state buttons).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionState {
    /// Font family
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    /// Font size in points
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u32>,
    /// Font style (e.g., "Regular", "Bold")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<String>,
    /// Whether font is underlined
    #[serde(default)]
    pub font_underline: bool,
    /// Image filename (relative to Images/ directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Outline thickness
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_thickness: Option<u32>,
    /// Whether to show title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_title: Option<bool>,
    /// Button title text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Title alignment ("top", "middle", "bottom")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_alignment: Option<String>,
    /// Title color in hex (e.g., "#ffffff")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_color: Option<String>,
}

/// Image data for storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// Original filename from the archive
    pub filename: String,
    /// SHA256 hash of the raw image data
    pub content_hash: String,
    /// Image format (png, jpeg, etc.)
    pub format: String,
    /// Image width in pixels
    pub width: Option<u32>,
    /// Image height in pixels
    pub height: Option<u32>,
    /// Base64-encoded image data
    pub data_base64: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_json() {
        let json = r#"{
            "AppVersion": "7.1.1.22340",
            "DeviceModel": "20GAT9901",
            "DeviceSettings": null,
            "FormatVersion": 1,
            "OSType": "macOS",
            "OSVersion": "26.2.0",
            "RequiredPlugins": ["com.elgato.streamdeck.system.hotkey"]
        }"#;

        let package: ProfilePackage = serde_json::from_str(json).unwrap();
        assert_eq!(package.app_version, "7.1.1.22340");
        assert_eq!(package.device_model, "20GAT9901");
        assert_eq!(package.format_version, 1);
        assert_eq!(package.os_type, Some("macOS".to_string()));
        assert_eq!(package.required_plugins.len(), 1);
    }

    #[test]
    fn test_parse_profile_manifest() {
        let json = r#"{
            "Device": {
                "Model": "20GAT9901",
                "UUID": "47462106-ecbc-42f6-ad87-46b9a8682b1b"
            },
            "Name": "Default Profile",
            "Pages": {
                "Current": "00000000-0000-0000-0000-000000000000",
                "Default": "45c31142-6662-45a2-86b3-b965a30669ae",
                "Pages": ["5eedeb1b-cc0e-42ac-8dbb-9c0cd3f38b15"]
            },
            "Version": "3.0"
        }"#;

        let profile: Profile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.name, "Default Profile");
        assert_eq!(profile.device.model, "20GAT9901");
        assert_eq!(profile.pages.pages.len(), 1);
    }
}
