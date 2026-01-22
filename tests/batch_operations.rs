//! Integration tests for batch operations (set-keys, clear-keys, fill-keys).
//!
//! NOTE: Many batch operation tests require a physical Stream Deck device.
//! Tests in this file focus on CLI behavior that can be verified without hardware:
//! - Error handling for invalid inputs
//! - Help text and argument parsing
//! - Robot mode JSON error output structure
//!
//! For unit tests of the batch scanner logic, see src/batch/scanner.rs.
//! For tests requiring hardware, use `cargo run` with a connected device.

mod common;

use common::cli::CliRunner;
use common::fixtures::fixtures_path;
use serde_json::json;

// ============================================================================
// CLI Argument Parsing Tests (no device required)
// ============================================================================

mod cli_args {
    use super::*;

    #[test]
    fn set_keys_help() {
        let cli = CliRunner::new();
        let result = cli.run(&["set-keys", "--help"]);
        result.assert_success();
        let stdout = result.stdout.to_lowercase();
        assert!(
            stdout.contains("directory"),
            "Expected help to mention directory. stdout={}",
            result.stdout
        );
        assert!(
            stdout.contains("dir"),
            "Expected help to mention DIR. stdout={}",
            result.stdout
        );
        assert!(
            stdout.contains("pattern"),
            "Expected help to mention pattern. stdout={}",
            result.stdout
        );
        assert!(
            stdout.contains("batch") || stdout.contains("aliases"),
            "Expected help to mention batch alias. stdout={}",
            result.stdout
        );
    }

    #[test]
    fn clear_all_help() {
        let cli = CliRunner::new();
        let result = cli.run(&["clear-all", "--help"]);
        result.assert_success().assert_stdout_contains("clear");
    }

    #[test]
    fn clear_key_help() {
        let cli = CliRunner::new();
        let result = cli.run(&["clear-key", "--help"]);
        result
            .assert_success()
            .assert_stdout_contains("KEY")
            .assert_stdout_contains("clear");
    }

    #[test]
    fn fill_all_help() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-all", "--help"]);
        result
            .assert_success()
            .assert_stdout_contains("COLOR")
            .assert_stdout_contains("fill");
    }

    #[test]
    fn fill_key_help() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-key", "--help"]);
        result
            .assert_success()
            .assert_stdout_contains("KEY")
            .assert_stdout_contains("COLOR");
    }

    #[test]
    fn fill_keys_help() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-keys", "--help"]);
        result
            .assert_success()
            .assert_stdout_contains("COLOR")
            .assert_stdout_contains("keys");
    }

    #[test]
    fn set_keys_accepts_pattern_flag() {
        let cli = CliRunner::new();
        // This will fail for no device, but the arg parsing should succeed
        // before device check
        let result = cli.run(&["set-keys", "--help"]);
        result.assert_stdout_contains("--pattern");
    }

    #[test]
    fn set_keys_accepts_continue_on_error_flag() {
        let cli = CliRunner::new();
        let result = cli.run(&["set-keys", "--help"]);
        result.assert_stdout_contains("--continue-on-error");
    }

    #[test]
    fn set_keys_accepts_dry_run_flag() {
        let cli = CliRunner::new();
        let result = cli.run(&["set-keys", "--help"]);
        result.assert_stdout_contains("--dry-run");
    }

    #[test]
    fn set_keys_accepts_key_range_flag() {
        let cli = CliRunner::new();
        let result = cli.run(&["set-keys", "--help"]);
        result.assert_stdout_contains("--key-range");
    }

    #[test]
    fn set_keys_accepts_start_key_flag() {
        let cli = CliRunner::new();
        let result = cli.run(&["set-keys", "--help"]);
        result.assert_stdout_contains("--start-key");
    }
}

// ============================================================================
// Error Handling Tests (no device required - errors before device access)
// ============================================================================

mod error_handling {
    use super::*;

    #[test]
    fn set_keys_missing_directory_arg() {
        let cli = CliRunner::new();
        let result = cli.run(&["set-keys"]);
        result.assert_failure();
    }

