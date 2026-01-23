//! Directory scanning for batch key operations.
//!
//! Scans directories for images matching a pattern and maps them to Stream Deck key indices.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;
use tracing::{debug, info, instrument, trace, warn};

/// Result of scanning a directory for key images.
#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    /// Successfully matched key mappings (sorted by key index).
    pub mappings: Vec<KeyMapping>,
    /// Files that didn't match the pattern.
    pub unmatched: Vec<PathBuf>,
    /// Files that matched but had errors (path, reason).
    pub invalid: Vec<(PathBuf, String)>,
}

impl ScanResult {
    /// Returns true if any keys were successfully matched.
    pub fn has_mappings(&self) -> bool {
        !self.mappings.is_empty()
    }

    /// Returns the number of successfully matched keys.
    pub fn mapping_count(&self) -> usize {
        self.mappings.len()
    }

    /// Returns true if there were any invalid files.
    pub fn has_invalid(&self) -> bool {
        !self.invalid.is_empty()
    }
}

/// A mapping from a key index to an image file.
#[derive(Debug, Clone, Serialize)]
pub struct KeyMapping {
    /// Key index (0-based).
    pub key: u8,
    /// Path to the image file.
    pub path: PathBuf,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// Errors that can occur during directory scanning.
#[derive(Debug, Error)]
pub enum ScanError {
    /// The specified directory does not exist.
    #[error("directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    /// The specified path is not a directory.
    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),

    /// Failed to read the directory.
    #[error("failed to read directory {0}: {1}")]
    ReadError(PathBuf, #[source] io::Error),

    /// Failed to read a directory entry.
    #[error("failed to read directory entry: {0}")]
    EntryError(#[source] io::Error),

    /// Failed to get file metadata.
    #[error("failed to get file metadata for {0}: {1}")]
    MetadataError(PathBuf, #[source] io::Error),
}

/// Scans a directory for files matching the pattern and maps them to key indices.
///
/// # Arguments
///
/// * `dir` - The directory to scan.
/// * `pattern` - The filename pattern with `{index}` placeholder (e.g., "key-{index}.png").
/// * `key_count` - Maximum number of keys on the device (indices 0 to key_count-1 are valid).
///
/// # Returns
///
/// A `ScanResult` containing:
/// - `mappings`: Files that matched and have valid key indices (sorted by key).
/// - `unmatched`: Files that didn't match the pattern.
/// - `invalid`: Files that matched but had errors (e.g., key index out of range).
///
/// # Example
///
/// ```ignore
/// let result = scan_directory(Path::new("./keys"), "key-{index}.png", 32)?;
/// for mapping in result.mappings {
///     println!("Key {}: {}", mapping.key, mapping.path.display());
/// }
/// ```
#[instrument(skip_all, fields(dir = %dir.display(), pattern = %pattern, key_count = %key_count))]
pub fn scan_directory(dir: &Path, pattern: &str, key_count: u8) -> Result<ScanResult, ScanError> {
    info!("Starting directory scan");

    // Validate directory exists
    if !dir.exists() {
        return Err(ScanError::DirectoryNotFound(dir.to_path_buf()));
    }

    if !dir.is_dir() {
        return Err(ScanError::NotADirectory(dir.to_path_buf()));
    }

    let mut mappings = Vec::new();
    let mut unmatched = Vec::new();
    let mut invalid = Vec::new();
    let mut seen_keys: std::collections::HashMap<u8, PathBuf> = std::collections::HashMap::new();

    // Read directory entries
    let dir_entries =
        std::fs::read_dir(dir).map_err(|e| ScanError::ReadError(dir.to_path_buf(), e))?;

    // Collect and sort entries for deterministic processing
    let mut entries: Vec<_> = dir_entries
        .collect::<Result<Vec<_>, io::Error>>()
        .map_err(ScanError::EntryError)?;

    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            trace!(path = %path.display(), "Skipping directory");
            continue;
        }

        // Try to extract key index from filename
        match extract_key_index(&path, pattern) {
            Some(key) if key < key_count => {
                // Check for duplicate key indices
                if let Some(prev_path) = seen_keys.get(&key) {
                    warn!(
                        key = %key,
                        prev_path = %prev_path.display(),
                        new_path = %path.display(),
                        "Duplicate key index - using later file"
                    );
                    // Remove the previous mapping
                    mappings.retain(|m: &KeyMapping| m.key != key);
                }

                // Get file metadata
                let metadata = std::fs::metadata(&path)
                    .map_err(|e| ScanError::MetadataError(path.clone(), e))?;

                debug!(
                    key = %key,
                    path = %path.display(),
                    size = %metadata.len(),
                    "Matched key file"
                );

                seen_keys.insert(key, path.clone());
                mappings.push(KeyMapping {
                    key,
                    path,
                    size_bytes: metadata.len(),
                });
            }
            Some(key) => {
                warn!(
                    key = %key,
                    max = %(key_count - 1),
                    path = %path.display(),
                    "Key index out of range"
                );
                invalid.push((
                    path,
                    format!("key {} out of range (max {})", key, key_count - 1),
                ));
            }
            None => {
                trace!(path = %path.display(), "File doesn't match pattern");
                unmatched.push(path);
            }
        }
    }

    // Sort mappings by key index for consistent ordering
    mappings.sort_by_key(|m| m.key);

    info!(
        matched = %mappings.len(),
        unmatched = %unmatched.len(),
        invalid = %invalid.len(),
        "Directory scan complete"
    );

    Ok(ScanResult {
        mappings,
        unmatched,
        invalid,
    })
}

/// Extracts the key index from a filename based on the pattern.
///
/// Supports patterns like:
/// - `key-{index}.png` → matches `key-0.png`, `key-12.png`
/// - `key-{index:02d}.png` → matches `key-00.png`, `key-12.png` (zero-padded)
/// - `icon_{index}.jpg` → matches `icon_5.jpg`
fn extract_key_index(path: &Path, pattern: &str) -> Option<u8> {
    let filename = path.file_name()?.to_str()?;

    // Pattern like "key-{index}.png" or "key-{index:02d}.png"
    // Find where {index...} would be and extract the number there

    // Split on "{index" to get prefix
    let parts: Vec<&str> = pattern.split("{index").collect();
    if parts.len() != 2 {
        return None;
    }

    let prefix = parts[0];

    // Get suffix after the closing brace
    let suffix = parts[1].split('}').nth(1)?;

    // Check if filename matches the pattern structure
    if !filename.starts_with(prefix) || !filename.ends_with(suffix) {
        return None;
    }

    // Extract the number between prefix and suffix
    let start = prefix.len();
    let end = filename.len() - suffix.len();
    if start >= end {
        return None;
    }

    let num_str = &filename[start..end];

    // Parse as u8, allowing leading zeros
    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        File::create(&path).unwrap();
        path
    }

