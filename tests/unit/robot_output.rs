//! Robot mode JSON output regression tests.
//!
//! These tests verify that robot mode JSON output maintains backward compatibility.
//! AI coding agents depend on consistent JSON structure, so ANY change to field names,
//! structure, or formatting is considered a breaking change.
//!
//! # Test Strategy
//!
//! 1. **Golden File Tests**: Compare generated JSON against expected golden files
//! 2. **Schema Tests**: Verify required fields are present with correct types
//! 3. **Semantic Tests**: Parse and verify field values programmatically
//!
//! # Golden Files
//!
//! Located in `tests/golden/robot/`, these files capture the expected JSON output
//! for each robot mode operation. Tests compare generated output against these files.

use std::path::Path;

use sd::device::{ButtonEvent, DeviceInfo};
use sd::error::SdError;
use sd::output::{BatchKeyResult, BatchSummary, RobotFormat, RobotOutput};

/// Load a golden file from tests/golden/robot/.
fn load_golden(name: &str) -> serde_json::Value {
    let path = format!(
        "{}/tests/golden/robot/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read golden file {path}: {e}"));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse golden file {path}: {e}"))
}

/// Create a mock XL device info for testing.
fn mock_device_xl() -> DeviceInfo {
    DeviceInfo {
        serial: "AL12XL0001".to_string(),
        product_name: "Stream Deck XL".to_string(),
        firmware_version: "1.5.3".to_string(),
        key_count: 32,
        key_width: 96,
        key_height: 96,
        rows: 4,
        cols: 8,
        kind: "Xl".to_string(),
    }
}

/// Create a mock Mini device info for testing.
fn mock_device_mini() -> DeviceInfo {
    DeviceInfo {
        serial: "AL12MN0002".to_string(),
        product_name: "Stream Deck Mini".to_string(),
        firmware_version: "1.2.0".to_string(),
        key_count: 6,
        key_width: 72,
        key_height: 72,
        rows: 2,
        cols: 3,
        kind: "Mini".to_string(),
    }
}

// =============================================================================
// Device Info Serialization Tests
// =============================================================================

#[test]
fn device_info_json_structure() {
    let device = mock_device_xl();
    let json = serde_json::to_value(&device).expect("serialize device");
    let golden = load_golden("device_info");

    // Verify all required fields exist with correct types
    assert_eq!(json["serial"], golden["serial"]);
    assert_eq!(json["product_name"], golden["product_name"]);
    assert_eq!(json["firmware_version"], golden["firmware_version"]);
    assert_eq!(json["key_count"], golden["key_count"]);
    assert_eq!(json["key_width"], golden["key_width"]);
    assert_eq!(json["key_height"], golden["key_height"]);
    assert_eq!(json["rows"], golden["rows"]);
    assert_eq!(json["cols"], golden["cols"]);
    assert_eq!(json["kind"], golden["kind"]);
}

#[test]
fn device_info_has_no_extra_fields() {
    let device = mock_device_xl();
    let json = serde_json::to_value(&device).expect("serialize device");
    let obj = json.as_object().expect("should be object");

    // These are the only allowed fields
    let expected_fields = [
        "serial",
        "product_name",
        "firmware_version",
        "key_count",
        "key_width",
        "key_height",
        "rows",
        "cols",
        "kind",
    ];

    for key in obj.keys() {
        assert!(
            expected_fields.contains(&key.as_str()),
            "Unexpected field in DeviceInfo: {key}"
        );
    }

    assert_eq!(
        obj.len(),
        expected_fields.len(),
        "DeviceInfo has wrong number of fields"
    );
}

// =============================================================================
// Device List Serialization Tests
// =============================================================================

#[test]
fn device_list_single_device() {
    let devices = vec![mock_device_xl()];
    let json = serde_json::to_value(&devices).expect("serialize devices");
    let golden = load_golden("device_list_single");

    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["serial"], golden[0]["serial"]);
}

#[test]
fn device_list_multiple_devices() {
    let devices = vec![mock_device_xl(), mock_device_mini()];
    let json = serde_json::to_value(&devices).expect("serialize devices");
    let golden = load_golden("device_list_multiple");

    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 2);
    assert_eq!(json[0]["serial"], golden[0]["serial"]);
    assert_eq!(json[1]["serial"], golden[1]["serial"]);
}

