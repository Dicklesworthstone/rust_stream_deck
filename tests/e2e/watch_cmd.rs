//! E2E tests for the watch command behavior.

use std::time::{Duration, Instant};

use crate::common::cli::CliRunner;
use crate::common::init_test_logging;

fn parse_json_lines(stdout: &str) {
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let _: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|_| panic!("Invalid JSONL line: {line}"));
    }
}

#[test]
fn watch_timeout_exits_quickly() {
    init_test_logging();
    let cli = CliRunner::new().with_env("RUST_LOG", "off");

    let start = Instant::now();
    let result = cli.run_robot(&["watch", "--timeout=1"]);
    let elapsed = start.elapsed();

    // Expect exit within a few seconds even without a device.
    assert!(
        elapsed < Duration::from_secs(5),
        "watch --timeout=1 should exit quickly (elapsed: {:?})",
        elapsed
    );

    // Either success (device connected) or expected error (no device).
    if result.success() {
        parse_json_lines(result.stdout.trim());
    } else {
        assert!(
            result.stderr.contains("\"error\"") || result.stderr.contains("error"),
            "Expected error output for watch without device"
        );
    }
}

#[test]
fn watch_robot_outputs_jsonl_if_any() {
    init_test_logging();
    let cli = CliRunner::new().with_env("RUST_LOG", "off");

    let result = cli.run_robot(&["watch", "--timeout=1"]);

    if result.success() {
        if !result.stdout.trim().is_empty() {
            parse_json_lines(result.stdout.trim());
        }
    } else if !result.stderr.trim().is_empty() {
        // Error output should be structured JSON from robot mode
        let stderr = result.stderr.trim();
        let parsed = serde_json::from_str::<serde_json::Value>(stderr);
        if let Ok(json) = parsed {
            assert_eq!(json.get("error").and_then(|v| v.as_bool()), Some(true));
        } else {
            assert!(stderr.contains("error"));
        }
    }
}

#[test]
fn watch_once_flag_is_accepted() {
    init_test_logging();
    let cli = CliRunner::new().with_env("RUST_LOG", "off");

    let result = cli.run_robot(&["watch", "--once", "--timeout=1"]);

    // Clap uses exit code 2 for argument parsing errors.
    assert_ne!(result.exit_code, 2, "--once should be a valid flag");
}
