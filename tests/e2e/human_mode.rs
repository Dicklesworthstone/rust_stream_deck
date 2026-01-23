//! Human-mode end-to-end tests.

use crate::common::assertions::{assert_has_ascii_box, assert_has_box_chars, assert_no_ansi};
use crate::common::cli::CliRunner;
use crate::common::init_test_logging;

#[test]
fn human_version_has_panel_and_is_not_json() {
    init_test_logging();
    let cli = CliRunner::new().with_env("RUST_LOG", "off");
    let result = cli.run(&["version"]);
    result.assert_success();

    let stdout = result.stdout.trim();
    let unicode_box = std::panic::catch_unwind(|| assert_has_box_chars(stdout)).is_ok();
    let ascii_box = std::panic::catch_unwind(|| assert_has_ascii_box(stdout)).is_ok();
    assert!(
        unicode_box || ascii_box,
        "Expected version output to contain a panel/box"
    );

    assert!(
        serde_json::from_str::<serde_json::Value>(stdout).is_err(),
        "Human mode output should not be JSON"
    );
    assert!(stdout.contains("Version"), "Expected Version label in output");
}

#[test]
fn no_color_disables_ansi_and_uses_ascii_box() {
    init_test_logging();
    let cli = CliRunner::new()
        .with_env("RUST_LOG", "off")
        .with_env("NO_COLOR", "1");
    let result = cli.run(&["version"]);
    result.assert_success();

    let stdout = result.stdout.trim();
    assert_no_ansi(stdout);

    // NO_COLOR should force safe_box mode which uses ASCII boxes.
    assert_has_ascii_box(stdout);
}