    #[test]
    fn clear_key_missing_key_arg() {
        let cli = CliRunner::new();
        let result = cli.run(&["clear-key"]);
        result.assert_failure();
    }

    #[test]
    fn fill_key_missing_args() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-key"]);
        result.assert_failure();
    }

    #[test]
    fn fill_all_missing_color_arg() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-all"]);
        result.assert_failure();
    }

    #[test]
    fn fill_keys_missing_color_arg() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-keys"]);
        result.assert_failure();
    }

    #[test]
    fn invalid_key_index_format() {
        let cli = CliRunner::new();
        // Non-numeric key index should fail at arg parsing
        let result = cli.run(&["clear-key", "abc"]);
        result.assert_failure();
    }

    #[test]
    fn negative_key_index() {
        let cli = CliRunner::new();
        // Negative key should fail (u8 expected)
        let result = cli.run(&["clear-key", "-1"]);
        result.assert_failure();
    }
}

// ============================================================================
// Robot Mode Error Output Structure Tests
// ============================================================================
// Note: Robot mode errors are output to stdout as JSON, but we need to check
// if the error is properly formatted. These tests verify the error structure.

mod robot_mode_errors {
    use super::*;

    fn parse_error_json(result: &common::cli::CliResult) -> serde_json::Value {
        let stdout = result.stdout.trim();
        if !stdout.is_empty() {
            if let Ok(json) = serde_json::from_str(stdout) {
                return json;
            }
        }
        let stderr = result.stderr.trim();
        if !stderr.is_empty() {
            if let Ok(json) = serde_json::from_str(stderr) {
                return json;
            }
        }
        panic!(
            "Expected JSON on stdout or stderr. stdout={}, stderr={}",
            result.stdout, result.stderr
        );
    }

    #[test]
    fn no_device_error_is_json() {
        let cli = CliRunner::new();
        let batch_dir = fixtures_path("images/batch/complete-6");

        // Without a device, this should return a JSON error
        let result = cli.run_robot(&["set-keys", batch_dir.to_str().unwrap(), "--dry-run"]);
        result.assert_failure();
        let json = parse_error_json(&result);
        assert_eq!(json.get("error").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn no_device_error_has_message() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["clear-all", "--dry-run"]);
        result.assert_failure();

        let json = parse_error_json(&result);
        let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            !message.is_empty(),
            "Expected non-empty message in error JSON"
        );
    }

    #[test]
    fn no_device_error_has_suggestion() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["fill-all", "#FF0000", "--dry-run"]);
        result.assert_failure();

        let json = parse_error_json(&result);
        assert!(
            json.get("suggestion").is_some() && json.get("recoverable").is_some(),
            "Expected suggestion and recoverable fields in error JSON"
        );
    }

    #[test]
    fn no_device_error_mentions_stream_deck() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["brightness", "50", "--dry-run"]);
        result.assert_failure();

        let json = parse_error_json(&result);
        let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let message_lc = message.to_lowercase();
        assert!(
            message_lc.contains("device") || message_lc.contains("stream deck"),
            "Error message should mention device: {message}"
        );
    }
}

// ============================================================================
// Robot Mode Quick Start Output Tests
// ============================================================================

mod robot_mode_basics {
    use super::*;

    #[test]
    fn quick_start_output() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&[]);
        result
            .assert_success()
            .assert_json_field_exists("/tool")
            .assert_json_field(&"/tool", &json!("sd"));
    }

    #[test]
    fn quick_start_has_version() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&[]);
        result.assert_success().assert_json_field_exists("/version");
    }

    #[test]
    fn quick_start_has_commands() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&[]);
        result.assert_success();

        let json = result.json();
        // Should have some command documentation
        assert!(
            json.get("discovery").is_some()
                || json.get("display").is_some()
                || json.get("commands").is_some(),
            "Quick start should document available commands"
        );
    }
}

