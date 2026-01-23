# Testing Guide for sd

> Guidelines and infrastructure for testing the Stream Deck CLI.

---

## Test Categories

This project uses three test categories, each with a specific purpose:

| Category | Location | Purpose | Speed |
|----------|----------|---------|-------|
| **Unit Tests** | `src/**/*.rs` | Test individual functions in isolation | Fast |
| **Integration Tests** | `tests/integration/` | Test component interactions | Medium |
| **E2E Tests** | `tests/e2e/` | Test full CLI behavior end-to-end | Slower |

### Unit Tests

Unit tests live alongside the code they test in `src/` modules. They:
- Test individual functions in isolation
- Use mocks for device operations
- Run on every commit
- Target 80%+ code coverage

```rust
// Example: tests inside src/config/selector.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_key() {
        let selector = KeySelector::parse("5").unwrap();
        assert!(matches!(selector, KeySelector::Single(5)));
    }
}
```

### Integration Tests

Integration tests in `tests/integration/` verify that components work together correctly:
- Test data flow between modules
- Use mock devices and fixtures
- Focus on internal API contracts

```rust
// tests/integration/device_operations.rs
use crate::common::mocks::mock_device_xl;

#[test]
fn test_device_info_fields() {
    let device = mock_device_xl();
    assert_eq!(device.key_count, 32);
    assert_eq!(device.rows, 4);
    assert_eq!(device.cols, 8);
}
```

### E2E Tests

End-to-end tests in `tests/e2e/` exercise the complete CLI:
- Test user-facing behavior
- Use the `CliRunner` helper to invoke the actual binary
- Verify stdout/stderr and exit codes

```rust
// tests/e2e/robot_mode.rs
use crate::common::cli::CliRunner;

#[test]
fn robot_list_outputs_json_array() {
    let cli = CliRunner::new();
    let result = cli.run_robot(&["list"]);
    result.assert_success();

    let json = result.json();
    assert!(json.is_array(), "Expected JSON array for device list");
}
```

---

## Test Infrastructure

### Directory Structure

```
tests/
├── common/                  # Shared test utilities
│   ├── mod.rs              # Module exports and test console helpers
│   ├── cli.rs              # CliRunner for E2E tests
│   ├── mocks.rs            # Mock device factories
│   ├── fixtures.rs         # TestImages, TestConfig, TestDir helpers
│   ├── assertions.rs       # Custom assertion macros
│   ├── capture.rs          # Output capture utilities
│   ├── env.rs              # Environment variable helpers
│   └── logging.rs          # Test logging setup
├── fixtures/               # Static test data
│   ├── README.md           # Fixtures documentation
│   └── images/             # Test images
├── e2e/                    # End-to-end CLI tests
│   ├── mod.rs
│   ├── robot_mode.rs       # JSON output tests
│   ├── human_mode.rs       # TTY output tests
│   ├── environment.rs      # Env var behavior tests
│   └── watch_cmd.rs        # Watch command tests
├── integration/            # Component integration tests
│   └── mod.rs
├── batch_operations.rs     # Batch command tests
└── generate_fixtures.rs    # Fixture generation script
```

### CliRunner

The `CliRunner` provides fluent assertions for CLI testing:

```rust
use crate::common::cli::CliRunner;
use std::time::Duration;

#[test]
fn test_version_command() {
    let cli = CliRunner::new();

    cli.run(&["version"])
        .assert_success()
        .assert_stdout_contains("sd ")
        .assert_stderr_is_empty()
        .assert_duration_under(Duration::from_secs(1));
}

#[test]
fn test_robot_mode() {
    let cli = CliRunner::new();

    cli.run_robot(&["list"])
        .assert_success()
        .assert_json_field_exists("/")  // Root is an array
        .assert_stdout_matches(r#"\["#);
}

#[test]
fn test_with_environment() {
    let cli = CliRunner::new()
        .with_env("NO_COLOR", "1")
        .with_timeout(Duration::from_secs(10));

    cli.run(&["list"])
        .assert_success()
        .assert_stdout_not_contains("\x1b[");  // No ANSI codes
}
```

#### CliRunner Methods

