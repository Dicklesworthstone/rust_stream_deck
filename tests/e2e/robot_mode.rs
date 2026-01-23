//! Robot-mode end-to-end tests.

use serde_json::Value;

use crate::common::cli::CliRunner;
use crate::common::init_test_logging;

fn parse_json(text: &str) -> Value {
    serde_json::from_str(text)
        .unwrap_or_else(|_| panic!("Failed to parse JSON:\n{text}"))
}

#[test]
fn robot_quick_start_outputs_json() {
    init_test_logging();
    let cli = CliRunner::new();
    let result = cli.run(&["--robot"]);
    result.assert_success();

    let json = parse_json(result.stdout.trim());
    assert_eq!(json.get("tool").and_then(|v| v.as_str()), Some("sd"));
    assert!(json.get("discovery").is_some());
    assert!(json.get("output_modes").is_some());
}

#[test]
fn robot_list_outputs_json_array() {
    init_test_logging();
    let cli = CliRunner::new();
    let result = cli.run_robot(&["list"]);
    result.assert_success();

    let json = parse_json(result.stdout.trim());
    assert!(json.is_array(), "Expected JSON array for device list");
}

#[test]
fn robot_format_flag_outputs_json() {
    init_test_logging();
    let cli = CliRunner::new();
    let result = cli.run(&["version", "--format=json"]);
    result.assert_success();

    let json = parse_json(result.stdout.trim());
    assert!(json.get("version").is_some());
}

#[test]
fn robot_error_includes_suggestion_or_device_info() {
    init_test_logging();
    let cli = CliRunner::new().with_env("RUST_LOG", "off");
    let result = cli.run_robot(&["info"]);

    if result.success() {
        let json = parse_json(result.stdout.trim());
        assert!(json.get("serial").is_some());
        return;
    }

    // In error cases, robot output is emitted to stderr.
    let stderr = result.stderr.trim();
    assert!(!stderr.is_empty(), "Expected robot error JSON in stderr");
    let json = parse_json(stderr);
    assert_eq!(json.get("error").and_then(|v| v.as_bool()), Some(true));
    assert!(json.get("message").is_some());
    assert!(json.get("suggestion").is_some());
}