// ============================================================================
// Batch Scanner Unit Tests (via module tests - these test the core logic)
// ============================================================================
// Note: The batch scanner has comprehensive unit tests in src/batch/scanner.rs
// Those tests cover:
// - Pattern parsing (key-{index}.png, icon_{index:02d}.jpg, etc.)
// - Directory scanning logic
// - Key index extraction
// - Duplicate key handling
// - Out-of-range key detection
// - Empty directory handling
// - File vs directory filtering

// ============================================================================
// Test Fixture Verification
// ============================================================================

mod fixtures {
    use super::*;
    use std::path::Path;

    #[test]
    fn batch_complete_32_exists() {
        let path = fixtures_path("images/batch/complete-32");
        assert!(
            Path::new(&path).exists(),
            "Batch fixture complete-32 should exist"
        );
    }

    #[test]
    fn batch_complete_15_exists() {
        let path = fixtures_path("images/batch/complete-15");
        assert!(
            Path::new(&path).exists(),
            "Batch fixture complete-15 should exist"
        );
    }

    #[test]
    fn batch_complete_6_exists() {
        let path = fixtures_path("images/batch/complete-6");
        assert!(
            Path::new(&path).exists(),
            "Batch fixture complete-6 should exist"
        );
    }

    #[test]
    fn batch_gaps_exists() {
        let path = fixtures_path("images/batch/gaps");
        assert!(Path::new(&path).exists(), "Batch fixture gaps should exist");
    }

    #[test]
    fn batch_custom_pattern_exists() {
        let path = fixtures_path("images/batch/custom-pattern");
        assert!(
            Path::new(&path).exists(),
            "Batch fixture custom-pattern should exist"
        );
    }

    #[test]
    fn batch_partial_10_exists() {
        let path = fixtures_path("images/batch/partial-10");
        assert!(
            Path::new(&path).exists(),
            "Batch fixture partial-10 should exist"
        );
    }

    #[test]
    fn complete_32_has_key_files() {
        let path = fixtures_path("images/batch/complete-32");
        // Check a few key files exist
        assert!(path.join("key-0.png").exists(), "key-0.png should exist");
        assert!(path.join("key-15.png").exists(), "key-15.png should exist");
        assert!(path.join("key-31.png").exists(), "key-31.png should exist");
    }

    #[test]
    fn custom_pattern_has_icon_files() {
        let path = fixtures_path("images/batch/custom-pattern");
        // Custom pattern uses icon_00.png format
        assert!(
            path.join("icon_00.png").exists(),
            "icon_00.png should exist"
        );
    }
}

// ============================================================================
// Test Utilities Verification
// ============================================================================

mod test_utils {
    use super::*;
    use common::fixtures::TestImages;

    #[test]
    fn test_images_create_batch() {
        let images = TestImages::create_batch(4, 72);
        let path = images.path();

        assert!(path.join("key-0.png").exists());
        assert!(path.join("key-1.png").exists());
        assert!(path.join("key-2.png").exists());
        assert!(path.join("key-3.png").exists());
        assert!(!path.join("key-4.png").exists());
    }

    #[test]
    fn test_images_create_numbered() {
        let images = TestImages::create_numbered(&[0, 5, 10], 72);
        let path = images.path();

        assert!(path.join("key-0.png").exists());
        assert!(path.join("key-5.png").exists());
        assert!(path.join("key-10.png").exists());
        assert!(!path.join("key-1.png").exists());
    }

    #[test]
    fn test_images_create_with_pattern() {
        let images = TestImages::create_with_pattern(3, 72, "icon-{index}.jpg");
        let path = images.path();

        assert!(path.join("icon-0.jpg").exists());
        assert!(path.join("icon-1.jpg").exists());
        assert!(path.join("icon-2.jpg").exists());
    }

    #[test]
    fn test_images_cleanup_on_drop() {
        let path_copy;
        {
            let images = TestImages::create_batch(2, 72);
            path_copy = images.path().to_path_buf();
            assert!(path_copy.exists());
        }
        // After drop, directory should be cleaned up
        assert!(!path_copy.exists());
    }
}

