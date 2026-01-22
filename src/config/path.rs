//! Path resolution helpers for declarative configuration files.
//!
//! Supports absolute paths, paths relative to the config file, and "~" home
//! directory expansion.

use std::path::{Path, PathBuf};

use tracing::{debug, trace, warn};

use crate::error::{Result, SdError};

/// Resolve a path from a config file.
///
/// Resolution rules:
/// 1. Absolute paths: used as-is
/// 2. Paths starting with `~`: expanded to home directory
/// 3. Relative paths: resolved relative to the config file's directory
pub fn resolve_path(path: &Path, config_dir: &Path) -> Result<PathBuf> {
    trace!(
        path = %path.display(),
        config_dir = %config_dir.display(),
        "Resolving path"
    );

    let path_str = path.to_string_lossy();

    // Home directory expansion
    if path_str == "~" || path_str.starts_with("~/") {
        let home = home_dir()?;
        let rest = path_str.strip_prefix("~/").unwrap_or("");
        let resolved = if rest.is_empty() {
            home
        } else {
            home.join(rest)
        };
        debug!(
            original = %path.display(),
            resolved = %resolved.display(),
            "Expanded home directory path"
        );
        return Ok(resolved);
    }

    // Absolute path
    if path.is_absolute() {
        debug!(path = %path.display(), "Using absolute path as-is");
        return Ok(path.to_path_buf());
    }

    // Relative path
    let resolved = config_dir.join(path);
    debug!(
        original = %path.display(),
        config_dir = %config_dir.display(),
        resolved = %resolved.display(),
        "Resolved relative path"
    );
    Ok(resolved)
}

/// Resolve the user's home directory (cross-platform).
pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| {
        SdError::ConfigInvalid("Could not determine home directory".to_string())
    })
}

/// Validate that a path exists and is a supported image file.
pub fn validate_image_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(SdError::ImageNotFound {
            path: path.display().to_string(),
        });
    }

    if !path.is_file() {
        return Err(SdError::ImageNotFound {
            path: format!("{} is not a file", path.display()),
        });
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp") => Ok(()),
        Some(other) => Err(SdError::ImageFormat(format!(
            "Unsupported image format: .{other}"
        ))),
        None => Err(SdError::ImageFormat(
            "Image file has no extension".to_string(),
        )),
    }
}

/// Path resolution context for a config file.
pub struct PathResolver {
    config_dir: PathBuf,
}

impl PathResolver {
    /// Create a resolver for a specific config file path.
    pub fn new(config_path: &Path) -> Result<Self> {
        let config_dir = config_path.parent().ok_or_else(|| {
            SdError::ConfigInvalid(format!(
                "Config path has no parent directory: {}",
                config_path.display()
            ))
        })?;

        let canonical = config_dir.canonicalize().unwrap_or_else(|_| {
            warn!(
                config_dir = %config_dir.display(),
                "Failed to canonicalize config directory"
            );
            config_dir.to_path_buf()
        });

        Ok(Self { config_dir: canonical })
    }

    /// Resolve a path relative to the config file.
    pub fn resolve(&self, path: &Path) -> Result<PathBuf> {
        resolve_path(path, &self.config_dir)
    }

    /// Resolve and validate an image path.
    pub fn resolve_image(&self, path: &Path) -> Result<PathBuf> {
        let resolved = self.resolve(path)?;
        validate_image_path(&resolved)?;
        Ok(resolved)
    }

    /// Return the base config directory.
    pub const fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_absolute_path() {
        let config_dir = Path::new("/some/config/dir");
        let path = Path::new("/absolute/path/to/image.png");

        let resolved = resolve_path(path, config_dir).unwrap();
        assert_eq!(resolved, PathBuf::from("/absolute/path/to/image.png"));
    }

    #[test]
    fn test_relative_path() {
        let config_dir = Path::new("/home/user/.config/sd");
        let path = Path::new("icons/test.png");

        let resolved = resolve_path(path, config_dir).unwrap();
        assert_eq!(
            resolved,
            PathBuf::from("/home/user/.config/sd/icons/test.png")
        );
    }

    #[test]
    fn test_relative_path_with_dots() {
        let config_dir = Path::new("/home/user/.config/sd/profiles");
        let path = Path::new("../icons/test.png");

        let resolved = resolve_path(path, config_dir).unwrap();
        assert_eq!(
            resolved,
            PathBuf::from("/home/user/.config/sd/profiles/../icons/test.png")
        );
    }

    #[test]
    fn test_home_expansion() {
        let config_dir = Path::new("/some/config/dir");
        let path = Path::new("~/icons/test.png");

        let resolved = resolve_path(path, config_dir).unwrap();

        let home = home_dir().unwrap();
        assert!(resolved.starts_with(&home));
        assert!(resolved.ends_with("icons/test.png"));
    }

    #[test]
    fn test_home_only() {
        let config_dir = Path::new("/some/config/dir");
        let path = Path::new("~");

        let resolved = resolve_path(path, config_dir).unwrap();
        let home = home_dir().unwrap();
        assert_eq!(resolved, home);
    }

    #[test]
    fn test_validate_existing_image() {
        let temp = TempDir::new().unwrap();
        let img_path = temp.path().join("test.png");
        File::create(&img_path).unwrap();

        assert!(validate_image_path(&img_path).is_ok());
    }

    #[test]
    fn test_validate_missing_image() {
        let result = validate_image_path(Path::new("/nonexistent/image.png"));
        assert!(matches!(result, Err(SdError::ImageNotFound { .. })));
    }

    #[test]
    fn test_validate_directory_not_file() {
        let temp = TempDir::new().unwrap();
        let result = validate_image_path(temp.path());
        assert!(matches!(result, Err(SdError::ImageNotFound { .. })));
    }

    #[test]
    fn test_validate_unsupported_format() {
        let temp = TempDir::new().unwrap();
        let txt_path = temp.path().join("test.txt");
        File::create(&txt_path).unwrap();

        let result = validate_image_path(&txt_path);
        assert!(matches!(result, Err(SdError::ImageFormat(_))));
    }

    #[test]
    fn test_validate_no_extension() {
        let temp = TempDir::new().unwrap();
        let no_ext = temp.path().join("noextension");
        File::create(&no_ext).unwrap();

        let result = validate_image_path(&no_ext);
        assert!(matches!(result, Err(SdError::ImageFormat(_))));
    }

    #[test]
    fn test_supported_formats() {
        let temp = TempDir::new().unwrap();

        for ext in ["png", "jpg", "jpeg", "gif", "bmp", "webp"] {
            let path = temp.path().join(format!("test.{ext}"));
            File::create(&path).unwrap();
            assert!(
                validate_image_path(&path).is_ok(),
                "Should support .{ext}"
            );
        }
    }

    #[test]
    fn test_path_resolver() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.yaml");
        File::create(&config_path).unwrap();

        let resolver = PathResolver::new(&config_path).unwrap();

        let resolved = resolver.resolve(Path::new("icons/test.png")).unwrap();
        assert!(resolved.starts_with(temp.path()));
    }

    #[test]
    fn test_path_resolver_image() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.yaml");
        File::create(&config_path).unwrap();

        let icons_dir = temp.path().join("icons");
        fs::create_dir(&icons_dir).unwrap();
        let img_path = icons_dir.join("test.png");
        File::create(&img_path).unwrap();

        let resolver = PathResolver::new(&config_path).unwrap();
        let resolved = resolver.resolve_image(Path::new("icons/test.png")).unwrap();

        let canonical = img_path.canonicalize().unwrap_or(img_path);
        assert_eq!(resolved, canonical);
    }
}