    #[test]
    fn test_extract_key_index_basic() {
        let path = PathBuf::from("key-5.png");
        assert_eq!(extract_key_index(&path, "key-{index}.png"), Some(5));
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn test_extract_key_index_zero_padded() {
        let path = PathBuf::from("key-05.png");
        // Pattern uses {index:02d} to indicate the index is zero-padded in filenames
        assert_eq!(extract_key_index(&path, "key-{index:02d}.png"), Some(5));
    }

    #[test]
    fn test_extract_key_index_different_pattern() {
        let path = PathBuf::from("icon_12.jpg");
        assert_eq!(extract_key_index(&path, "icon_{index}.jpg"), Some(12));
    }

    #[test]
    fn test_extract_key_index_no_match() {
        let path = PathBuf::from("other.png");
        assert_eq!(extract_key_index(&path, "key-{index}.png"), None);
    }

    #[test]
    fn test_extract_key_index_wrong_extension() {
        let path = PathBuf::from("key-5.jpg");
        assert_eq!(extract_key_index(&path, "key-{index}.png"), None);
    }

    #[test]
    fn test_scan_directory_basic() {
        let tmp = TempDir::new().unwrap();
        create_test_file(tmp.path(), "key-0.png");
        create_test_file(tmp.path(), "key-1.png");
        create_test_file(tmp.path(), "key-5.png");
        create_test_file(tmp.path(), "other.txt");

        let result = scan_directory(tmp.path(), "key-{index}.png", 32).unwrap();

        assert_eq!(result.mappings.len(), 3);
        assert_eq!(result.mappings[0].key, 0);
        assert_eq!(result.mappings[1].key, 1);
        assert_eq!(result.mappings[2].key, 5);
        assert_eq!(result.unmatched.len(), 1);
        assert!(result.invalid.is_empty());
    }

    #[test]
    fn test_scan_directory_out_of_range() {
        let tmp = TempDir::new().unwrap();
        create_test_file(tmp.path(), "key-0.png");
        create_test_file(tmp.path(), "key-50.png"); // Out of range for 32-key device

        let result = scan_directory(tmp.path(), "key-{index}.png", 32).unwrap();

        assert_eq!(result.mappings.len(), 1);
        assert_eq!(result.invalid.len(), 1);
        assert!(result.invalid[0].1.contains("out of range"));
    }

    #[test]
    fn test_scan_directory_empty() {
        let tmp = TempDir::new().unwrap();

        let result = scan_directory(tmp.path(), "key-{index}.png", 32).unwrap();

        assert!(result.mappings.is_empty());
        assert!(result.unmatched.is_empty());
        assert!(result.invalid.is_empty());
    }

    #[test]
    fn test_scan_directory_not_found() {
        let result = scan_directory(Path::new("/nonexistent/path"), "key-{index}.png", 32);
        assert!(matches!(result, Err(ScanError::DirectoryNotFound(_))));
    }

    #[test]
    fn test_scan_directory_not_a_directory() {
        let tmp = TempDir::new().unwrap();
        let file_path = create_test_file(tmp.path(), "not_a_dir.txt");

        let result = scan_directory(&file_path, "key-{index}.png", 32);
        assert!(matches!(result, Err(ScanError::NotADirectory(_))));
    }

    #[test]
    fn test_scan_directory_skips_subdirs() {
        let tmp = TempDir::new().unwrap();
        create_test_file(tmp.path(), "key-0.png");
        fs::create_dir(tmp.path().join("subdir")).unwrap();

        let result = scan_directory(tmp.path(), "key-{index}.png", 32).unwrap();

        assert_eq!(result.mappings.len(), 1);
    }

    #[test]
    fn test_scan_directory_duplicate_keys() {
        let tmp = TempDir::new().unwrap();
        // Create two files with the same key index
        // Note: which one "wins" depends on filesystem iteration order
        create_test_file(tmp.path(), "key-0.png");
        create_test_file(tmp.path(), "key-00.png"); // Also matches key 0

        // Use a pattern that matches both
        let result = scan_directory(tmp.path(), "key-{index}.png", 32).unwrap();

        // Should only have one mapping for key 0
        assert_eq!(result.mappings.len(), 1);
        assert_eq!(result.mappings[0].key, 0);
    }

    #[test]
    fn test_scan_result_helpers() {
        let result = ScanResult {
            mappings: vec![KeyMapping {
                key: 0,
                path: PathBuf::from("test.png"),
                size_bytes: 100,
            }],
            unmatched: vec![],
            invalid: vec![],
        };

        assert!(result.has_mappings());
        assert_eq!(result.mapping_count(), 1);
        assert!(!result.has_invalid());
    }
}