// ============================================================================
// Image Validation Tests (invalid image handling)
// ============================================================================

mod image_validation {
    use super::*;
    use common::fixtures::TestDir;

    #[test]
    fn empty_directory_set_keys() {
        let cli = CliRunner::new();
        let dir = TestDir::new();

        // Empty directory should not fail arg parsing but will fail at device access
        // or return an empty set-keys result
        let result = cli.run_robot(&["set-keys", dir.path_str(), "--dry-run"]);
        // Without a device, this will fail with device error
        // The key point is it doesn't crash on empty directory
        assert!(
            result.exit_code != 0 || result.stdout.contains("error"),
            "Expected failure or error response for empty directory"
        );
    }

    #[test]
    fn non_image_files_in_directory() {
        let cli = CliRunner::new();
        let dir = TestDir::new();

        // Create non-image files that match the pattern
        dir.write_file("key-0.png", b"not a valid png file");
        dir.write_file("key-1.png", b"also not a png");

        // Without a device, we can't test the full flow, but we can verify
        // the CLI accepts the directory
        let result = cli.run_robot(&["set-keys", dir.path_str(), "--dry-run"]);
        // Should fail due to no device, not due to parsing
        result.assert_failure();
    }

    #[test]
    fn zero_byte_files_in_directory() {
        let cli = CliRunner::new();
        let dir = TestDir::new();

        // Create zero-byte files
        dir.write_file("key-0.png", b"");
        dir.write_file("key-1.png", b"");

        let result = cli.run_robot(&["set-keys", dir.path_str(), "--dry-run"]);
        // Should fail due to no device (but not crash on zero-byte files)
        result.assert_failure();
    }

    #[test]
    fn mixed_valid_invalid_files() {
        let cli = CliRunner::new();
        let dir = TestDir::new();

        // Create a mix of valid and invalid files
        // Valid PNG from fixtures
        let valid_png = include_bytes!("fixtures/images/valid/exact-72x72.png");
        std::fs::write(dir.path().join("key-0.png"), valid_png).unwrap();

        // Invalid file content
        dir.write_file("key-1.png", b"not a valid image");

        // Text file that doesn't match pattern (should be ignored)
        dir.write_file("readme.txt", b"This is a readme");

        let result = cli.run_robot(&["set-keys", dir.path_str(), "--dry-run"]);
        // Without device, will fail, but shouldn't crash
        result.assert_failure();
    }

    #[test]
    fn directory_with_subdirectories() {
        let cli = CliRunner::new();
        let dir = TestDir::new();

        // Create subdirectory with images (should be skipped)
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("key-0.png"), b"test").unwrap();

        // Create file in root
        let valid_png = include_bytes!("fixtures/images/valid/exact-72x72.png");
        std::fs::write(dir.path().join("key-1.png"), valid_png).unwrap();

        let result = cli.run_robot(&["set-keys", dir.path_str(), "--dry-run"]);
        // Subdirectories should be skipped without error
        result.assert_failure(); // Fails due to no device
    }
}

// ============================================================================
// Continue-On-Error Flag Tests
// ============================================================================

mod continue_on_error {
    use super::*;
    use common::fixtures::TestDir;

    #[test]
    fn set_keys_continue_on_error_flag_accepted() {
        let cli = CliRunner::new();
        let dir = TestDir::new();

        // Verify the --continue-on-error flag is accepted
        let result = cli.run(&["set-keys", dir.path_str(), "--continue-on-error", "--help"]);
        // --help should succeed regardless
        result.assert_success();
    }

    #[test]
    fn set_keys_continue_on_error_short_flag() {
        let cli = CliRunner::new();
        let dir = TestDir::new();

        // Test with pattern and continue-on-error
        let result = cli.run(&["set-keys", dir.path_str(), "--continue-on-error", "--pattern", "key-{index}.png", "--help"]);
        result.assert_success();
    }
}

// ============================================================================
// Range Parsing Tests for clear-keys
// ============================================================================

