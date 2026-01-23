//! Robot mode JSON output implementation.
#![allow(dead_code)]

use std::path::Path;

use serde::Serialize;
use tracing::{debug, instrument, trace};

use crate::device::{ButtonEvent, DeviceInfo};
use crate::error::SdError;

use super::{BatchKeyResult, BatchSummary, Output, RobotFormat, ValidationResult};

/// JSON output implementation for AI agents and scripting.
///
/// IMPORTANT: This implementation must match existing JSON output.
pub struct RobotOutput {
    format: RobotFormat,
}

impl RobotOutput {
    #[instrument]
    pub fn new(format: RobotFormat) -> Self {
        debug!(?format, "Creating RobotOutput");
        Self { format }
    }

    /// Output any serializable data as JSON to stdout.
    #[instrument(skip(self, data), fields(format = ?self.format))]
    fn output_json<T: Serialize + ?Sized>(&self, data: &T) {
        let json = match self.format {
            RobotFormat::Json => {
                trace!("Serializing as pretty JSON");
                serde_json::to_string_pretty(data).expect("serialization failed")
            }
            RobotFormat::JsonCompact => {
                trace!("Serializing as compact JSON");
                serde_json::to_string(data).expect("serialization failed")
            }
        };
        trace!(json_len = json.len(), "JSON serialized");
        println!("{json}");
    }

    /// Output single-line JSON (for streaming events).
    #[instrument(skip(self, data))]
    fn output_json_line<T: Serialize>(&self, data: &T) {
        let json = serde_json::to_string(data).expect("serialization failed");
        trace!(json_len = json.len(), "JSON line serialized");
        println!("{json}");
    }

    /// Output pretty JSON to stderr (matches existing error behavior).
    #[instrument(skip(self, data))]
    fn output_json_pretty_stderr<T: Serialize>(&self, data: &T) {
        let json = serde_json::to_string_pretty(data).expect("serialization failed");
        trace!(json_len = json.len(), "JSON error serialized");
        eprintln!("{json}");
    }
}

impl Output for RobotOutput {
    #[instrument(skip(self))]
    fn success(&self, message: &str) {
        debug!(message, "Robot: success");
        self.output_json(&serde_json::json!({
            "success": true,
            "message": message
        }));
    }

    #[instrument(skip(self))]
    fn error(&self, error: &SdError) {
        debug!(error = %error, "Robot: error");
        self.output_json_pretty_stderr(&serde_json::json!({
            "error": true,
            "message": error.to_string(),
            "suggestion": error.suggestion(),
            "recoverable": error.is_user_recoverable(),
        }));
    }

    #[instrument(skip(self))]
    fn warning(&self, message: &str) {
        debug!(message, "Robot: warning");
        self.output_json(&serde_json::json!({
            "warning": true,
            "message": message
        }));
    }

    #[instrument(skip(self))]
    fn info(&self, message: &str) {
        debug!(message, "Robot: info");
        self.output_json(&serde_json::json!({
            "info": true,
            "message": message
        }));
    }

    #[instrument(skip(self, devices), fields(count = devices.len()))]
    fn device_list(&self, devices: &[DeviceInfo]) {
        debug!("Robot: device_list");
        self.output_json(devices);
    }

    #[instrument(skip(self, info), fields(serial = %info.serial))]
    fn device_info(&self, info: &DeviceInfo) {
        debug!("Robot: device_info");
        self.output_json(info);
    }

    #[instrument(skip(self, event), fields(key = event.key, pressed = event.pressed))]
    fn button_event(&self, event: &ButtonEvent) {
        trace!("Robot: button_event");
        self.output_json_line(event);
    }

    #[instrument(skip(self, states), fields(count = states.len()))]
    fn button_states(&self, states: &[bool]) {
        debug!("Robot: button_states");
        self.output_json(states);
    }

    #[instrument(skip(self))]
    fn brightness_set(&self, level: u8) {
        debug!(level, "Robot: brightness_set");
        self.output_json(&serde_json::json!({ "brightness": level, "ok": true }));
    }

    #[instrument(skip(self, image))]
    fn key_set(&self, key: u8, image: &Path) {
        debug!(key, image = %image.display(), "Robot: key_set");
        self.output_json(&serde_json::json!({
            "key": key,
            "image": image.display().to_string(),
            "ok": true
        }));
    }

    #[instrument(skip(self))]
    fn key_cleared(&self, key: u8) {
        debug!(key, "Robot: key_cleared");
        self.output_json(&serde_json::json!({ "key": key, "cleared": true }));
    }

    #[instrument(skip(self))]
    fn key_filled(&self, key: u8, color: &str) {
        debug!(key, color, "Robot: key_filled");
        self.output_json(&serde_json::json!({
            "key": key,
            "color": color,
            "ok": true
        }));
    }

    #[instrument(skip(self))]
    fn all_cleared(&self) {
        debug!("Robot: all_cleared");
        self.output_json(&serde_json::json!({ "cleared": "all", "ok": true }));
    }

