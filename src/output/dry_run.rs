//! Dry-run JSON response structures for robot mode.

use serde::Serialize;

use crate::device::DeviceInfo;

/// Common dry-run response wrapper.
#[derive(Debug, Serialize)]
pub struct DryRunResponse<T: Serialize> {
    /// Always true in dry-run mode.
    pub dry_run: bool,
    /// The action that would be performed.
    pub action: String,
    /// Whether the operation would succeed.
    pub would_succeed: bool,
    /// If would_succeed is false, the reason why.
    pub failure_reason: Option<String>,
    /// Validation details.
    pub validation: ValidationDetails,
    /// Action-specific details.
    pub details: T,
    /// Device context.
    pub device: DeviceContext,
}

/// Validation details for dry-run inputs.
#[derive(Debug, Serialize)]
pub struct ValidationDetails {
    /// Whether all inputs are valid.
    pub inputs_valid: bool,
    /// List of validation errors.
    pub errors: Vec<ValidationError>,
    /// List of validation warnings.
    pub warnings: Vec<String>,
}

/// A single validation error with an optional suggestion.
#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub field: String,
    pub error: String,
    pub suggestion: Option<String>,
}

/// Device context for dry-run output.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceContext {
    pub model: String,
    pub serial: Option<String>,
    pub connected: bool,
    pub key_count: u8,
    pub key_dimensions: (u32, u32),
}

impl DeviceContext {
    /// Build a device context from known device info.
    #[must_use]
    pub fn from_info(info: &DeviceInfo) -> Self {
        Self {
            model: info.product_name.clone(),
            serial: Some(info.serial.clone()),
            connected: true,
            key_count: info.key_count,
            key_dimensions: (info.key_width as u32, info.key_height as u32),
        }
    }

    /// Build a disconnected device context with optional serial.
    #[must_use]
    pub fn disconnected(serial: Option<String>) -> Self {
        Self {
            model: "unknown".to_string(),
            serial,
            connected: false,
            key_count: 0,
            key_dimensions: (0, 0),
        }
    }
}

impl<T: Serialize> DryRunResponse<T> {
    /// Success constructor.
    #[must_use]
    pub fn success(action: &str, details: T, device: DeviceContext) -> Self {
        Self {
            dry_run: true,
            action: action.to_string(),
            would_succeed: true,
            failure_reason: None,
            validation: ValidationDetails {
                inputs_valid: true,
                errors: vec![],
                warnings: vec![],
            },
            details,
            device,
        }
    }

    /// Failure constructor.
    #[must_use]
    pub fn failure(
        action: &str,
        reason: &str,
        errors: Vec<ValidationError>,
        details: T,
        device: DeviceContext,
    ) -> Self {
        Self {
            dry_run: true,
            action: action.to_string(),
            would_succeed: false,
            failure_reason: Some(reason.to_string()),
            validation: ValidationDetails {
                inputs_valid: false,
                errors,
                warnings: vec![],
            },
            details,
            device,
        }
    }

    /// Attach warnings to an existing response.
    #[must_use]
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.validation.warnings = warnings;
        self
    }
}

// === Command-specific dry-run details ===

/// Dry-run details for brightness command.
#[derive(Debug, Serialize)]
pub struct BrightnessDryRunDetails {
    /// Target brightness level (0-100).
    pub target_level: u8,
    /// Current brightness level (if known).
    pub current_level: Option<u8>,
    /// Human-readable description of the change.
    pub description: String,
}

impl BrightnessDryRunDetails {
    /// Create new brightness dry-run details.
    #[must_use]
    pub fn new(target_level: u8, current_level: Option<u8>) -> Self {
        let description = match current_level {
            Some(current) if current == target_level => {
                format!("Brightness already at {target_level}%")
            }
            Some(current) => {
                let direction = if target_level > current {
                    "increase"
                } else {
                    "decrease"
                };
                format!("Would {direction} brightness from {current}% to {target_level}%")
            }
            None => format!("Would set brightness to {target_level}%"),
        };
        Self {
            target_level,
            current_level,
            description,
        }
    }
}

