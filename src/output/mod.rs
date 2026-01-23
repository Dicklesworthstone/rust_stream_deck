//! Output mode abstraction for robot and human output.
#![allow(dead_code)]

use std::path::Path;

use rich_rust::prelude::Console;
use serde::Serialize;

use crate::cli::Cli;
use crate::device::{ButtonEvent, DeviceInfo};
use crate::error::SdError;

pub mod dry_run;
pub mod human;
pub mod robot;

pub use dry_run::{
    BrightnessDryRunDetails, ClearAllDryRunDetails, ClearKeyDryRunDetails, ClearKeysDryRunDetails,
    DeviceContext, DryRunResponse, FillKeyDryRunDetails, ImageSourceInfo, ProcessingInfo,
    SetKeyDryRunDetails, ValidationError,
};
pub use human::HumanOutput;
pub use robot::RobotOutput;

// === Batch Operation Result Types ===

/// Result of a single key operation in a batch.
#[derive(Debug, Clone, Serialize)]
pub struct BatchKeyResult {
    pub key: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BatchKeyResult {
    /// Create a successful result for a set-key operation.
    #[must_use]
    pub fn set_key_success(key: u8, path: &Path) -> Self {
        Self {
            key,
            path: Some(path.display().to_string()),
            color: None,
            ok: true,
            error: None,
        }
    }

    /// Create a failed result for a set-key operation.
    #[must_use]
    pub fn set_key_failure(key: u8, path: &Path, error: &str) -> Self {
        Self {
            key,
            path: Some(path.display().to_string()),
            color: None,
            ok: false,
            error: Some(error.to_string()),
        }
    }

    /// Create a successful result for a clear-key operation.
    #[must_use]
    pub fn clear_success(key: u8) -> Self {
        Self {
            key,
            path: None,
            color: None,
            ok: true,
            error: None,
        }
    }

    /// Create a failed result for a clear-key operation.
    #[must_use]
    pub fn clear_failure(key: u8, error: &str) -> Self {
        Self {
            key,
            path: None,
            color: None,
            ok: false,
            error: Some(error.to_string()),
        }
    }

    /// Create a successful result for a fill-key operation.
    #[must_use]
    pub fn fill_success(key: u8, color: &str) -> Self {
        Self {
            key,
            path: None,
            color: Some(color.to_string()),
            ok: true,
            error: None,
        }
    }

    /// Create a failed result for a fill-key operation.
    #[must_use]
    pub fn fill_failure(key: u8, color: &str, error: &str) -> Self {
        Self {
            key,
            path: None,
            color: Some(color.to_string()),
            ok: false,
            error: Some(error.to_string()),
        }
    }
}

/// Summary statistics for a batch operation.
#[derive(Debug, Clone, Serialize)]
pub struct BatchSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<usize>,
}

// === Validation Result Types ===

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Error - validation fails
    Error,
    /// Warning - validation passes but issue noted
    Warning,
}

/// A single validation issue (error or warning).
#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    /// Field or location where the issue occurred
    pub field: String,
    /// Human-readable message describing the issue
    pub message: String,
    /// Severity level
    pub severity: IssueSeverity,
    /// Optional suggestion for fixing the issue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ValidationIssue {
    /// Create a new error issue.
    #[must_use]
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: IssueSeverity::Error,
            suggestion: None,
        }
    }

    /// Create a new warning issue.
    #[must_use]
    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity: IssueSeverity::Warning,
            suggestion: None,
        }
    }

    /// Add a suggestion to this issue.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Result of validating a configuration file.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    /// Whether the configuration is valid (no errors)
    pub valid: bool,
    /// Path to the config file
    pub config_path: String,
    /// Name of the profile (if specified in config)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_name: Option<String>,
    /// All issues found during validation
    pub issues: Vec<ValidationIssue>,
    /// Summary statistics
    pub summary: ValidationSummary,
}

/// Summary of validation results.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationSummary {
    pub error_count: usize,
    pub warning_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
}

