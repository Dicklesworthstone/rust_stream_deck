//! Error types for Stream Deck CLI operations.

use thiserror::Error;

/// Primary error type for Stream Deck operations.
#[derive(Error, Debug)]
pub enum SdError {
    // Device errors
    #[error("No Stream Deck devices found")]
    NoDevicesFound,

    #[error("Device not found: {serial}")]
    DeviceNotFound { serial: String },

    #[error("Multiple devices found, specify --serial: {serials:?}")]
    MultipleDevices { serials: Vec<String> },

    #[error("Failed to open device '{serial}': {reason}")]
    DeviceOpenFailed { serial: String, reason: String },

    #[error("Device communication error: {0}")]
    DeviceCommunication(String),

    // Image errors
    #[error("Invalid image dimensions: expected {expected_w}x{expected_h}, got {actual_w}x{actual_h}")]
    InvalidImageDimensions {
        expected_w: u32,
        expected_h: u32,
        actual_w: u32,
        actual_h: u32,
    },

    #[error("Image processing failed: {0}")]
    ImageProcessing(String),

    #[error("Image file not found: {path}")]
    ImageNotFound { path: String },

    // Key errors
    #[error("Invalid key index {index}: device has {max} keys (0-{max_idx})")]
    InvalidKeyIndex { index: u8, max: u8, max_idx: u8 },

    // Configuration errors
    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: String },

    #[error("Configuration parse error: {0}")]
    ConfigParse(String),

    #[error("Invalid brightness value {value}: must be 0-100")]
    InvalidBrightness { value: u8 },

    // Web server errors
    #[error("Web server failed to start on {addr}: {reason}")]
    WebServerFailed { addr: String, reason: String },

    // General errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl SdError {
    /// Returns true if the error is recoverable by the user.
    pub const fn is_user_recoverable(&self) -> bool {
        matches!(
            self,
            Self::NoDevicesFound
                | Self::DeviceNotFound { .. }
                | Self::MultipleDevices { .. }
                | Self::InvalidKeyIndex { .. }
                | Self::InvalidBrightness { .. }
                | Self::ImageNotFound { .. }
                | Self::ConfigNotFound { .. }
        )
    }

    /// Returns a suggestion for how to fix the error.
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            Self::NoDevicesFound => Some("Ensure Stream Deck is connected via USB"),
            Self::MultipleDevices { .. } => Some("Use --serial to specify which device"),
            Self::InvalidBrightness { .. } => Some("Use a value between 0 and 100"),
            Self::ConfigNotFound { .. } => Some("Run: sd init"),
            _ => None,
        }
    }
}

/// Convenience type alias for Results using SdError.
pub type Result<T> = std::result::Result<T, SdError>;

/// Extension trait for adding context to errors.
pub trait ResultExt<T> {
    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

impl<T, E: std::error::Error> ResultExt<T> for std::result::Result<T, E> {
    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.map_err(|e| SdError::Other(format!("{}: {e}", f().into())))
    }
}