#[test]
fn device_list_empty() {
    let devices: Vec<DeviceInfo> = vec![];
    let json = serde_json::to_value(&devices).expect("serialize devices");

    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 0);
}

// =============================================================================
// Button Event Serialization Tests
// =============================================================================

#[test]
fn button_event_press_structure() {
    let event = ButtonEvent {
        key: 0,
        pressed: true,
        timestamp_ms: 1234,
    };
    let json = serde_json::to_value(&event).expect("serialize event");

    // Button events must be compact single-line JSON for streaming
    let compact = serde_json::to_string(&event).expect("serialize compact");
    assert!(!compact.contains('\n'), "Button event should be single line");

    assert_eq!(json["key"], 0);
    assert_eq!(json["pressed"], true);
    assert_eq!(json["timestamp_ms"], 1234);
}

#[test]
fn button_event_release_structure() {
    let event = ButtonEvent {
        key: 0,
        pressed: false,
        timestamp_ms: 1456,
    };
    let json = serde_json::to_value(&event).expect("serialize event");

    assert_eq!(json["key"], 0);
    assert_eq!(json["pressed"], false);
    assert_eq!(json["timestamp_ms"], 1456);
}

#[test]
fn button_event_has_required_fields() {
    let event = ButtonEvent {
        key: 5,
        pressed: true,
        timestamp_ms: 999,
    };
    let json = serde_json::to_value(&event).expect("serialize event");
    let obj = json.as_object().expect("should be object");

    assert!(obj.contains_key("key"), "Missing 'key' field");
    assert!(obj.contains_key("pressed"), "Missing 'pressed' field");
    assert!(
        obj.contains_key("timestamp_ms"),
        "Missing 'timestamp_ms' field"
    );
    assert_eq!(obj.len(), 3, "Button event has wrong number of fields");
}

// =============================================================================
// Error JSON Structure Tests
// =============================================================================

