//! Error types for Stream Deck CLI operations.

use thiserror::Error;

/// Primary error type for Stream Deck operations.
#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants reserved for future use
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
    #[error(
        "Invalid image dimensions: expected {expected_w}x{expected_h}, got {actual_w}x{actual_h}"
    )]
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

    #[error("Unsupported image format: {0}")]
    ImageFormat(String),

    // Key errors
    #[error("Invalid key index {index}: device has {max} keys (0-{max_idx})")]
    InvalidKeyIndex { index: u8, max: u8, max_idx: u8 },

    // Configuration errors
    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: String },

    #[error("Configuration parse error: {0}")]
    ConfigParse(String),

    #[error("Invalid configuration: {0}")]
    ConfigInvalid(String),

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
    /// Returns true if the error is likely related to device connection or transport.
    #[allow(dead_code)]
    pub const fn is_connection_error(&self) -> bool {
        matches!(
            self,
            Self::NoDevicesFound
                | Self::DeviceNotFound { .. }
                | Self::DeviceOpenFailed { .. }
                | Self::DeviceCommunication(_)
        )
    }

    /// Returns true if the error is related to image handling.
    #[allow(dead_code)]
    pub const fn is_image_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidImageDimensions { .. }
                | Self::ImageProcessing(_)
                | Self::ImageNotFound { .. }
                | Self::ImageFormat(_)
        )
    }

    /// Returns true if the error is related to config parsing/availability.
    #[allow(dead_code)]
    pub const fn is_config_error(&self) -> bool {
        matches!(
            self,
            Self::ConfigNotFound { .. } | Self::ConfigParse(_) | Self::ConfigInvalid(_)
        )
    }

    /// Returns true if retrying might resolve the error.
    #[allow(dead_code)]
    pub const fn is_retryable(&self) -> bool {
        self.is_connection_error()
    }

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
                | Self::ImageFormat(_)
                | Self::ConfigNotFound { .. }
                | Self::ConfigInvalid(_)
        )
    }

    /// Returns a suggestion for how to fix the error.
    pub const fn suggestion(&self) -> Option<&'static str> {
        match self {
            Self::NoDevicesFound => Some("Ensure Stream Deck is connected via USB"),
            Self::MultipleDevices { .. } => Some("Use --serial to specify which device"),
            Self::InvalidBrightness { .. } => Some("Use a value between 0 and 100"),
            Self::ConfigNotFound { .. } => Some("Run: sd init"),
            Self::ImageFormat { .. } | Self::ImageFormat(_) => {
                Some("Use a supported image format: png, jpg, jpeg, gif, bmp, webp")
            }
            Self::ConfigInvalid { .. } | Self::ConfigInvalid(_) => {
                Some("Check configuration values for validity")
            }
            _ => None,
        }
    }
}

/// Convenience type alias for Results using `SdError`.
pub type Result<T> = std::result::Result<T, SdError>;

/// Extension trait for adding context to errors.
#[allow(dead_code)] // Reserved for future use
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

#[cfg(test)]
mod tests {
    use super::SdError;

    #[test]
    fn test_connection_error_classification() {
        assert!(SdError::NoDevicesFound.is_connection_error());
        assert!(
            SdError::DeviceNotFound {
                serial: "abc".to_string()
            }
            .is_connection_error()
        );
        assert!(
            SdError::DeviceOpenFailed {
                serial: "abc".to_string(),
                reason: "oops".to_string()
            }
            .is_connection_error()
        );
        assert!(SdError::DeviceCommunication("hid".to_string()).is_connection_error());
        assert!(!SdError::InvalidBrightness { value: 101 }.is_connection_error());
    }

    #[test]
    fn test_image_error_classification() {
        assert!(
            SdError::ImageNotFound {
                path: "/tmp/missing.png".to_string()
            }
            .is_image_error()
        );
        assert!(SdError::ImageProcessing("bad".to_string()).is_image_error());
        assert!(
            SdError::InvalidImageDimensions {
                expected_w: 72,
                expected_h: 72,
                actual_w: 10,
                actual_h: 10
            }
            .is_image_error()
        );
        assert!(!SdError::InvalidBrightness { value: 1 }.is_image_error());
    }

    #[test]
    fn test_config_error_classification() {
        assert!(
            SdError::ConfigNotFound {
                path: "missing.toml".to_string()
            }
            .is_config_error()
        );
        assert!(SdError::ConfigParse("bad".to_string()).is_config_error());
        assert!(
            !SdError::ImageNotFound {
                path: "no.png".to_string()
            }
            .is_config_error()
        );
    }

    #[test]
    fn test_retryable_matches_connection_errors() {
        let err = SdError::DeviceCommunication("hid".to_string());
        assert!(err.is_retryable());
        assert!(!SdError::InvalidBrightness { value: 100 }.is_retryable());
    }
}
