//! Environment variable behavior end-to-end tests.

use crate::common::cli::CliRunner;
use crate::common::init_test_logging;

#[test]
fn sd_format_env_sets_json_output() {
    init_test_logging();
    let cli = CliRunner::new()
        .with_env("RUST_LOG", "off")
        .with_env("SD_FORMAT", "json");
    let result = cli.run(&["version"]);
    result.assert_success();

    let json: serde_json::Value = serde_json::from_str(result.stdout.trim())
        .expect("Expected JSON output with SD_FORMAT=json");
    assert!(json.get("version").is_some());
}

#[test]
fn sd_format_env_sets_compact_json() {
    init_test_logging();
    let cli = CliRunner::new()
        .with_env("RUST_LOG", "off")
        .with_env("SD_FORMAT", "json-compact");
    let result = cli.run(&["version"]);
    result.assert_success();

    let stdout = result.stdout.trim_end();
    let json: serde_json::Value = serde_json::from_str(stdout)
        .expect("Expected JSON output with SD_FORMAT=json-compact");
    assert!(json.get("version").is_some());
    assert_eq!(stdout.lines().count(), 1, "Expected compact JSON single line");
}

#[test]
fn cli_format_flag_overrides_env() {
    init_test_logging();
    let cli = CliRunner::new()
        .with_env("RUST_LOG", "off")
        .with_env("SD_FORMAT", "json");
    let result = cli.run(&["version", "--format=text"]);
    result.assert_success();

    assert!(
        serde_json::from_str::<serde_json::Value>(result.stdout.trim()).is_err(),
        "--format=text should override SD_FORMAT=json"
    );
}