    #[instrument(skip(self))]
    fn all_filled(&self, color: &str) {
        debug!(color, "Robot: all_filled");
        self.output_json(&serde_json::json!({
            "filled": "all",
            "color": color,
            "ok": true
        }));
    }

    #[instrument(skip(self))]
    fn version_info(&self, version: &str, git_sha: Option<&str>, build_time: Option<&str>) {
        debug!(version, ?git_sha, ?build_time, "Robot: version_info");
        self.output_json(&serde_json::json!({
            "version": version,
            "git_sha": git_sha,
            "build_time": build_time
        }));
    }

    #[instrument(skip(self))]
    fn rule(&self, _title: Option<&str>) {
        trace!("Robot: rule (no-op)");
    }

    #[instrument(skip(self))]
    fn newline(&self) {
        trace!("Robot: newline (no-op)");
    }

    #[instrument(skip(self, results, summary), fields(total = summary.total, success = summary.success))]
    fn batch_set_keys(&self, results: &[BatchKeyResult], summary: &BatchSummary) {
        debug!("Robot: batch_set_keys");
        self.output_json(&serde_json::json!({
            "command": "set-keys",
            "ok": summary.is_success(),
            "results": results,
            "summary": {
                "total": summary.total,
                "success": summary.success,
                "failed": summary.failed,
                "skipped": summary.skipped,
            }
        }));
    }

    #[instrument(skip(self, results, summary), fields(total = summary.total, success = summary.success))]
    fn batch_fill_keys(&self, color: &str, results: &[BatchKeyResult], summary: &BatchSummary) {
        debug!(color, "Robot: batch_fill_keys");
        self.output_json(&serde_json::json!({
            "command": "fill-keys",
            "color": color,
            "ok": summary.is_success(),
            "results": results,
            "summary": {
                "total": summary.total,
                "filled": summary.success,
                "failed": summary.failed,
            }
        }));
    }

    #[instrument(skip(self, results, summary), fields(total = summary.total, success = summary.success))]
    fn batch_clear_keys(&self, results: &[BatchKeyResult], summary: &BatchSummary) {
        debug!("Robot: batch_clear_keys");
        self.output_json(&serde_json::json!({
            "command": "clear-keys",
            "ok": summary.is_success(),
            "results": results,
            "summary": {
                "total": summary.total,
                "cleared": summary.success,
                "failed": summary.failed,
            }
        }));
    }

    #[instrument(skip(self, result), fields(valid = result.valid, errors = result.summary.error_count))]
    fn validation_result(&self, result: &ValidationResult) {
        debug!("Robot: validation_result");
        self.output_json(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{ButtonEvent, DeviceInfo};
    use crate::error::SdError;

    fn mock_device() -> DeviceInfo {
        DeviceInfo {
            serial: "TEST-0001".to_string(),
            product_name: "Stream Deck XL".to_string(),
            firmware_version: "1.0.0".to_string(),
            key_count: 32,
            key_width: 96,
            key_height: 96,
            rows: 4,
            cols: 8,
            kind: "Xl".to_string(),
        }
    }

    #[test]
    fn device_info_is_serializable() {
        let device = mock_device();
        let json = serde_json::to_string_pretty(&device).expect("serialize device");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert_eq!(parsed["serial"], "TEST-0001");
        assert_eq!(parsed["product_name"], "Stream Deck XL");
        assert_eq!(parsed["key_count"], 32);
    }

    #[test]
    fn device_list_is_serializable() {
        let devices = vec![mock_device()];
        let json = serde_json::to_string_pretty(&devices).expect("serialize devices");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 1);
        assert_eq!(parsed[0]["serial"], "TEST-0001");
    }

    #[test]
    fn button_event_is_serializable() {
        let event = ButtonEvent {
            key: 3,
            pressed: true,
            timestamp_ms: 12,
        };
        let json = serde_json::to_string(&event).expect("serialize event");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert_eq!(parsed["key"], 3);
        assert_eq!(parsed["pressed"], true);
        assert_eq!(parsed["timestamp_ms"], 12);
    }

    #[test]
    fn error_json_has_required_fields() {
        let err = SdError::NoDevicesFound;
        let json = serde_json::json!({
            "error": true,
            "message": err.to_string(),
            "suggestion": err.suggestion(),
            "recoverable": err.is_user_recoverable(),
        });
        assert_eq!(json["error"], true);
        assert!(json["message"].is_string());
        assert!(json["suggestion"].is_string());
        assert!(json["recoverable"].is_boolean());
    }

    #[test]
    fn robot_format_selection() {
        let pretty = RobotOutput::new(RobotFormat::Json);
        let compact = RobotOutput::new(RobotFormat::JsonCompact);
        assert!(matches!(pretty.format, RobotFormat::Json));
        assert!(matches!(compact.format, RobotFormat::JsonCompact));
    }
}