| Method | Purpose |
|--------|---------|
| `run(&[args])` | Execute command with arguments |
| `run_robot(&[args])` | Execute with `--robot` flag |
| `run_dry_run(&[args])` | Execute with `--dry-run` flag |
| `run_robot_dry_run(&[args])` | Execute with both flags |
| `with_env(key, value)` | Set environment variable |
| `with_timeout(duration)` | Set execution timeout |
| `with_working_dir(path)` | Set working directory |

#### CliResult Assertions

| Assertion | Purpose |
|-----------|---------|
| `assert_success()` | Exit code is 0 |
| `assert_failure()` | Exit code is non-zero |
| `assert_exit_code(n)` | Exit code matches |
| `assert_stdout_contains(text)` | Stdout contains text |
| `assert_stdout_not_contains(text)` | Stdout excludes text |
| `assert_stdout_matches(regex)` | Stdout matches pattern |
| `assert_stdout_is_empty()` | Stdout is empty |
| `assert_stderr_contains(text)` | Stderr contains text |
| `assert_stderr_is_empty()` | Stderr is empty |
| `assert_json_field(ptr, value)` | JSON field matches |
| `assert_json_field_exists(ptr)` | JSON field exists |
| `assert_json_array_len(ptr, n)` | JSON array has length |
| `assert_duration_under(dur)` | Execution faster than |
| `assert_duration_over(dur)` | Execution slower than |

### Mock Devices

Mock device factories in `tests/common/mocks.rs`:

```rust
use crate::common::mocks::{mock_device_xl, mock_device_mini, mock_multiple_devices};

#[test]
fn test_xl_device() {
    let device = mock_device_xl();
    assert_eq!(device.serial, "XL-TEST-0001");
    assert_eq!(device.key_count, 32);
}

#[test]
fn test_mini_device() {
    let device = mock_device_mini();
    assert_eq!(device.key_count, 6);
}

#[test]
fn test_multiple_devices() {
    let devices = mock_multiple_devices();
    assert_eq!(devices.len(), 3);  // 2 XL + 1 Mini
}
```

### Test Fixtures

Fixtures provide temporary test data with automatic cleanup:

```rust
use crate::common::fixtures::{TestImages, TestConfig, TestDir, fixtures_path};

#[test]
fn test_batch_images() {
    // Create 32 test images in temp directory
    let images = TestImages::create_batch(32, 72);

    // Use in CLI commands
    let cli = CliRunner::new();
    cli.run(&["set-keys", images.path_str(), "--dry-run"])
        .assert_success();

    // Directory cleaned up when `images` goes out of scope
}

#[test]
fn test_numbered_images() {
    // Create images only for specific keys
    let images = TestImages::create_numbered(&[0, 5, 10, 15], 96);
    assert!(images.path().join("key-0.png").exists());
    assert!(!images.path().join("key-1.png").exists());
}

#[test]
fn test_config_file() {
    let config = TestConfig::yaml(r#"
        brightness: 80
        keys:
          "0-7":
            color: "#FF0000"
    "#);

    let cli = CliRunner::new();
    cli.run(&["apply", config.path_str(), "--dry-run"])
        .assert_success();
}

#[test]
fn test_static_fixture() {
    // Reference static fixtures in tests/fixtures/
    let path = fixtures_path("images/valid/72x72.png");
    assert!(path.exists());
}
```

---

## Writing Tests

### Naming Convention

Test functions follow this pattern:

```
test_<function_or_feature>_<scenario>_<expected_outcome>
```

Examples:
```rust
#[test]
fn test_set_brightness_out_of_range_returns_error() { }

#[test]
fn test_parse_key_selector_range_format() { }

#[test]
fn test_robot_mode_error_includes_suggestion() { }
```

### Test Structure (AAA Pattern)

All tests should follow the Arrange-Act-Assert pattern:

```rust
#[test]
fn test_example() {
    // Arrange - Set up test data and dependencies
    let cli = CliRunner::new();
    let images = TestImages::create_batch(8, 72);

    // Act - Execute the code under test
    let result = cli.run(&["set-keys", images.path_str(), "--dry-run"]);

    // Assert - Verify the expected outcome
    result.assert_success();
    result.assert_stdout_contains("8 keys would be set");
}
```