impl ValidationResult {
    /// Create a new validation result for a config file.
    #[must_use]
    pub fn new(config_path: &Path) -> Self {
        Self {
            valid: true,
            config_path: config_path.display().to_string(),
            config_name: None,
            issues: Vec::new(),
            summary: ValidationSummary {
                error_count: 0,
                warning_count: 0,
                key_count: None,
                brightness: None,
            },
        }
    }

    /// Add an error issue.
    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue::error(field, message));
        self.summary.error_count += 1;
        self.valid = false;
    }

    /// Add a warning issue.
    pub fn add_warning(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue::warning(field, message));
        self.summary.warning_count += 1;
    }

    /// Check if validation passed (no errors).
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    /// Get all errors.
    #[must_use]
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect()
    }

    /// Get all warnings.
    #[must_use]
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .collect()
    }
}

impl BatchSummary {
    #[must_use]
    pub fn new(total: usize, success: usize, failed: usize) -> Self {
        Self {
            total,
            success,
            failed,
            skipped: None,
        }
    }

    #[must_use]
    pub fn with_skipped(mut self, skipped: usize) -> Self {
        self.skipped = Some(skipped);
        self
    }

    #[must_use]
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

/// JSON formatting options for robot mode.
#[derive(Debug, Clone, Copy)]
pub enum RobotFormat {
    /// Pretty-printed JSON (default for --robot).
    Json,
    /// Single-line JSON (--format=json-compact).
    JsonCompact,
}

/// Determines how command output is rendered.
#[derive(Debug)]
pub enum OutputMode {
    /// JSON output for AI agents and scripting.
    Robot(RobotFormat),
    /// Styled terminal output for human users.
    Human(Console),
}

impl OutputMode {
    /// Create OutputMode from CLI arguments.
    #[must_use]
    pub fn from_cli(cli: &Cli) -> Self {
        if cli.use_json() {
            let format = if cli.use_compact_json() {
                RobotFormat::JsonCompact
            } else {
                RobotFormat::Json
            };
            Self::Robot(format)
        } else {
            let mut builder = Console::builder().safe_box(cli.no_color);
            if cli.no_color {
                builder = builder.no_color();
            }
            Self::Human(builder.build())
        }
    }

    /// Returns true if output should be JSON.
    #[must_use]
    pub const fn is_robot(&self) -> bool {
        matches!(self, Self::Robot(_))
    }

    /// Convert into the appropriate Output implementation.
    #[must_use]
    pub fn into_output(self) -> Box<dyn Output> {
        match self {
            Self::Robot(format) => Box::new(RobotOutput::new(format)),
            Self::Human(console) => Box::new(HumanOutput::new(console)),
        }
    }
}

/// Trait for all output operations.
///
/// Commands call these methods without knowing the output mode.
pub trait Output {
    // Basic messages
    fn success(&self, message: &str);
    fn error(&self, error: &SdError);
    fn warning(&self, message: &str);
    fn info(&self, message: &str);

    // Device operations
    fn device_list(&self, devices: &[DeviceInfo]);
    fn device_info(&self, info: &DeviceInfo);

    // Button events
    fn button_event(&self, event: &ButtonEvent);
    fn button_states(&self, states: &[bool]);

    // Display operations
    fn brightness_set(&self, level: u8);
    fn key_set(&self, key: u8, image: &Path);
    fn key_cleared(&self, key: u8);
    fn key_filled(&self, key: u8, color: &str);
    fn all_cleared(&self);
    fn all_filled(&self, color: &str);

    // Metadata
    fn version_info(&self, version: &str, git_sha: Option<&str>, build_time: Option<&str>);

    // Visual separators
    fn rule(&self, title: Option<&str>);
    fn newline(&self);

    // Batch operations
    /// Output results of a batch set-keys operation.
    fn batch_set_keys(&self, results: &[BatchKeyResult], summary: &BatchSummary);

    /// Output results of a batch fill-keys operation.
    fn batch_fill_keys(&self, color: &str, results: &[BatchKeyResult], summary: &BatchSummary);

    /// Output results of a batch clear-keys operation.
    fn batch_clear_keys(&self, results: &[BatchKeyResult], summary: &BatchSummary);

    // Validation output
    /// Output results of config validation.
    fn validation_result(&self, result: &ValidationResult);
}
