//! CLI test runner with fluent assertions.
//!
//! Provides infrastructure for executing the `sd` binary and verifying output,
//! exit codes, timing, and JSON responses in robot mode.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Configuration for CLI test runs.
#[derive(Debug, Clone)]
pub struct CliConfig {
    /// Maximum time to wait for command completion.
    pub timeout: Duration,
    /// Environment variables to set for the command.
    pub env_vars: HashMap<String, String>,
    /// Working directory for command execution.
    pub working_dir: Option<PathBuf>,
    /// Standard input to provide to the command.
    pub stdin: Option<String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            env_vars: HashMap::new(),
            working_dir: None,
            stdin: None,
        }
    }
}

/// Main test runner for the `sd` CLI binary.
///
/// # Example
///
/// ```ignore
/// let cli = CliRunner::new();
/// cli.run(&["list", "--robot"])
///    .assert_success()
///    .assert_stdout_contains("serial");
/// ```
pub struct CliRunner {
    binary_path: PathBuf,
    config: CliConfig,
}

impl Default for CliRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CliRunner {
    /// Create a new CLI runner pointing to the compiled `sd` binary.
    #[must_use]
    pub fn new() -> Self {
        // Use CARGO_BIN_EXE_sd which is set by cargo test for binary crates
        let binary = env!("CARGO_BIN_EXE_sd");
        Self {
            binary_path: PathBuf::from(binary),
            config: CliConfig::default(),
        }
    }

    /// Set the timeout for command execution.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Add an environment variable for command execution.
    #[must_use]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.config
            .env_vars
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Set the working directory for command execution.
    #[must_use]
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.config.working_dir = Some(dir);
        self
    }

    /// Set standard input for the command.
    #[must_use]
    pub fn with_stdin(mut self, stdin: &str) -> Self {
        self.config.stdin = Some(stdin.to_string());
        self
    }

    /// Execute the command with the given arguments.
    ///
    /// # Panics
    ///
    /// Panics if the command fails to execute.
    #[must_use]
    pub fn run(&self, args: &[&str]) -> CliResult {
        let start = Instant::now();

        let mut cmd = Command::new(&self.binary_path);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

        // Apply environment variables
        for (key, value) in &self.config.env_vars {
            cmd.env(key, value);
        }

        // Apply working directory
        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output().expect("Failed to execute command");
        let duration = start.elapsed();

        CliResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration,
            args: args.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Execute with `--robot` flag for JSON output.
    #[must_use]
    pub fn run_robot(&self, args: &[&str]) -> CliResult {
        let mut full_args = vec!["--robot"];
        full_args.extend(args);
        self.run(&full_args)
    }

    /// Execute with `--dry-run` flag.
    #[must_use]
    pub fn run_dry_run(&self, args: &[&str]) -> CliResult {
        let mut full_args = args.to_vec();
        full_args.push("--dry-run");
        self.run(&full_args)
    }

    /// Execute with both `--robot` and `--dry-run` flags.
    #[must_use]
    pub fn run_robot_dry_run(&self, args: &[&str]) -> CliResult {
        let mut full_args = vec!["--robot"];
        full_args.extend(args);
        full_args.push("--dry-run");
        self.run(&full_args)
    }
}

/// Captured output from CLI execution with fluent assertions.
#[derive(Debug, Clone)]
pub struct CliResult {
    /// Standard output captured from the command.
    pub stdout: String,
    /// Standard error captured from the command.
    pub stderr: String,
    /// Exit code from the command.
    pub exit_code: i32,
    /// Time taken to execute the command.
    pub duration: Duration,
    /// Arguments passed to the command.
    pub args: Vec<String>,
}

impl CliResult {
    /// Check if the command succeeded (exit code 0).
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    // === Fluent Assertions (all return &Self for chaining) ===

    /// Assert the command succeeded.
    ///
    /// # Panics
    ///
    /// Panics if the command did not exit with code 0.
    #[must_use]
    pub fn assert_success(&self) -> &Self {
        assert!(
            self.success(),
            "Command {:?} failed with exit code {}: {}",
            self.args,
            self.exit_code,
            self.stderr
        );
        self
    }

    /// Assert the command failed.
    ///
    /// # Panics
    ///
    /// Panics if the command exited with code 0.
    #[must_use]
    pub fn assert_failure(&self) -> &Self {
        assert!(
            !self.success(),
            "Command {:?} unexpectedly succeeded",
            self.args
        );
        self
    }

    /// Assert a specific exit code.
    ///
    /// # Panics
    ///
    /// Panics if the exit code doesn't match.
    #[must_use]
    pub fn assert_exit_code(&self, expected: i32) -> &Self {
        assert_eq!(
            self.exit_code, expected,
            "Expected exit code {expected}, got {} for {:?}",
            self.exit_code, self.args
        );
        self
    }

    // === Stdout Assertions ===

    /// Assert stdout contains the given text.
    ///
    /// # Panics
    ///
    /// Panics if stdout doesn't contain the text.
    #[must_use]
    pub fn assert_stdout_contains(&self, text: &str) -> &Self {
        assert!(
            self.stdout.contains(text),
            "stdout does not contain \"{text}\"\nActual stdout:\n{}",
            self.stdout
        );
        self
    }

