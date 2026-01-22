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
}
