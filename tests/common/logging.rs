//! Log output verification helpers.
//!
//! Provides utilities for parsing and asserting on log output from CLI commands,
//! useful for verifying that appropriate log levels and messages are emitted.

/// Verifier for log output captured from stderr.
///
/// # Example
///
/// ```ignore
/// let result = cli.run(&["-vvv", "set-key", "0", "image.png"]);
/// LogVerifier::from_stderr(&result.stderr)
///     .assert_debug("loading image")
///     .assert_info("key 0 updated")
///     .assert_no_errors();
/// ```
pub struct LogVerifier {
    log_lines: Vec<String>,
}

impl LogVerifier {
    /// Create a verifier from stderr output.
    #[must_use]
    pub fn from_stderr(stderr: &str) -> Self {
        Self {
            log_lines: stderr.lines().map(String::from).collect(),
        }
    }

    /// Create a verifier from a vector of log lines.
    #[must_use]
    pub fn from_lines(lines: Vec<String>) -> Self {
        Self { log_lines: lines }
    }

    /// Check if any log line contains both the level and message.
    fn contains_level_and_message(&self, level: &str, message: &str) -> bool {
        self.log_lines
            .iter()
            .any(|line| line.contains(level) && line.to_lowercase().contains(&message.to_lowercase()))
    }

    /// Get all lines containing the specified level.
    #[must_use]
    pub fn lines_with_level(&self, level: &str) -> Vec<&str> {
        self.log_lines
            .iter()
            .filter(|line| line.contains(level))
            .map(String::as_str)
            .collect()
    }

    /// Assert that a log entry exists with the given level and message.
    ///
    /// # Panics
    ///
    /// Panics if no matching log entry is found.
    #[must_use]
    pub fn assert_contains_level(&self, level: &str, message: &str) -> &Self {
        assert!(
            self.contains_level_and_message(level, message),
            "No {level} log containing \"{message}\" found in:\n{}",
            self.log_lines.join("\n")
        );
        self
    }

    /// Assert that a TRACE level log entry exists with the given message.
    ///
    /// # Panics
    ///
    /// Panics if no matching TRACE entry is found.
    #[must_use]
    pub fn assert_trace(&self, message: &str) -> &Self {
        self.assert_contains_level("TRACE", message)
    }

    /// Assert that a DEBUG level log entry exists with the given message.
    ///
    /// # Panics
    ///
    /// Panics if no matching DEBUG entry is found.
    #[must_use]
    pub fn assert_debug(&self, message: &str) -> &Self {
        self.assert_contains_level("DEBUG", message)
    }

    /// Assert that an INFO level log entry exists with the given message.
    ///
    /// # Panics
    ///
    /// Panics if no matching INFO entry is found.
    #[must_use]
    pub fn assert_info(&self, message: &str) -> &Self {
        self.assert_contains_level("INFO", message)
    }

    /// Assert that a WARN level log entry exists with the given message.
    ///
    /// # Panics
    ///
    /// Panics if no matching WARN entry is found.
    #[must_use]
    pub fn assert_warn(&self, message: &str) -> &Self {
        self.assert_contains_level("WARN", message)
    }

    /// Assert that an ERROR level log entry exists with the given message.
    ///
    /// # Panics
    ///
    /// Panics if no matching ERROR entry is found.
    #[must_use]
    pub fn assert_error(&self, message: &str) -> &Self {
        self.assert_contains_level("ERROR", message)
    }

    /// Assert that no ERROR level log entries exist.
    ///
    /// # Panics
    ///
    /// Panics if any ERROR entries are found.
    #[must_use]
    pub fn assert_no_errors(&self) -> &Self {
        let errors: Vec<_> = self.lines_with_level("ERROR");
        assert!(
            errors.is_empty(),
            "Found unexpected errors:\n{}",
            errors.join("\n")
        );
        self
    }

    /// Assert that no WARN level log entries exist.
    ///
    /// # Panics
    ///
    /// Panics if any WARN entries are found.
    #[must_use]
    pub fn assert_no_warnings(&self) -> &Self {
        let warnings: Vec<_> = self.lines_with_level("WARN");
        assert!(
            warnings.is_empty(),
            "Found unexpected warnings:\n{}",
            warnings.join("\n")
        );
        self
    }

    /// Assert that no WARN or ERROR level log entries exist.
    ///
    /// # Panics
    ///
    /// Panics if any WARN or ERROR entries are found.
    #[must_use]
    pub fn assert_clean(&self) -> &Self {
        self.assert_no_warnings().assert_no_errors()
    }

    /// Get the total number of log lines.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.log_lines.len()
    }

    /// Assert the log has at least N lines.
    ///
    /// # Panics
    ///
    /// Panics if there are fewer than N lines.
    #[must_use]
    pub fn assert_min_lines(&self, min: usize) -> &Self {
        assert!(
            self.log_lines.len() >= min,
            "Expected at least {min} log lines, got {}",
            self.log_lines.len()
        );
        self
    }

    /// Assert the log contains a line matching the regex pattern.
    ///
    /// # Panics
    ///
    /// Panics if no line matches the pattern.
    #[must_use]
    pub fn assert_line_matches(&self, pattern: &str) -> &Self {
        let re = regex::Regex::new(pattern).expect("Invalid regex pattern");
        let found = self.log_lines.iter().any(|line| re.is_match(line));
        assert!(
            found,
            "No log line matches pattern \"{pattern}\"\nLog lines:\n{}",
            self.log_lines.join("\n")
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_verifier_basic() {
        let logs = "2026-01-21 INFO: Starting operation\n\
                    2026-01-21 DEBUG: Loading config\n\
                    2026-01-21 INFO: Operation complete";

        LogVerifier::from_stderr(logs)
            .assert_info("Starting operation")
            .assert_debug("Loading config")
            .assert_no_errors()
            .assert_no_warnings();
    }

    #[test]
    fn test_log_verifier_with_errors() {
        let logs = "2026-01-21 INFO: Starting\n\
                    2026-01-21 ERROR: Something failed";

        let verifier = LogVerifier::from_stderr(logs);
        verifier.assert_error("Something failed");

        // Should have error lines
        assert_eq!(verifier.lines_with_level("ERROR").len(), 1);
    }

    #[test]
    fn test_log_verifier_case_insensitive_message() {
        let logs = "2026-01-21 DEBUG: Loading Configuration File";

        LogVerifier::from_stderr(logs).assert_debug("loading configuration");
    }

    #[test]
    fn test_log_verifier_line_count() {
        let logs = "line1\nline2\nline3";
        let verifier = LogVerifier::from_stderr(logs);

        assert_eq!(verifier.line_count(), 3);
        verifier.assert_min_lines(3);
    }
}