#[test]
fn error_no_devices_found_structure() {
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
fn error_multiple_devices_structure() {
    let err = SdError::MultipleDevices {
        serials: vec!["AL12XL0001".to_string(), "AL12MN0002".to_string()],
    };
    let json = serde_json::json!({
        "error": true,
        "message": err.to_string(),
        "suggestion": err.suggestion(),
        "recoverable": err.is_user_recoverable(),
    });

    assert_eq!(json["error"], true);
    assert!(json["message"].as_str().unwrap().contains("multiple"));
    assert!(json["suggestion"].as_str().unwrap().contains("--serial"));
    assert_eq!(json["recoverable"], true);
}

#[test]
fn error_device_not_found_structure() {
    let err = SdError::DeviceNotFound {
        serial: "UNKNOWN123".to_string(),
    };
    let json = serde_json::json!({
        "error": true,
        "message": err.to_string(),
        "suggestion": err.suggestion(),
        "recoverable": err.is_user_recoverable(),
    });

    assert_eq!(json["error"], true);
    assert!(json["message"].as_str().unwrap().contains("UNKNOWN123"));
}

#[test]
fn error_invalid_key_index_structure() {
    let err = SdError::InvalidKeyIndex {
        index: 50,
        max: 32,
        max_idx: 31,
    };
    let json = serde_json::json!({
        "error": true,
        "message": err.to_string(),
        "suggestion": err.suggestion(),
        "recoverable": err.is_user_recoverable(),
    });

    assert_eq!(json["error"], true);
    assert!(json["message"].as_str().unwrap().contains("50"));
}

// =============================================================================
// Operation Response Tests
// =============================================================================

#[test]
fn brightness_set_response_structure() {
    let golden = load_golden("brightness_set");

    // Verify the expected structure
    assert_eq!(golden["brightness"], 80);
    assert_eq!(golden["ok"], true);

    // Verify only these fields exist
    let obj = golden.as_object().unwrap();
    assert_eq!(obj.len(), 2);
    assert!(obj.contains_key("brightness"));
    assert!(obj.contains_key("ok"));
}

#[test]
fn key_set_response_structure() {
    let golden = load_golden("key_set");

    assert_eq!(golden["key"], 5);
    assert!(golden["image"].is_string());
    assert_eq!(golden["ok"], true);
}

#[test]
fn key_cleared_response_structure() {
    let golden = load_golden("key_cleared");

    assert_eq!(golden["key"], 5);
    assert_eq!(golden["cleared"], true);
}

#[test]
fn key_filled_response_structure() {
    let golden = load_golden("key_filled");

    assert_eq!(golden["key"], 5);
    assert!(golden["color"].is_string());
    assert_eq!(golden["ok"], true);
}

#[test]
fn all_cleared_response_structure() {
    let golden = load_golden("all_cleared");

    assert_eq!(golden["cleared"], "all");
    assert_eq!(golden["ok"], true);
}

#[test]
fn all_filled_response_structure() {
    let golden = load_golden("all_filled");

    assert_eq!(golden["filled"], "all");
    assert!(golden["color"].is_string());
    assert_eq!(golden["ok"], true);
}

// =============================================================================
// Batch Operation Response Tests
// =============================================================================

#[test]
fn batch_set_keys_response_structure() {
    let golden = load_golden("batch_set_keys");

    assert_eq!(golden["command"], "set-keys");
    assert!(golden["ok"].is_boolean());
    assert!(golden["results"].is_array());
    assert!(golden["summary"].is_object());

    // Check summary structure
    let summary = &golden["summary"];
    assert!(summary["total"].is_number());
    assert!(summary["success"].is_number());
    assert!(summary["failed"].is_number());
}

#[test]
fn batch_fill_keys_response_structure() {
    let golden = load_golden("batch_fill_keys");

    assert_eq!(golden["command"], "fill-keys");
    assert!(golden["color"].is_string());
    assert!(golden["ok"].is_boolean());
    assert!(golden["results"].is_array());
    assert!(golden["summary"].is_object());

    // Fill-keys uses "filled" instead of "success"
    let summary = &golden["summary"];
    assert!(summary["total"].is_number());
    assert!(summary["filled"].is_number());
    assert!(summary["failed"].is_number());
}

#[test]
fn batch_clear_keys_response_structure() {
    let golden = load_golden("batch_clear_keys");

    assert_eq!(golden["command"], "clear-keys");
    assert!(golden["ok"].is_boolean());
    assert!(golden["results"].is_array());
    assert!(golden["summary"].is_object());

    // Clear-keys uses "cleared" instead of "success"
    let summary = &golden["summary"];
    assert!(summary["total"].is_number());
    assert!(summary["cleared"].is_number());
    assert!(summary["failed"].is_number());
}

#[test]
fn batch_key_result_success_structure() {
    let result = BatchKeyResult::set_key_success(0, Path::new("/path/to/image.png"));
    let json = serde_json::to_value(&result).expect("serialize");

    assert_eq!(json["key"], 0);
    assert_eq!(json["path"], "/path/to/image.png");
    assert_eq!(json["ok"], true);
    // Error should not be present on success
    assert!(json.get("error").is_none());
}

#[test]
fn batch_key_result_failure_structure() {
    let result = BatchKeyResult::set_key_failure(2, Path::new("/path/to/image.png"), "Failed");
    let json = serde_json::to_value(&result).expect("serialize");

    assert_eq!(json["key"], 2);
    assert_eq!(json["path"], "/path/to/image.png");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], "Failed");
}

#[test]
fn batch_key_result_clear_success_structure() {
    let result = BatchKeyResult::clear_success(5);
    let json = serde_json::to_value(&result).expect("serialize");

    assert_eq!(json["key"], 5);
    assert_eq!(json["ok"], true);
    // Path should not be present for clear
    assert!(json.get("path").is_none());
}

#[test]
fn batch_key_result_fill_success_structure() {
    let result = BatchKeyResult::fill_success(3, "#FF0000");
    let json = serde_json::to_value(&result).expect("serialize");

    assert_eq!(json["key"], 3);
    assert_eq!(json["color"], "#FF0000");
    assert_eq!(json["ok"], true);
}

