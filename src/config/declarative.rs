//! Declarative configuration loading for Stream Deck profiles.
//!
//! This module provides loading of user-defined profiles from YAML or TOML files.
//! Unlike the [`loader`](super::loader) module which handles Elgato's
//! `.streamDeckProfile` format, this module handles our declarative config format.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, trace, warn};

use crate::error::{Result, SdError};

use super::{KeyConfig, KeySelector};

/// Configuration file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    /// YAML format (.yaml, .yml).
    Yaml,
    /// TOML format (.toml).
    Toml,
}

impl ConfigFormat {
    /// Detect format from file extension.
    ///
    /// Returns `None` if the extension is not recognized.
    #[must_use]
    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        trace!(extension = %ext, "Detecting config format from extension");
        match ext.to_lowercase().as_str() {
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }

    /// Get the canonical file extension for this format.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Toml => "toml",
        }
    }
}

/// Declarative profile configuration.
///
/// Represents a complete profile that can be loaded from YAML or TOML.
/// Keys are mapped using [`KeySelector`] strings to [`KeyConfig`] values.
///
/// # Example YAML
///
/// ```yaml
/// name: My Profile
/// brightness: 75
/// keys:
///   "0":
///     image: ~/icons/chrome.png
///   "8-15":
///     pattern: ./numbers/{index}.png
///   "row-3":
///     color: "#222222"
///   "default":
///     clear: true
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ProfileConfig {
    /// Optional profile name for identification.
    #[serde(default)]
    pub name: Option<String>,

    /// Target device serial number.
    ///
    /// If specified, the profile will only apply to this device.
    /// If omitted, applies to the first available device.
    #[serde(default)]
    pub device: Option<String>,

    /// Brightness level (0-100).
    ///
    /// If specified, sets the device brightness when applying the profile.
    #[serde(default)]
    pub brightness: Option<u8>,

    /// Key configurations mapped by selector.
    ///
    /// Keys are [`KeySelector`] strings (e.g., "0", "8-15", "row-0", "default").
    /// Values are [`KeyConfig`] entries specifying how to configure each key.
    #[serde(default)]
    pub keys: HashMap<String, KeyConfig>,
}

impl ProfileConfig {
    /// Create an empty profile configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the profile configuration.
    ///
    /// Checks that:
    /// - Brightness is in range 0-100
    /// - All key selectors are valid
    /// - All key configs are valid
    ///
    /// # Errors
    ///
    /// Returns an error if any validation check fails.
    pub fn validate(&self) -> Result<()> {
        trace!(name = ?self.name, "Validating profile config");

        // Validate brightness range
        if let Some(brightness) = self.brightness {
            if brightness > 100 {
                return Err(SdError::InvalidBrightness { value: brightness });
            }
            debug!(brightness, "Brightness validated");
        }

        // Validate each key entry
        for (selector_str, config) in &self.keys {
            // Validate selector syntax
            KeySelector::parse(selector_str).map_err(|e| {
                SdError::ConfigParse(format!("Invalid key selector '{selector_str}': {e}"))
            })?;

            // Validate key config
            config.validate().map_err(|e| {
                SdError::ConfigInvalid(format!("Invalid config for key '{selector_str}': {e}"))
            })?;
        }

        debug!(keys = self.keys.len(), "All key entries validated");
        Ok(())
    }

    /// Parse and validate key selectors.
    ///
    /// Returns a vector of (selector, config) pairs sorted by priority.
    /// Higher priority (more specific) selectors come first.
    ///
    /// # Errors
    ///
    /// Returns an error if any selector string is invalid.
    pub fn parsed_keys(&self) -> Result<Vec<(KeySelector, &KeyConfig)>> {
        let mut entries: Vec<(KeySelector, &KeyConfig)> = Vec::with_capacity(self.keys.len());

        for (selector_str, config) in &self.keys {
            let selector = KeySelector::parse(selector_str)?;
            entries.push((selector, config));
        }

        // Sort by priority (lower number = higher priority = comes first)
        entries.sort_by_key(|(sel, _)| sel.priority());

        trace!(
            count = entries.len(),
            "Parsed and sorted key entries by priority"
        );
        Ok(entries)
    }
}

