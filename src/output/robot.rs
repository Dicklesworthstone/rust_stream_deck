//! Robot mode JSON output implementation.
#![allow(dead_code)]

use std::path::Path;

use serde::Serialize;
use tracing::{debug, instrument, trace};

use crate::device::{ButtonEvent, DeviceInfo};
use crate::error::SdError;

use super::{Output, RobotFormat};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{ButtonEvent, DeviceInfo};
    use crate::error::SdError;
    use gag::BufferRedirect;
    use std::io::{Read, Write};
    use std::path::Path;

    fn capture_stdout<F: FnOnce()>(func: F) -> String {
        let mut redirect = BufferRedirect::stdout().expect("redirect stdout");
        func();
        let _ = std::io::stdout().flush();
        let mut output = String::new();
        redirect
            .read_to_string(&mut output)
            .expect("read stdout");
        output
    }

    fn capture_stderr<F: FnOnce()>(func: F) -> String {
        let mut redirect = BufferRedirect::stderr().expect("redirect stderr");
        func();
        let _ = std::io::stderr().flush();
        let mut output = String::new();
        redirect
            .read_to_string(&mut output)
            .expect("read stderr");
        output
    }

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
    fn device_list_matches_pretty_json() {
        let output = RobotOutput::new(RobotFormat::Json);
        let devices = vec![mock_device()];
        let rendered = capture_stdout(|| output.device_list(&devices));
        let expected = serde_json::to_string_pretty(&devices).expect("serialize devices");
        assert_eq!(rendered.trim_end(), expected);
    }

    #[test]
    fn button_event_is_single_line_json() {
        let output = RobotOutput::new(RobotFormat::Json);
        let event = ButtonEvent {
            key: 3,
            pressed: true,
            timestamp_ms: 12,
        };
        let rendered = capture_stdout(|| output.button_event(&event));
        let expected = serde_json::to_string(&event).expect("serialize event");
        assert_eq!(rendered.trim_end(), expected);
    }

    #[test]
    fn error_output_matches_shape() {
        let output = RobotOutput::new(RobotFormat::Json);
        let err = SdError::NoDevicesFound;
        let rendered = capture_stderr(|| output.error(&err));
        let expected = serde_json::to_string_pretty(&serde_json::json!({
            "error": true,
            "message": err.to_string(),
            "suggestion": err.suggestion(),
            "recoverable": err.is_user_recoverable(),
        }))
        .expect("serialize error");
        assert_eq!(rendered.trim_end(), expected);
    }

    #[test]
    fn key_set_compact_json() {
        let output = RobotOutput::new(RobotFormat::JsonCompact);
        let image = Path::new("icon.png");
        let rendered = capture_stdout(|| output.key_set(2, image));
        let expected = serde_json::to_string(&serde_json::json!({
            "key": 2,
            "image": "icon.png",
            "ok": true
        }))
        .expect("serialize key_set");
        assert_eq!(rendered.trim_end(), expected);
    }
}