/// Dry-run details for set-key command.
#[derive(Debug, Serialize)]
pub struct SetKeyDryRunDetails {
    /// Target key index.
    pub key: u8,
    /// Source image information.
    pub source: ImageSourceInfo,
    /// Processing requirements.
    pub processing: ProcessingInfo,
}

/// Information about the source image.
#[derive(Debug, Clone, Serialize)]
pub struct ImageSourceInfo {
    /// Path to the image file.
    pub path: String,
    /// Whether the file exists.
    pub exists: bool,
    /// Whether the file is readable.
    pub readable: bool,
    /// Image format (if detected).
    pub format: Option<String>,
    /// Image dimensions (width, height) if readable.
    pub dimensions: Option<(u32, u32)>,
    /// File size in bytes.
    pub size_bytes: Option<u64>,
}

/// Processing requirements for an image.
#[derive(Debug, Serialize)]
pub struct ProcessingInfo {
    /// Whether the image needs to be resized.
    pub resize_needed: bool,
    /// Target dimensions (width, height).
    pub target_dimensions: (u32, u32),
}

impl SetKeyDryRunDetails {
    /// Create new set-key dry-run details.
    #[must_use]
    pub fn new(key: u8, source: ImageSourceInfo, processing: ProcessingInfo) -> Self {
        Self {
            key,
            source,
            processing,
        }
    }
}

/// Dry-run details for fill-key command.
#[derive(Debug, Serialize)]
pub struct FillKeyDryRunDetails {
    /// Target key index.
    pub key: u8,
    /// Color in hex format (with # prefix).
    pub color: String,
    /// RGB components.
    pub rgb: (u8, u8, u8),
    /// Human-readable description.
    pub description: String,
}

impl FillKeyDryRunDetails {
    /// Create new fill-key dry-run details.
    #[must_use]
    pub fn new(key: u8, color: String, rgb: (u8, u8, u8)) -> Self {
        let description = format!(
            "Would fill key {} with color {} (R:{}, G:{}, B:{})",
            key, color, rgb.0, rgb.1, rgb.2
        );
        Self {
            key,
            color,
            rgb,
            description,
        }
    }
}

/// Dry-run details for clear-key command.
#[derive(Debug, Serialize)]
pub struct ClearKeyDryRunDetails {
    /// Target key index.
    pub key: u8,
    /// Human-readable description.
    pub description: String,
}

impl ClearKeyDryRunDetails {
    /// Create new clear-key dry-run details.
    #[must_use]
    pub fn new(key: u8) -> Self {
        Self {
            key,
            description: format!("Would clear key {} (set to black)", key),
        }
    }
}

/// Dry-run details for clear-all command.
#[derive(Debug, Serialize)]
pub struct ClearAllDryRunDetails {
    /// Total number of keys that would be cleared.
    pub key_count: u8,
    /// Human-readable description.
    pub description: String,
}

impl ClearAllDryRunDetails {
    /// Create new clear-all dry-run details.
    #[must_use]
    pub fn new(key_count: u8) -> Self {
        Self {
            key_count,
            description: format!("Would clear all {} keys (set to black)", key_count),
        }
    }
}

/// Dry-run details for clear-keys (batch) command.
#[derive(Debug, Serialize)]
pub struct ClearKeysDryRunDetails {
    /// List of keys that would be cleared.
    pub keys: Vec<u8>,
    /// Total number of keys that would be cleared.
    pub total_count: usize,
    /// Human-readable description.
    pub description: String,
}

impl ClearKeysDryRunDetails {
    /// Create new clear-keys dry-run details.
    #[must_use]
    pub fn new(keys: Vec<u8>) -> Self {
        let total_count = keys.len();
        let description = if total_count == 1 {
            format!("Would clear key {}", keys[0])
        } else {
            format!("Would clear {} keys: {:?}", total_count, keys)
        };
        Self {
            keys,
            total_count,
            description,
        }
    }
}