/// Load a profile configuration from a file.
///
/// Automatically detects the format from the file extension:
/// - `.yaml` or `.yml` → YAML
/// - `.toml` → TOML
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The format cannot be detected from the extension
/// - The file content cannot be parsed
/// - Validation fails
#[instrument(skip_all, fields(path = %path.as_ref().display()))]
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<ProfileConfig> {
    let path = path.as_ref();
    info!("Loading configuration file");

    // Detect format from extension
    let format = ConfigFormat::from_extension(path).ok_or_else(|| {
        SdError::ConfigParse(format!(
            "Unknown config format for '{}': expected .yaml, .yml, or .toml",
            path.display()
        ))
    })?;
    debug!(format = ?format, "Detected config format");

    // Read file content
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SdError::ConfigNotFound {
                path: path.display().to_string(),
            }
        } else {
            SdError::Io(e)
        }
    })?;
    debug!(bytes = content.len(), "Read config file");

    // Parse based on format
    load_config_from_str(&content, format)
}

/// Load a profile configuration from a string with a specified format.
///
/// # Errors
///
/// Returns an error if parsing or validation fails.
#[instrument(skip(content), fields(format = ?format, content_len = content.len()))]
pub fn load_config_from_str(content: &str, format: ConfigFormat) -> Result<ProfileConfig> {
    trace!("Parsing config content");

    let config: ProfileConfig = match format {
        ConfigFormat::Yaml => {
            serde_yaml::from_str(content).map_err(|e| SdError::ConfigParse(format!("YAML: {e}")))?
        }
        ConfigFormat::Toml => {
            toml::from_str(content).map_err(|e| SdError::ConfigParse(format!("TOML: {e}")))?
        }
    };

    // Handle warnings for unknown fields (serde ignores them by default)
    // This is a forward-compatibility feature
    trace!(name = ?config.name, "Parsed config structure");

    // Validate the configuration
    config.validate()?;

    info!(
        name = ?config.name,
        keys = config.keys.len(),
        brightness = ?config.brightness,
        device = ?config.device,
        "Configuration loaded and validated"
    );

    Ok(config)
}