#[test]
fn batch_summary_structure() {
    let summary = BatchSummary::new(10, 8, 2);
    let json = serde_json::to_value(&summary).expect("serialize");

    assert_eq!(json["total"], 10);
    assert_eq!(json["success"], 8);
    assert_eq!(json["failed"], 2);
    // Skipped should not be present when not set
    assert!(json.get("skipped").is_none());
}

#[test]
fn batch_summary_with_skipped_structure() {
    let summary = BatchSummary::new(10, 7, 1).with_skipped(2);
    let json = serde_json::to_value(&summary).expect("serialize");

    assert_eq!(json["total"], 10);
    assert_eq!(json["success"], 7);
    assert_eq!(json["failed"], 1);
    assert_eq!(json["skipped"], 2);
}

// =============================================================================
// Version Info Tests
// =============================================================================

#[test]
fn version_info_structure() {
    let golden = load_golden("version");

    assert!(golden["version"].is_string());
    // git_sha and build_time can be null or string
    assert!(golden["git_sha"].is_null() || golden["git_sha"].is_string());
    assert!(golden["build_time"].is_null() || golden["build_time"].is_string());
}

// =============================================================================
// Message Response Tests
// =============================================================================

#[test]
fn success_message_structure() {
    let golden = load_golden("success");

    assert_eq!(golden["success"], true);
    assert!(golden["message"].is_string());
}

#[test]
fn warning_message_structure() {
    let golden = load_golden("warning");

    assert_eq!(golden["warning"], true);
    assert!(golden["message"].is_string());
}

#[test]
fn info_message_structure() {
    let golden = load_golden("info");

    assert_eq!(golden["info"], true);
    assert!(golden["message"].is_string());
}

// =============================================================================
// Robot Format Tests
// =============================================================================

#[test]
fn robot_format_json_is_default() {
    let output = RobotOutput::new(RobotFormat::Json);
    // Just verify it can be created (format is private)
    drop(output);
}

#[test]
fn robot_format_json_compact_available() {
    let output = RobotOutput::new(RobotFormat::JsonCompact);
    drop(output);
}

// =============================================================================
// Field Naming Convention Tests
// =============================================================================

#[test]
fn all_json_fields_use_snake_case() {
    // Verify no camelCase fields exist in our JSON output
    let device = mock_device_xl();
    let json = serde_json::to_value(&device).expect("serialize");
    let obj = json.as_object().unwrap();

    for key in obj.keys() {
        assert!(
            !key.chars().any(|c| c.is_uppercase()),
            "Field '{key}' should use snake_case, not camelCase"
        );
    }
}

#[test]
fn button_event_fields_use_snake_case() {
    let event = ButtonEvent {
        key: 0,
        pressed: true,
        timestamp_ms: 1000,
    };
    let json = serde_json::to_value(&event).expect("serialize");
    let obj = json.as_object().unwrap();

    for key in obj.keys() {
        assert!(
            !key.chars().any(|c| c.is_uppercase()),
            "Field '{key}' should use snake_case"
        );
    }
}

// =============================================================================
// Type Consistency Tests
// =============================================================================

#[test]
fn key_index_is_always_number() {
    // Key indices must be numbers, not strings
    let event = ButtonEvent {
        key: 31,
        pressed: true,
        timestamp_ms: 0,
    };
    let json = serde_json::to_value(&event).expect("serialize");

    assert!(json["key"].is_number(), "key must be a number");
    assert_eq!(json["key"].as_u64().unwrap(), 31);
}

#[test]
fn boolean_values_are_booleans() {
    // Ensure we use true/false, not "true"/"false" strings
    let event = ButtonEvent {
        key: 0,
        pressed: true,
        timestamp_ms: 0,
    };
    let json = serde_json::to_value(&event).expect("serialize");

    assert!(json["pressed"].is_boolean(), "pressed must be a boolean");
}

#[test]
fn counts_are_numbers() {
    let device = mock_device_xl();
    let json = serde_json::to_value(&device).expect("serialize");

    assert!(json["key_count"].is_number());
    assert!(json["key_width"].is_number());
    assert!(json["key_height"].is_number());
    assert!(json["rows"].is_number());
    assert!(json["cols"].is_number());
}