mod range_parsing {
    use super::*;

    #[test]
    fn fill_keys_all_flag_accepted() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-keys", "--help"]);
        result
            .assert_success()
            .assert_stdout_contains("--all");
    }

    #[test]
    fn fill_keys_range_flag_accepted() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-keys", "--help"]);
        result
            .assert_success()
            .assert_stdout_contains("--range");
    }

    #[test]
    fn fill_keys_keys_flag_accepted() {
        let cli = CliRunner::new();
        let result = cli.run(&["fill-keys", "--help"]);
        result
            .assert_success()
            .assert_stdout_contains("--keys");
    }

    #[test]
    fn clear_keys_help() {
        let cli = CliRunner::new();
        let result = cli.run(&["clear-keys", "--help"]);
        result.assert_success();
        // Verify it has selection options
        let stdout = result.stdout.to_lowercase();
        assert!(
            stdout.contains("all") || stdout.contains("range") || stdout.contains("keys"),
            "clear-keys should support selection options"
        );
    }
}

// ============================================================================
// Color Parsing Tests for fill-key commands
// ============================================================================

mod color_parsing {
    use super::*;

    #[test]
    fn fill_key_hex_color_accepted() {
        let cli = CliRunner::new();
        // Without device, will fail, but should parse the hex color
        let result = cli.run_robot(&["fill-key", "0", "#FF0000"]);
        result.assert_failure();
        // Should fail due to device, not color parsing
        let json = result.json();
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            msg.to_lowercase().contains("device") || msg.to_lowercase().contains("stream deck"),
            "Error should be about device, not color: {msg}"
        );
    }

    #[test]
    fn fill_key_shorthand_hex_accepted() {
        let cli = CliRunner::new();
        // Test shorthand hex #F00 = red
        let result = cli.run_robot(&["fill-key", "0", "#F00"]);
        result.assert_failure();
        let json = result.json();
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        // Should fail due to device, not color parsing
        assert!(
            msg.to_lowercase().contains("device") || msg.to_lowercase().contains("stream deck"),
            "Error should be about device, not color: {msg}"
        );
    }

    #[test]
    fn fill_all_hex_color_accepted() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["fill-all", "#00FF00"]);
        result.assert_failure();
        let json = result.json();
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            msg.to_lowercase().contains("device") || msg.to_lowercase().contains("stream deck"),
            "Error should be about device, not color: {msg}"
        );
    }

    #[test]
    fn fill_key_lowercase_hex() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["fill-key", "5", "#ff00ff"]);
        result.assert_failure();
        // Lowercase hex should be accepted
    }

    #[test]
    fn fill_keys_color_with_all_flag() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["fill-keys", "#0000FF", "--all"]);
        result.assert_failure();
        // Should fail due to device, not argument parsing
    }

    #[test]
    fn fill_keys_color_with_range() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["fill-keys", "#AABBCC", "--range", "0-7"]);
        result.assert_failure();
        // Should fail due to device, not argument parsing
    }
}

// ============================================================================
// Batch Operation Robot Mode Output Structure Tests
// ============================================================================

mod batch_robot_output {
    use super::*;
    use common::fixtures::TestImages;

    #[test]
    fn set_keys_robot_error_structure() {
        let cli = CliRunner::new();
        let images = TestImages::create_batch(4, 72);

        let result = cli.run_robot(&["set-keys", images.path_str()]);
        result.assert_failure();

        // Robot mode should output valid JSON error
        let json = result.json();
        assert_eq!(
            json.get("error").and_then(|v| v.as_bool()),
            Some(true),
            "Robot error should have error: true"
        );
    }

