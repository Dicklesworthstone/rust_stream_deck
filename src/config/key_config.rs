//! Key configuration types for declarative Stream Deck profiles.
//!
//! This module provides the [`KeyConfig`] enum which represents different
//! ways to configure a Stream Deck key: image, pattern, color, or clear.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::error::{Result, SdError};

/// Configuration for a single key or key group.
///
/// Each key in a Stream Deck profile can be configured in one of several ways:
/// - An image from a file path
/// - A pattern for batch key assignment (using `{index}` placeholder)
/// - A solid color fill
/// - Cleared (set to black)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KeyConfig {
    /// Static image from file path.
    Image {
        /// Path to the image file (absolute, relative, or with ~ expansion).
        image: PathBuf,
        /// Optional text label overlay (future enhancement).
        #[serde(default)]
        label: Option<String>,
    },

    /// Pattern for batch key assignment.
    ///
    /// The pattern must contain `{index}` which will be replaced with
    /// the key index for each key in the selector.
    Pattern {
        /// Pattern string with `{index}` placeholder.
        pattern: String,
        /// How to handle missing files.
        #[serde(default)]
        missing: MissingBehavior,
    },

    /// Solid color fill.
    Color {
        /// Color specification (hex, RGB array, or named color).
        color: ColorSpec,
    },

    /// Clear key (set to black).
    Clear {
        /// Must be `true` to clear the key.
        clear: bool,
    },
}

/// How to handle missing pattern files.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissingBehavior {
    /// Fail if any file is missing (default).
    #[default]
    Error,
    /// Skip keys with missing files (leave unchanged).
    Skip,
    /// Clear keys with missing files (set to black).
    Clear,
}

/// Color specification supporting multiple input formats.
///
/// Colors can be specified as:
/// - Hex strings: `"#FF5500"` or `"FF5500"`
/// - RGB arrays: `[255, 85, 0]`
/// - Named colors: `"red"`, `"blue"`, etc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ColorSpec {
    /// Hex format: "#FF5500" or "FF5500".
    Hex(String),
    /// RGB array: [255, 85, 0].
    Rgb([u8; 3]),
}

impl ColorSpec {
    /// Parse color specification to RGB values.
    ///
    /// # Errors
    ///
    /// Returns an error if the color specification is invalid.
    pub fn to_rgb(&self) -> Result<(u8, u8, u8)> {
        match self {
            Self::Hex(hex) => parse_hex_color(hex),
            Self::Rgb([r, g, b]) => Ok((*r, *g, *b)),
        }
    }

    /// Get normalized hex representation.
    ///
    /// # Errors
    ///
    /// Returns an error if the color specification is invalid.
    pub fn to_hex(&self) -> Result<String> {
        let (r, g, b) = self.to_rgb()?;
        Ok(format!("#{r:02X}{g:02X}{b:02X}"))
    }
}

/// Parse a hex color string to RGB values.
///
/// Supports both `#RRGGBB` and `RRGGBB` formats.
fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8)> {
    trace!(hex = %hex, "Parsing hex color");
    let hex = hex.trim_start_matches('#');

    // Check for named colors first
    if let Some(rgb) = named_color_to_rgb(hex) {
        debug!(name = %hex, r = rgb.0, g = rgb.1, b = rgb.2, "Resolved named color");
        return Ok(rgb);
    }

    if hex.len() != 6 {
        return Err(SdError::ConfigParse(format!(
            "Invalid hex color '{hex}': expected 6 hex digits"
        )));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| SdError::ConfigParse(format!("Invalid red component in '{hex}'")))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| SdError::ConfigParse(format!("Invalid green component in '{hex}'")))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| SdError::ConfigParse(format!("Invalid blue component in '{hex}'")))?;

    debug!(hex = %hex, r, g, b, "Parsed hex color");
    Ok((r, g, b))
}

/// Convert a named color to RGB values.
///
/// Returns `None` if the color name is not recognized.
fn named_color_to_rgb(name: &str) -> Option<(u8, u8, u8)> {
    let rgb = match name.to_lowercase().as_str() {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 255, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "cyan" => (0, 255, 255),
        "magenta" => (255, 0, 255),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        "pink" => (255, 192, 203),
        "gray" | "grey" => (128, 128, 128),
        _ => {
            warn!(name = %name, "Unknown named color");
            return None;
        }
    };
    Some(rgb)
}