/// Save a profile configuration to a file.
///
/// Automatically detects the format from the file extension.
///
/// # Errors
///
/// Returns an error if:
/// - The format cannot be detected from the extension
/// - Serialization fails
/// - The file cannot be written
#[instrument(skip(config), fields(path = %path.as_ref().display()))]
pub fn save_config<P: AsRef<Path>>(config: &ProfileConfig, path: P) -> Result<()> {
    let path = path.as_ref();
    info!("Saving configuration file");

    // Detect format from extension
    let format = ConfigFormat::from_extension(path).ok_or_else(|| {
        SdError::ConfigParse(format!(
            "Unknown config format for '{}': expected .yaml, .yml, or .toml",
            path.display()
        ))
    })?;
    debug!(format = ?format, "Using config format");

    // Serialize to string
    let content = match format {
        ConfigFormat::Yaml => {
            serde_yaml::to_string(config).map_err(|e| SdError::ConfigParse(format!("YAML: {e}")))?
        }
        ConfigFormat::Toml => toml::to_string_pretty(config)
            .map_err(|e| SdError::ConfigParse(format!("TOML: {e}")))?,
    };

    // Write to file
    std::fs::write(path, content)?;

    info!(
        bytes = path.metadata().map(|m| m.len()).unwrap_or(0),
        "Configuration saved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ColorSpec, MissingBehavior};
    use std::path::PathBuf;

    #[test]
    fn test_format_detection_yaml() {
        assert_eq!(
            ConfigFormat::from_extension(Path::new("config.yaml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_extension(Path::new("config.yml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_extension(Path::new("config.YAML")),
            Some(ConfigFormat::Yaml)
        );
    }

    #[test]
    fn test_format_detection_toml() {
        assert_eq!(
            ConfigFormat::from_extension(Path::new("config.toml")),
            Some(ConfigFormat::Toml)
        );
        assert_eq!(
            ConfigFormat::from_extension(Path::new("profile.TOML")),
            Some(ConfigFormat::Toml)
        );
    }

    #[test]
    fn test_format_detection_unknown() {
        assert_eq!(ConfigFormat::from_extension(Path::new("config.json")), None);
        assert_eq!(ConfigFormat::from_extension(Path::new("config.txt")), None);
        assert_eq!(ConfigFormat::from_extension(Path::new("config")), None);
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(ConfigFormat::Yaml.extension(), "yaml");
        assert_eq!(ConfigFormat::Toml.extension(), "toml");
    }

    #[test]
    fn test_load_yaml_minimal() {
        let yaml = r#"
name: Test Profile
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.name, Some("Test Profile".to_string()));
        assert!(config.keys.is_empty());
    }

    #[test]
    fn test_load_yaml_full() {
        let yaml = r##"
name: Full Profile
device: ABC123
brightness: 75
keys:
  "0":
    image: ~/icons/chrome.png
  "8-15":
    pattern: ./numbers/{index}.png
  "row-3":
    color: "#222222"
  "default":
    clear: true
"##;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();

        assert_eq!(config.name, Some("Full Profile".to_string()));
        assert_eq!(config.device, Some("ABC123".to_string()));
        assert_eq!(config.brightness, Some(75));
        assert_eq!(config.keys.len(), 4);
    }

    #[test]
    fn test_load_toml_minimal() {
        let toml_str = r#"
name = "Test Profile"
"#;
        let config = load_config_from_str(toml_str, ConfigFormat::Toml).unwrap();
        assert_eq!(config.name, Some("Test Profile".to_string()));
    }

    #[test]
    fn test_load_toml_full() {
        let toml_str = r##"
name = "Full Profile"
device = "ABC123"
brightness = 75

[keys."0"]
image = "~/icons/chrome.png"

[keys."8-15"]
pattern = "./numbers/{index}.png"

[keys."row-3"]
color = "#222222"

[keys."default"]
clear = true
"##;
        let config = load_config_from_str(toml_str, ConfigFormat::Toml).unwrap();

        assert_eq!(config.name, Some("Full Profile".to_string()));
        assert_eq!(config.device, Some("ABC123".to_string()));
        assert_eq!(config.brightness, Some(75));
        assert_eq!(config.keys.len(), 4);
    }

    #[test]
    fn test_load_empty_yaml() {
        let yaml = "";
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        assert!(config.name.is_none());
        assert!(config.keys.is_empty());
    }

    #[test]
    fn test_load_empty_toml() {
        let toml_str = "";
        let config = load_config_from_str(toml_str, ConfigFormat::Toml).unwrap();
        assert!(config.name.is_none());
        assert!(config.keys.is_empty());
    }

    #[test]
    fn test_validate_brightness_valid() {
        let config = ProfileConfig {
            brightness: Some(100),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_brightness_invalid() {
        let config = ProfileConfig {
            brightness: Some(101),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_selector() {
        let mut keys = HashMap::new();
        keys.insert(
            "invalid-selector".to_string(),
            KeyConfig::Clear { clear: true },
        );
        let config = ProfileConfig {
            keys,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_key_config() {
        let mut keys = HashMap::new();
        keys.insert(
            "0".to_string(),
            KeyConfig::Pattern {
                pattern: "no-placeholder.png".to_string(), // Missing {index}
                missing: MissingBehavior::Error,
            },
        );
        let config = ProfileConfig {
            keys,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_parsed_keys_priority_order() {
        let yaml = r#"
keys:
  "default":
    clear: true
  "row-0":
    color: red
  "0":
    color: blue
  "1-5":
    color: green
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        let parsed = config.parsed_keys().unwrap();

        // Should be ordered: Single, Range, Row, Default
        assert!(matches!(parsed[0].0, KeySelector::Single(0)));
        assert!(matches!(
            parsed[1].0,
            KeySelector::Range { start: 1, end: 5 }
        ));
        assert!(matches!(parsed[2].0, KeySelector::Row(0)));
        assert!(matches!(parsed[3].0, KeySelector::Default));
    }

    #[test]
    fn test_key_config_image_yaml() {
        let yaml = r#"
keys:
  "0":
    image: /path/to/image.png
    label: My Label
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        let key_config = config.keys.get("0").unwrap();

        match key_config {
            KeyConfig::Image { image, label } => {
                assert_eq!(image, &PathBuf::from("/path/to/image.png"));
                assert_eq!(label, &Some("My Label".to_string()));
            }
            _ => panic!("Expected Image config"),
        }
    }

    #[test]
    fn test_key_config_pattern_yaml() {
        let yaml = r#"
keys:
  "0-7":
    pattern: ./icons/{index}.png
    missing: skip
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        let key_config = config.keys.get("0-7").unwrap();

        match key_config {
            KeyConfig::Pattern { pattern, missing } => {
                assert_eq!(pattern, "./icons/{index}.png");
                assert_eq!(missing, &MissingBehavior::Skip);
            }
            _ => panic!("Expected Pattern config"),
        }
    }

    #[test]
    fn test_key_config_color_hex_yaml() {
        let yaml = r##"
keys:
  "0":
    color: "#FF5500"
"##;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        let key_config = config.keys.get("0").unwrap();

        match key_config {
            KeyConfig::Color { color } => {
                assert_eq!(color.to_rgb().unwrap(), (255, 85, 0));
            }
            _ => panic!("Expected Color config"),
        }
    }

    #[test]
    fn test_key_config_color_rgb_yaml() {
        let yaml = r#"
keys:
  "0":
    color: [255, 85, 0]
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        let key_config = config.keys.get("0").unwrap();

        match key_config {
            KeyConfig::Color { color } => {
                assert_eq!(color.to_rgb().unwrap(), (255, 85, 0));
            }
            _ => panic!("Expected Color config"),
        }
    }

    #[test]
    fn test_key_config_color_named_yaml() {
        let yaml = r#"
keys:
  "0":
    color: red
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        let key_config = config.keys.get("0").unwrap();

        match key_config {
            KeyConfig::Color { color } => {
                assert_eq!(color.to_rgb().unwrap(), (255, 0, 0));
            }
            _ => panic!("Expected Color config"),
        }
    }

    #[test]
    fn test_key_config_clear_yaml() {
        let yaml = r#"
keys:
  "default":
    clear: true
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        let key_config = config.keys.get("default").unwrap();

        assert!(matches!(key_config, KeyConfig::Clear { clear: true }));
    }

    #[test]
    fn test_yaml_parse_error() {
        let yaml = "invalid: [yaml: content";
        let result = load_config_from_str(yaml, ConfigFormat::Yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SdError::ConfigParse(_)));
    }

    #[test]
    fn test_toml_parse_error() {
        let toml_str = "invalid = [toml content";
        let result = load_config_from_str(toml_str, ConfigFormat::Toml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SdError::ConfigParse(_)));
    }

    #[test]
    fn test_profile_config_new() {
        let config = ProfileConfig::new();
        assert!(config.name.is_none());
        assert!(config.device.is_none());
        assert!(config.brightness.is_none());
        assert!(config.keys.is_empty());
    }

    #[test]
    fn test_unknown_fields_ignored() {
        // YAML with unknown fields should be parsed (forward compatibility)
        let yaml = r#"
name: Test
unknown_field: some value
another_unknown:
  nested: true
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.name, Some("Test".to_string()));
    }

    #[test]
    fn test_roundtrip_yaml() {
        let mut keys = HashMap::new();
        keys.insert(
            "0".to_string(),
            KeyConfig::Color {
                color: ColorSpec::Hex("#FF0000".to_string()),
            },
        );

        let config = ProfileConfig {
            name: Some("Test".to_string()),
            device: None,
            brightness: Some(80),
            keys,
        };

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Parse back
        let parsed: ProfileConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.name, parsed.name);
        assert_eq!(config.brightness, parsed.brightness);
        assert_eq!(config.keys.len(), parsed.keys.len());
    }

    #[test]
    fn test_save_and_load_yaml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.yaml");

        let mut keys = HashMap::new();
        keys.insert("0".to_string(), KeyConfig::Clear { clear: true });

        let config = ProfileConfig {
            name: Some("Save Test".to_string()),
            brightness: Some(50),
            keys,
            ..Default::default()
        };

        // Save
        save_config(&config, &path).unwrap();

        // Load
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.name, config.name);
        assert_eq!(loaded.brightness, config.brightness);
    }

    #[test]
    fn test_save_and_load_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.toml");

        let mut keys = HashMap::new();
        keys.insert("0".to_string(), KeyConfig::Clear { clear: true });

        let config = ProfileConfig {
            name: Some("Save Test".to_string()),
            brightness: Some(50),
            keys,
            ..Default::default()
        };

        // Save
        save_config(&config, &path).unwrap();

        // Load
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.name, config.name);
        assert_eq!(loaded.brightness, config.brightness);
    }

    #[test]
    fn test_load_file_not_found() {
        let result = load_config("/nonexistent/path/config.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SdError::ConfigNotFound { .. }));
    }

    #[test]
    fn test_load_unknown_extension() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        std::fs::write(&path, "{}").unwrap();

        let result = load_config(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SdError::ConfigParse(_)));
    }

    #[test]
    fn test_all_selector_types_yaml() {
        let yaml = r#"
keys:
  "0":
    clear: true
  "1-5":
    clear: true
  "row-0":
    clear: true
  "col-0":
    clear: true
  "default":
    clear: true
"#;
        let config = load_config_from_str(yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.keys.len(), 5);
        assert!(config.validate().is_ok());
    }
}
