//! Device information types for Stream Deck devices.

use serde::Serialize;

/// Information about a connected Stream Deck device.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    /// Device serial number
    pub serial: String,
    /// Human-readable product name
    pub product_name: String,
    /// Firmware version string
    pub firmware_version: String,
    /// Number of keys on the device
    pub key_count: u8,
    /// Width of key images in pixels
    pub key_width: usize,
    /// Height of key images in pixels
    pub key_height: usize,
    /// Number of key rows
    pub rows: u8,
    /// Number of key columns
    pub cols: u8,
    /// Device kind/model identifier
    pub kind: String,
}

/// Supported Stream Deck device models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
pub enum DeviceModel {
    /// Stream Deck Mini (6 keys, 3x2)
    Mini,
    /// Stream Deck Mini MK.2 (6 keys, 3x2)
    MiniMk2,
    /// Stream Deck Original (15 keys, 5x3)
    Original,
    /// Stream Deck Original V2 (15 keys, 5x3)
    OriginalV2,
    /// Stream Deck MK.2 (15 keys, 5x3)
    Mk2,
    /// Stream Deck XL (32 keys, 8x4)
    Xl,
    /// Stream Deck XL V2 (32 keys, 8x4)
    XlV2,
    /// Stream Deck Pedal (3 pedals, no display)
    Pedal,
    /// Stream Deck + (8 keys + LCD + dials)
    Plus,
    /// Stream Deck Neo (8 keys + touch strip)
    Neo,
}

#[allow(dead_code)]
impl DeviceModel {
    /// Returns the number of keys for this device model.
    #[must_use]
    pub const fn key_count(self) -> u8 {
        match self {
            Self::Mini | Self::MiniMk2 => 6,
            Self::Original | Self::OriginalV2 | Self::Mk2 => 15,
            Self::Xl | Self::XlV2 => 32,
            Self::Pedal => 3,
            Self::Plus | Self::Neo => 8,
        }
    }

    /// Returns the key image dimensions (width, height) in pixels.
    #[must_use]
    pub const fn key_dimensions(self) -> (u32, u32) {
        match self {
            Self::Mini | Self::MiniMk2 | Self::Original | Self::OriginalV2 | Self::Mk2 => (72, 72),
            Self::Xl | Self::XlV2 => (96, 96),
            Self::Pedal => (0, 0), // No display
            Self::Plus => (120, 120),
            Self::Neo => (72, 72),
        }
    }

    /// Returns the key layout (columns, rows).
    #[must_use]
    pub const fn layout(self) -> (u8, u8) {
        match self {
            Self::Mini | Self::MiniMk2 => (3, 2),
            Self::Original | Self::OriginalV2 | Self::Mk2 => (5, 3),
            Self::Xl | Self::XlV2 => (8, 4),
            Self::Pedal => (3, 1),
            Self::Plus | Self::Neo => (4, 2),
        }
    }

    /// Returns a human-readable name for this device model.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Mini => "Stream Deck Mini",
            Self::MiniMk2 => "Stream Deck Mini MK.2",
            Self::Original => "Stream Deck (Original)",
            Self::OriginalV2 => "Stream Deck (Original V2)",
            Self::Mk2 => "Stream Deck MK.2",
            Self::Xl => "Stream Deck XL",
            Self::XlV2 => "Stream Deck XL V2",
            Self::Pedal => "Stream Deck Pedal",
            Self::Plus => "Stream Deck +",
            Self::Neo => "Stream Deck Neo",
        }
    }
}

/// Button press/release event.
#[derive(Debug, Clone, Serialize)]
pub struct ButtonEvent {
    /// Key index (0-based)
    pub key: u8,
    /// True if pressed, false if released
    pub pressed: bool,
    /// Timestamp in milliseconds since watch started
    pub timestamp_ms: u64,
}

/// Connection retry options for opening devices.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConnectionOptions {
    /// Maximum number of connection attempts (default: 3).
    pub max_retries: u32,
    /// Initial delay between retries (default: 1000ms).
    pub retry_delay: std::time::Duration,
    /// Exponential backoff factor (default: 1.5).
    pub backoff_factor: f32,
    /// Maximum delay cap (default: 10000ms).
    pub max_delay: std::time::Duration,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        use std::time::Duration;
        Self {
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
            backoff_factor: 1.5,
            max_delay: Duration::from_millis(10000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_model_key_count() {
        assert_eq!(DeviceModel::Mini.key_count(), 6);
        assert_eq!(DeviceModel::Mk2.key_count(), 15);
        assert_eq!(DeviceModel::Xl.key_count(), 32);
    }

    #[test]
    fn test_device_model_dimensions() {
        assert_eq!(DeviceModel::Mini.key_dimensions(), (72, 72));
        assert_eq!(DeviceModel::Xl.key_dimensions(), (96, 96));
        assert_eq!(DeviceModel::Plus.key_dimensions(), (120, 120));
    }

    #[test]
    fn test_device_model_layout() {
        assert_eq!(DeviceModel::Mini.layout(), (3, 2));
        assert_eq!(DeviceModel::Mk2.layout(), (5, 3));
        assert_eq!(DeviceModel::Xl.layout(), (8, 4));
    }
}