impl KeyConfig {
    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Image path is empty
    /// - Pattern does not contain `{index}` placeholder
    /// - Color specification is invalid
    /// - `clear: false` is specified (should omit the key instead)
    pub fn validate(&self) -> Result<()> {
        trace!(config = ?self, "Validating key config");
        match self {
            Self::Image { image, .. } => {
                // Path validation happens during resolution
                if image.as_os_str().is_empty() {
                    return Err(SdError::ConfigInvalid("Empty image path".to_string()));
                }
                Ok(())
            }
            Self::Pattern { pattern, .. } => {
                if !pattern.contains("{index}") {
                    return Err(SdError::ConfigInvalid(
                        "Pattern must contain {index} placeholder".to_string(),
                    ));
                }
                Ok(())
            }
            Self::Color { color } => {
                color.to_rgb()?; // Validates color spec
                Ok(())
            }
            Self::Clear { clear } => {
                if !clear {
                    return Err(SdError::ConfigInvalid(
                        "clear: false is not allowed; omit the key instead".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Get a human-readable description of this configuration.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Image { image, label } => {
                let mut desc = format!("image: {}", image.display());
                if let Some(l) = label {
                    desc.push_str(&format!(" (label: {l})"));
                }
                desc
            }
            Self::Pattern { pattern, .. } => format!("pattern: {pattern}"),
            Self::Color { color } => {
                if let Ok(hex) = color.to_hex() {
                    format!("color: {hex}")
                } else {
                    "color: (invalid)".to_string()
                }
            }
            Self::Clear { .. } => "clear".to_string(),
        }
    }
}

/// Resolved key configuration (after path expansion and validation).
#[derive(Debug, Clone)]
pub enum ResolvedKey {
    /// Resolved image path.
    Image(PathBuf),
    /// RGB color values.
    Color(u8, u8, u8),
    /// Clear the key (set to black).
    Clear,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_config() {
        let yaml = r#"image: ~/icons/test.png"#;
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Image { image, label } => {
                assert_eq!(image.to_str().unwrap(), "~/icons/test.png");
                assert!(label.is_none());
            }
            _ => panic!("Expected Image config"),
        }
    }

    #[test]
    fn test_parse_image_with_label() {
        let yaml = r#"
image: ~/icons/test.png
label: My App
"#;
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Image { label, .. } => {
                assert_eq!(label, Some("My App".to_string()));
            }
            _ => panic!("Expected Image config"),
        }
    }

    #[test]
    fn test_parse_pattern_config() {
        let yaml = r#"pattern: ~/icons/{index}.png"#;
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Pattern { pattern, missing } => {
                assert_eq!(pattern, "~/icons/{index}.png");
                assert!(matches!(missing, MissingBehavior::Error));
            }
            _ => panic!("Expected Pattern config"),
        }
    }

    #[test]
    fn test_parse_pattern_with_missing() {
        let yaml = r#"
pattern: ~/icons/{index}.png
missing: skip
"#;
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Pattern { missing, .. } => {
                assert!(matches!(missing, MissingBehavior::Skip));
            }
            _ => panic!("Expected Pattern config"),
        }
    }

    #[test]
    fn test_parse_color_hex() {
        let yaml = "color: \"#FF5500\"";
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Color { color } => {
                assert_eq!(color.to_rgb().unwrap(), (255, 85, 0));
            }
            _ => panic!("Expected Color config"),
        }
    }

    #[test]
    fn test_parse_color_hex_no_hash() {
        let yaml = "color: \"FF5500\"";
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Color { color } => {
                assert_eq!(color.to_rgb().unwrap(), (255, 85, 0));
            }
            _ => panic!("Expected Color config"),
        }
    }

    #[test]
    fn test_parse_color_rgb_array() {
        let yaml = r#"color: [255, 85, 0]"#;
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Color { color } => {
                assert_eq!(color.to_rgb().unwrap(), (255, 85, 0));
            }
            _ => panic!("Expected Color config"),
        }
    }

    #[test]
    fn test_parse_color_named() {
        let yaml = r#"color: red"#;
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        match config {
            KeyConfig::Color { color } => {
                assert_eq!(color.to_rgb().unwrap(), (255, 0, 0));
            }
            _ => panic!("Expected Color config"),
        }
    }

    #[test]
    fn test_parse_clear() {
        let yaml = r#"clear: true"#;
        let config: KeyConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(matches!(config, KeyConfig::Clear { clear: true }));
    }

    #[test]
    fn test_validate_empty_image() {
        let config = KeyConfig::Image {
            image: PathBuf::from(""),
            label: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_pattern_no_placeholder() {
        let config = KeyConfig::Pattern {
            pattern: "~/icons/test.png".to_string(),
            missing: MissingBehavior::Error,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_color() {
        // This tests that invalid hex colors fail
        let result = parse_hex_color("GGGGGG");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_clear_false() {
        let config = KeyConfig::Clear { clear: false };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_color_to_hex() {
        let color = ColorSpec::Rgb([255, 85, 0]);
        assert_eq!(color.to_hex().unwrap(), "#FF5500");

        let color = ColorSpec::Hex("red".to_string());
        assert_eq!(color.to_hex().unwrap(), "#FF0000");
    }

    #[test]
    fn test_description() {
        let img = KeyConfig::Image {
            image: PathBuf::from("test.png"),
            label: Some("Test".to_string()),
        };
        assert!(img.description().contains("test.png"));
        assert!(img.description().contains("Test"));

        let color = KeyConfig::Color {
            color: ColorSpec::Hex("#FF0000".to_string()),
        };
        assert!(color.description().contains("#FF0000"));
    }

    #[test]
    fn test_named_colors() {
        let colors = vec![
            ("black", (0, 0, 0)),
            ("white", (255, 255, 255)),
            ("red", (255, 0, 0)),
            ("green", (0, 255, 0)),
            ("blue", (0, 0, 255)),
            ("gray", (128, 128, 128)),
            ("grey", (128, 128, 128)),
        ];

        for (name, expected) in colors {
            assert_eq!(named_color_to_rgb(name).unwrap(), expected);
        }
    }

    #[test]
    fn test_unknown_named_color() {
        assert!(named_color_to_rgb("chartreuse").is_none());
    }

    #[test]
    fn test_toml_parsing() {
        // Parse a key config from TOML format
        let config: KeyConfig = toml::from_str("image = \"test.png\"").unwrap();
        match config {
            KeyConfig::Image { image, .. } => {
                assert_eq!(image.to_str().unwrap(), "test.png");
            }
            _ => panic!("Expected Image config"),
        }
    }

    #[test]
    fn test_color_spec_equality() {
        let c1 = ColorSpec::Hex("#FF0000".to_string());
        let c2 = ColorSpec::Hex("#FF0000".to_string());
        assert_eq!(c1, c2);

        let c3 = ColorSpec::Rgb([255, 0, 0]);
        let c4 = ColorSpec::Rgb([255, 0, 0]);
        assert_eq!(c3, c4);
    }

    #[test]
    fn test_missing_behavior_default() {
        let mb = MissingBehavior::default();
        assert_eq!(mb, MissingBehavior::Error);
    }
}