### Testing Error Cases

Always test error conditions and verify helpful messages:

```rust
#[test]
fn test_invalid_key_returns_clear_error() {
    let cli = CliRunner::new();

    // Key 99 is out of range for any device
    let result = cli.run_robot(&["fill-key", "99", "#FF0000"]);

    result.assert_failure();
    let json = result.json();
    assert_eq!(json["error"], true);
    assert!(json["message"].as_str().unwrap().contains("key index"));
    assert!(json["suggestion"].is_some());
}
```

### Testing Robot Mode

Robot mode tests should verify JSON structure:

```rust
#[test]
fn test_list_robot_mode_structure() {
    let cli = CliRunner::new();
    let result = cli.run_robot(&["list"]);

    result.assert_success();

    // Verify it's valid JSON array
    let json = result.json();
    assert!(json.is_array());

    // If devices exist, verify structure
    if let Some(first) = json.as_array().and_then(|arr| arr.first()) {
        result
            .assert_json_field_exists("/0/serial")
            .assert_json_field_exists("/0/product_name")
            .assert_json_field_exists("/0/key_count");
    }
}
```

### Testing Human Mode

Human mode tests verify terminal output formatting:

```rust
#[test]
fn test_human_mode_no_color_env() {
    let cli = CliRunner::new()
        .with_env("NO_COLOR", "1");

    let result = cli.run(&["version"]);

    result.assert_success();
    // Verify no ANSI escape codes
    result.assert_stdout_not_contains("\x1b[");
}
```

---

## Running Tests

### Basic Commands

```bash
# Run all tests
cargo test

# Run only unit tests (in src/)
cargo test --lib

# Run only integration tests
cargo test --test integration

# Run only E2E tests
cargo test --test e2e

# Run a specific test
cargo test test_robot_mode_error

# Run tests matching a pattern
cargo test robot

# Run tests with output shown
cargo test -- --nocapture
```

### With Logging

```bash
# Enable debug logging
RUST_LOG=debug cargo test -- --nocapture

# Enable trace logging for specific module
RUST_LOG=sd::config=trace cargo test config

# Quiet mode (errors only)
RUST_LOG=error cargo test
```

### Test Filtering

```bash
# Run tests in a specific module
cargo test --test e2e robot_mode

# Run ignored tests
cargo test -- --ignored

# List all tests without running
cargo test -- --list
```

### Code Coverage

Install and run tarpaulin for coverage reporting:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html

# Coverage with specific output
cargo tarpaulin --out Lcov --output-dir coverage/

# Coverage for specific tests
cargo tarpaulin --test e2e
```

---

## Best Practices

### Do

- Write tests for both happy path and error cases
- Use descriptive test names that explain the scenario
- Keep tests focused on one behavior
- Use fixtures for test data (not hardcoded strings)
- Clean up resources (handled automatically by `TempDir`)
- Test robot mode JSON structure, not just string content

### Don't

- Test private implementation details
- Rely on test execution order
- Use `sleep()` for timing (use timeouts instead)
- Hardcode paths or system-specific values
- Skip error case testing

### Performance

- E2E tests spawn processes - group related assertions in one test
- Use `--test-threads=1` if tests have shared state
- Prefer integration tests over E2E when testing internal behavior

---

## CI Integration

Tests run automatically on every pull request and push to main:

```yaml
# .github/workflows/test.yml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@nightly
    - run: cargo test --all-targets
```

### Required Checks

Before merging, ensure:

```bash
# All tests pass
cargo test

# No compiler warnings
cargo clippy --all-targets -- -D warnings

# Code is formatted
cargo fmt --check
```

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| "Binary not found" | Run `cargo build` first |
| Flaky timing tests | Increase timeout, use `assert_duration_under` |
| JSON parsing fails | Check `--robot` flag, verify output format |
| Tests hang | Check for infinite loops, add timeout |
| Fixtures not found | Verify `CARGO_MANIFEST_DIR` is set |

### Debugging Failed Tests

```bash
# Run with maximum verbosity
RUST_LOG=trace cargo test test_name -- --nocapture

# Run single test in isolation
cargo test --test e2e -- test_name --test-threads=1

# Print test binary output
cargo test -- --show-output
```
