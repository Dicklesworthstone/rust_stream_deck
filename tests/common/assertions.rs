//! Custom assertion helpers for tests.
#![allow(dead_code)]

use tracing::error;

#[must_use]
pub fn assert_json_has_fields(json_str: &str, fields: &[&str]) -> serde_json::Value {
    let value: serde_json::Value =
        serde_json::from_str(json_str).expect("invalid JSON payload");
    for field in fields {
        if !value.get(field).is_some() {
            error!(field, json = %value, "Missing expected JSON field");
            panic!("Missing JSON field: {field}");
        }
    }
    value
}

pub fn assert_json_array_len(json_str: &str, len: usize) {
    let value: serde_json::Value =
        serde_json::from_str(json_str).expect("invalid JSON payload");
    let array = value.as_array().expect("JSON value is not an array");
    if array.len() != len {
        error!(expected = len, actual = array.len(), "Unexpected JSON array length");
        panic!("Expected array length {len}, got {}", array.len());
    }
}

pub fn assert_no_ansi(output: &str) {
    if output.contains("\u{1b}[") {
        error!("ANSI escape sequence detected");
        panic!("Expected no ANSI escape sequences");
    }
}

pub fn assert_has_ansi(output: &str) {
    if !output.contains("\u{1b}[") {
        error!("No ANSI escape sequence detected");
        panic!("Expected ANSI escape sequences");
    }
}

pub fn assert_contains_all(output: &str, expected: &[&str]) {
    for needle in expected {
        if !output.contains(needle) {
            error!(needle, "Missing expected substring");
            panic!("Missing expected substring: {needle}");
        }
    }
}

pub fn assert_has_box_chars(output: &str) {
    let box_chars = [
        "\u{2500}", // ─
        "\u{2502}", // │
        "\u{250c}", // ┌
        "\u{2510}", // ┐
        "\u{2514}", // └
        "\u{2518}", // ┘
        "\u{256d}", // ╭
        "\u{256e}", // ╮
        "\u{2570}", // ╰
        "\u{256f}", // ╯
    ];
    if !box_chars.iter().any(|ch| output.contains(ch)) {
        error!("No box drawing characters found");
        panic!("Expected box drawing characters");
    }
}

pub fn assert_has_ascii_box(output: &str) {
    let has_ascii = output.contains('+') && output.contains('-') && output.contains('|');
    if !has_ascii {
        error!("No ASCII box characters found");
        panic!("Expected ASCII box characters (+ - |)");
    }
}