    /// Assert stdout does not contain the given text.
    ///
    /// # Panics
    ///
    /// Panics if stdout contains the text.
    #[must_use]
    pub fn assert_stdout_not_contains(&self, text: &str) -> &Self {
        assert!(
            !self.stdout.contains(text),
            "stdout unexpectedly contains \"{text}\""
        );
        self
    }

    /// Assert stdout matches a regex pattern.
    ///
    /// # Panics
    ///
    /// Panics if stdout doesn't match the pattern.
    #[must_use]
    pub fn assert_stdout_matches(&self, pattern: &str) -> &Self {
        let re = regex::Regex::new(pattern).expect("Invalid regex pattern");
        assert!(
            re.is_match(&self.stdout),
            "stdout does not match pattern \"{pattern}\"\nActual stdout:\n{}",
            self.stdout
        );
        self
    }

    /// Assert stdout is empty.
    ///
    /// # Panics
    ///
    /// Panics if stdout is not empty.
    #[must_use]
    pub fn assert_stdout_is_empty(&self) -> &Self {
        assert!(
            self.stdout.is_empty(),
            "Expected empty stdout, got: {}",
            self.stdout
        );
        self
    }

    // === Stderr Assertions ===

    /// Assert stderr contains the given text.
    ///
    /// # Panics
    ///
    /// Panics if stderr doesn't contain the text.
    #[must_use]
    pub fn assert_stderr_contains(&self, text: &str) -> &Self {
        assert!(
            self.stderr.contains(text),
            "stderr does not contain \"{text}\"\nActual stderr:\n{}",
            self.stderr
        );
        self
    }

    /// Assert stderr is empty (or contains only whitespace).
    ///
    /// # Panics
    ///
    /// Panics if stderr is not empty.
    #[must_use]
    pub fn assert_stderr_is_empty(&self) -> &Self {
        assert!(
            self.stderr.trim().is_empty(),
            "Expected empty stderr, got: {}",
            self.stderr
        );
        self
    }

    // === JSON Assertions (for robot mode) ===

    /// Parse stdout as JSON.
    ///
    /// # Panics
    ///
    /// Panics if stdout is not valid JSON.
    #[must_use]
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|_| panic!("Failed to parse JSON from stdout:\n{}", self.stdout))
    }

    /// Assert a JSON field matches an expected value using JSON pointer syntax.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist or doesn't match.
    #[must_use]
    pub fn assert_json_field(&self, json_pointer: &str, expected: &Value) -> &Self {
        let json = self.json();
        let actual = json.pointer(json_pointer).unwrap_or_else(|| {
            panic!(
                "JSON path {json_pointer} not found in:\n{}",
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )
        });
        assert_eq!(actual, expected, "JSON field {json_pointer} mismatch");
        self
    }

    /// Assert a JSON field exists at the given pointer path.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist.
    #[must_use]
    pub fn assert_json_field_exists(&self, json_pointer: &str) -> &Self {
        let json = self.json();
        assert!(
            json.pointer(json_pointer).is_some(),
            "JSON path {json_pointer} not found"
        );
        self
    }

    /// Assert a JSON array has the expected length.
    ///
    /// # Panics
    ///
    /// Panics if the field is not an array or has wrong length.
    #[must_use]
    pub fn assert_json_array_len(&self, json_pointer: &str, expected_len: usize) -> &Self {
        let json = self.json();
        let arr = json
            .pointer(json_pointer)
            .unwrap_or_else(|| panic!("JSON path {json_pointer} not found"))
            .as_array()
            .unwrap_or_else(|| panic!("JSON path {json_pointer} is not an array"));
        assert_eq!(
            arr.len(),
            expected_len,
            "Array at {json_pointer} has {} elements, expected {expected_len}",
            arr.len()
        );
        self
    }

    // === Timing Assertions ===

    /// Assert the command completed within the given duration.
    ///
    /// # Panics
    ///
    /// Panics if the command took longer.
    #[must_use]
    pub fn assert_duration_under(&self, max: Duration) -> &Self {
        assert!(
            self.duration < max,
            "Command took {:?}, expected under {max:?}",
            self.duration
        );
        self
    }

    /// Assert the command took at least the given duration.
    ///
    /// # Panics
    ///
    /// Panics if the command completed too quickly.
    #[must_use]
    pub fn assert_duration_over(&self, min: Duration) -> &Self {
        assert!(
            self.duration > min,
            "Command took {:?}, expected over {min:?}",
            self.duration
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_runner_version() {
        let cli = CliRunner::new();
        cli.run(&["version"]).assert_success();
    }

    #[test]
    fn test_cli_runner_robot_mode() {
        let cli = CliRunner::new();
        let result = cli.run_robot(&[]);

        result
            .assert_success()
            .assert_json_field_exists("/tool")
            .assert_json_field(&"/tool", &Value::String("sd".to_string()));
    }

    #[test]
    fn test_cli_runner_invalid_command() {
        let cli = CliRunner::new();
        cli.run(&["nonexistent-command"]).assert_failure();
    }
}