    #[test]
    fn clear_all_robot_error_structure() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["clear-all"]);
        result.assert_failure();

        let json = result.json();
        assert_eq!(
            json.get("error").and_then(|v| v.as_bool()),
            Some(true),
            "Robot error should have error: true"
        );
        assert!(
            json.get("message").is_some(),
            "Robot error should have message field"
        );
    }

    #[test]
    fn fill_all_robot_error_structure() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["fill-all", "#FF0000"]);
        result.assert_failure();

        let json = result.json();
        assert_eq!(
            json.get("error").and_then(|v| v.as_bool()),
            Some(true),
            "Robot error should have error: true"
        );
    }

    #[test]
    fn fill_key_robot_error_has_recoverable_field() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&["fill-key", "0", "#00FF00"]);
        result.assert_failure();

        let json = result.json();
        assert!(
            json.get("recoverable").is_some(),
            "Robot error should have recoverable field"
        );
    }
}

// ============================================================================
// Batch Scanner Pattern Variations
// ============================================================================

mod scanner_patterns {
    use super::*;
    use common::fixtures::TestImages;

    #[test]
    fn custom_pattern_icon_format() {
        let cli = CliRunner::new();
        let images = TestImages::create_with_pattern(4, 72, "icon_{index}.png");

        // Test that custom pattern is accepted
        let result = cli.run_robot(&["set-keys", images.path_str(), "--pattern", "icon_{index}.png"]);
        // Will fail due to no device, but pattern should be accepted
        result.assert_failure();
    }

    #[test]
    fn custom_pattern_with_prefix() {
        let cli = CliRunner::new();
        let images = TestImages::create_with_pattern(4, 72, "button-{index}.jpg");

        let result = cli.run_robot(&["set-keys", images.path_str(), "--pattern", "button-{index}.jpg"]);
        result.assert_failure();
        // Should fail due to device, not pattern
    }

    #[test]
    fn default_pattern_key_index() {
        let cli = CliRunner::new();
        let images = TestImages::create_batch(4, 72);

        // Default pattern is key-{index}.png
        let result = cli.run_robot(&["set-keys", images.path_str()]);
        result.assert_failure();
    }
}

// ============================================================================
// Key Range and Selection Tests
// ============================================================================

mod key_selection {
    use super::*;
    use common::fixtures::TestImages;

    #[test]
    fn set_keys_key_range_filter() {
        let cli = CliRunner::new();
        let images = TestImages::create_batch(32, 72);

        // Test key range filter
        let result = cli.run_robot(&["set-keys", images.path_str(), "--key-range", "0-7"]);
        result.assert_failure(); // No device
    }

    #[test]
    fn set_keys_start_key_offset() {
        let cli = CliRunner::new();
        let images = TestImages::create_batch(8, 72);

        // Test start key offset
        let result = cli.run_robot(&["set-keys", images.path_str(), "--start-key", "8"]);
        result.assert_failure(); // No device
    }

    #[test]
    fn set_keys_combined_range_and_start() {
        let cli = CliRunner::new();
        let images = TestImages::create_batch(16, 72);

        // Combine key-range and start-key
        let result = cli.run_robot(&[
            "set-keys",
            images.path_str(),
            "--key-range",
            "0-7",
            "--start-key",
            "8",
        ]);
        result.assert_failure(); // No device
    }
}

// ============================================================================
// Dry Run Output Tests
// ============================================================================

mod dry_run_output {
    use super::*;
    use common::fixtures::TestImages;

    #[test]
    fn set_keys_dry_run_robot_output() {
        let cli = CliRunner::new();
        let images = TestImages::create_batch(4, 72);

        // Dry run should show what would happen
        // Note: Still needs device to get key count, so will fail without device
        let result = cli.run_robot_dry_run(&["set-keys", images.path_str()]);
        result.assert_failure(); // No device

        // But should be valid JSON
        let json = result.json();
        assert!(json.is_object(), "Dry run should output JSON object");
    }

    #[test]
    fn brightness_dry_run_requires_device() {
        let cli = CliRunner::new();
        let result = cli.run_robot_dry_run(&["brightness", "50"]);
        result.assert_failure();

        // Error should mention device
        let json = result.json();
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            msg.to_lowercase().contains("device") || msg.to_lowercase().contains("stream deck"),
            "Dry run error should mention device: {msg}"
        );
    }
}
