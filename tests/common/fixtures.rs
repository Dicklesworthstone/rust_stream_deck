//! Test fixture helpers for creating temporary test data.
//!
//! Provides utilities for generating temporary directories with test images
//! and configuration files that are automatically cleaned up.

use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage};
use tempfile::TempDir;

/// Get the path to the test fixtures directory.
#[must_use]
pub fn fixtures_path(subpath: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(subpath)
}

/// Test images in a temporary directory with automatic cleanup.
///
/// # Example
///
/// ```ignore
/// let images = TestImages::create_batch(32, 72);
/// // Use images.path() in tests
/// // Directory is automatically cleaned up when `images` is dropped
/// ```
pub struct TestImages {
    /// The temporary directory containing the images.
    pub dir: TempDir,
}

impl TestImages {
    /// Create a temporary directory with N solid-color test images.
    ///
    /// Images are named `key-0.png`, `key-1.png`, etc.
    /// Colors vary based on index for visual distinction.
    ///
    /// # Panics
    ///
    /// Panics if image creation fails.
    #[must_use]
    pub fn create_batch(count: u8, size: u32) -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");

        for i in 0..count {
            let color = Rgb([
                i.wrapping_mul(8),
                i.wrapping_mul(16),
                i.wrapping_mul(24),
            ]);
            let img = RgbImage::from_pixel(size, size, color);
            let path = dir.path().join(format!("key-{i}.png"));
            img.save(&path)
                .unwrap_or_else(|_| panic!("Failed to save image at {path:?}"));
        }

        Self { dir }
    }

    /// Create images only for specific key indices.
    ///
    /// Useful for testing partial batch operations.
    ///
    /// # Panics
    ///
    /// Panics if image creation fails.
    #[must_use]
    pub fn create_numbered(keys: &[u8], size: u32) -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");

        for &key in keys {
            let img = RgbImage::from_pixel(
                size,
                size,
                Rgb([key.wrapping_mul(8), 128, 255_u8.wrapping_sub(key.wrapping_mul(8))]),
            );
            let path = dir.path().join(format!("key-{key}.png"));
            img.save(&path)
                .unwrap_or_else(|_| panic!("Failed to save image at {path:?}"));
        }

        Self { dir }
    }

    /// Create images with a custom naming pattern.
    ///
    /// The pattern should contain `{index}` which will be replaced with the key number.
    ///
    /// # Panics
    ///
    /// Panics if image creation fails.
    #[must_use]
    pub fn create_with_pattern(count: u8, size: u32, pattern: &str) -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");

        for i in 0..count {
            let color = Rgb([
                i.wrapping_mul(8),
                i.wrapping_mul(16),
                i.wrapping_mul(24),
            ]);
            let img = RgbImage::from_pixel(size, size, color);
            let filename = pattern.replace("{index}", &i.to_string());
            let path = dir.path().join(filename);
            img.save(&path)
                .unwrap_or_else(|_| panic!("Failed to save image at {path:?}"));
        }

        Self { dir }
    }

    /// Get the path to the temporary directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Get the path as a string (useful for CLI arguments).
    ///
    /// # Panics
    ///
    /// Panics if the path is not valid UTF-8.
    #[must_use]
    pub fn path_str(&self) -> &str {
        self.dir.path().to_str().expect("Path is not valid UTF-8")
    }
}

/// Temporary configuration file with automatic cleanup.
///
/// # Example
///
/// ```ignore
/// let config = TestConfig::yaml("brightness: 80\nlayout: default");
/// // Use config.config_path in tests
/// ```
pub struct TestConfig {
    /// The temporary directory containing the config file.
    pub dir: TempDir,
    /// Path to the configuration file.
    pub config_path: PathBuf,
}

impl TestConfig {
    /// Create a temporary YAML configuration file.
    ///
    /// # Panics
    ///
    /// Panics if file creation fails.
    #[must_use]
    pub fn yaml(content: &str) -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");
        let config_path = dir.path().join("config.yaml");
        std::fs::write(&config_path, content).expect("Failed to write config file");
        Self { dir, config_path }
    }

    /// Create a temporary TOML configuration file.
    ///
    /// # Panics
    ///
    /// Panics if file creation fails.
    #[must_use]
    pub fn toml(content: &str) -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");
        let config_path = dir.path().join("config.toml");
        std::fs::write(&config_path, content).expect("Failed to write config file");
        Self { dir, config_path }
    }

    /// Create a temporary JSON configuration file.
    ///
    /// # Panics
    ///
    /// Panics if file creation fails.
    #[must_use]
    pub fn json(content: &str) -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, content).expect("Failed to write config file");
        Self { dir, config_path }
    }

    /// Get the config file path as a string.
    ///
    /// # Panics
    ///
    /// Panics if the path is not valid UTF-8.
    #[must_use]
    pub fn path_str(&self) -> &str {
        self.config_path.to_str().expect("Path is not valid UTF-8")
    }
}

/// Temporary directory for general test use.
///
/// Provides a clean temporary directory that is automatically cleaned up.
pub struct TestDir {
    /// The temporary directory.
    pub dir: TempDir,
}

impl TestDir {
    /// Create a new empty temporary directory.
    ///
    /// # Panics
    ///
    /// Panics if directory creation fails.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    /// Get the path to the temporary directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Get the path as a string.
    ///
    /// # Panics
    ///
    /// Panics if the path is not valid UTF-8.
    #[must_use]
    pub fn path_str(&self) -> &str {
        self.dir.path().to_str().expect("Path is not valid UTF-8")
    }

    /// Write a file to the temporary directory.
    ///
    /// # Panics
    ///
    /// Panics if file writing fails.
    pub fn write_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, content).expect("Failed to write file");
        path
    }
}

impl Default for TestDir {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_batch_images() {
        let images = TestImages::create_batch(4, 72);

        for i in 0..4 {
            let path = images.path().join(format!("key-{i}.png"));
            assert!(path.exists(), "Image key-{i}.png should exist");
        }
    }

    #[test]
    fn test_create_numbered_images() {
        let images = TestImages::create_numbered(&[0, 5, 10], 72);

        assert!(images.path().join("key-0.png").exists());
        assert!(images.path().join("key-5.png").exists());
        assert!(images.path().join("key-10.png").exists());
        assert!(!images.path().join("key-1.png").exists());
    }

    #[test]
    fn test_yaml_config() {
        let config = TestConfig::yaml("brightness: 80");
        assert!(config.config_path.exists());

        let content = std::fs::read_to_string(&config.config_path).unwrap();
        assert_eq!(content, "brightness: 80");
    }

    #[test]
    fn test_temp_dir_cleanup() {
        let path: PathBuf;
        {
            let dir = TestDir::new();
            path = dir.path().to_path_buf();
            dir.write_file("test.txt", b"hello");
            assert!(path.exists());
        }
        // Directory should be cleaned up after drop
        assert!(!path.exists());
    }
}
