//! Integration tests for configuration parsing.
//!
//! Tests verify key selector parsing, resolution, and configuration
//! file handling across different device models.

use sd::config::KeySelector;
use sd::device::DeviceInfo;
use sd::error::SdError;

/// Create a test DeviceInfo for XL (32 keys, 8x4).
fn xl_device() -> DeviceInfo {
    DeviceInfo {
        serial: "TEST-XL".to_string(),
        product_name: "Stream Deck XL".to_string(),
        firmware_version: "1.0.0".to_string(),
        key_count: 32,
        key_width: 96,
        key_height: 96,
        rows: 4,
        cols: 8,
        kind: "xl".to_string(),
    }
}

/// Create a test DeviceInfo for Mini (6 keys, 3x2).
fn mini_device() -> DeviceInfo {
    DeviceInfo {
        serial: "TEST-MINI".to_string(),
        product_name: "Stream Deck Mini".to_string(),
        firmware_version: "1.0.0".to_string(),
        key_count: 6,
        key_width: 72,
        key_height: 72,
        rows: 2,
        cols: 3,
        kind: "mini".to_string(),
    }
}

/// Create a test DeviceInfo for MK2 (15 keys, 5x3).
fn mk2_device() -> DeviceInfo {
    DeviceInfo {
        serial: "TEST-MK2".to_string(),
        product_name: "Stream Deck MK.2".to_string(),
        firmware_version: "1.0.0".to_string(),
        key_count: 15,
        key_width: 72,
        key_height: 72,
        rows: 3,
        cols: 5,
        kind: "mk2".to_string(),
    }
}

// ===== KeySelector Parsing Tests =====

#[test]
fn test_selector_parse_single_key() {
    assert_eq!(KeySelector::parse("0").unwrap(), KeySelector::Single(0));
    assert_eq!(KeySelector::parse("15").unwrap(), KeySelector::Single(15));
    assert_eq!(KeySelector::parse("31").unwrap(), KeySelector::Single(31));

    // Whitespace handling
    assert_eq!(KeySelector::parse(" 5 ").unwrap(), KeySelector::Single(5));
    assert_eq!(
        KeySelector::parse("\t10\n").unwrap(),
        KeySelector::Single(10)
    );
}

#[test]
fn test_selector_parse_range() {
    assert_eq!(
        KeySelector::parse("0-7").unwrap(),
        KeySelector::Range { start: 0, end: 7 }
    );
    assert_eq!(
        KeySelector::parse("8-15").unwrap(),
        KeySelector::Range { start: 8, end: 15 }
    );
    assert_eq!(
        KeySelector::parse("24-31").unwrap(),
        KeySelector::Range { start: 24, end: 31 }
    );

    // Single-element range
    assert_eq!(
        KeySelector::parse("5-5").unwrap(),
        KeySelector::Range { start: 5, end: 5 }
    );
}

#[test]
fn test_selector_parse_row() {
    assert_eq!(KeySelector::parse("row-0").unwrap(), KeySelector::Row(0));
    assert_eq!(KeySelector::parse("row-1").unwrap(), KeySelector::Row(1));
    assert_eq!(KeySelector::parse("row-3").unwrap(), KeySelector::Row(3));
}

#[test]
fn test_selector_parse_column() {
    assert_eq!(KeySelector::parse("col-0").unwrap(), KeySelector::Column(0));
    assert_eq!(KeySelector::parse("col-7").unwrap(), KeySelector::Column(7));
}

#[test]
fn test_selector_parse_default() {
    assert_eq!(KeySelector::parse("default").unwrap(), KeySelector::Default);
}

#[test]
fn test_selector_parse_errors() {
    // Empty string
    assert!(KeySelector::parse("").is_err());

    // Invalid format
    assert!(KeySelector::parse("foo").is_err());
    assert!(KeySelector::parse("key-5").is_err());

    // Invalid range (start > end)
    assert!(KeySelector::parse("15-8").is_err());

    // Invalid row/col numbers
    assert!(KeySelector::parse("row-").is_err());
    assert!(KeySelector::parse("col-").is_err());
    assert!(KeySelector::parse("row-abc").is_err());
    assert!(KeySelector::parse("col-xyz").is_err());
}

// ===== KeySelector Resolution Tests =====

#[test]
fn test_selector_resolve_single_xl() {
    let device = xl_device();

    assert_eq!(KeySelector::Single(0).resolve(&device).unwrap(), vec![0]);
    assert_eq!(KeySelector::Single(15).resolve(&device).unwrap(), vec![15]);
    assert_eq!(KeySelector::Single(31).resolve(&device).unwrap(), vec![31]);
}

#[test]
fn test_selector_resolve_single_mini() {
    let device = mini_device();

    assert_eq!(KeySelector::Single(0).resolve(&device).unwrap(), vec![0]);
    assert_eq!(KeySelector::Single(5).resolve(&device).unwrap(), vec![5]);

    // Out of range for Mini
    assert!(KeySelector::Single(6).resolve(&device).is_err());
}

#[test]
fn test_selector_resolve_range_xl() {
    let device = xl_device();

    // First row
    assert_eq!(
        KeySelector::Range { start: 0, end: 7 }
            .resolve(&device)
            .unwrap(),
        vec![0, 1, 2, 3, 4, 5, 6, 7]
    );

    // Second row
    assert_eq!(
        KeySelector::Range { start: 8, end: 15 }
            .resolve(&device)
            .unwrap(),
        vec![8, 9, 10, 11, 12, 13, 14, 15]
    );

    // All keys
    let all_keys: Vec<u8> = (0..32).collect();
    assert_eq!(
        KeySelector::Range { start: 0, end: 31 }
            .resolve(&device)
            .unwrap(),
        all_keys
    );
}

#[test]
fn test_selector_resolve_row_xl() {
    let device = xl_device();

    // XL is 8 columns x 4 rows
    // Row 0: keys 0-7
    assert_eq!(
        KeySelector::Row(0).resolve(&device).unwrap(),
        vec![0, 1, 2, 3, 4, 5, 6, 7]
    );

    // Row 1: keys 8-15
    assert_eq!(
        KeySelector::Row(1).resolve(&device).unwrap(),
        vec![8, 9, 10, 11, 12, 13, 14, 15]
    );

    // Row 3: keys 24-31
    assert_eq!(
        KeySelector::Row(3).resolve(&device).unwrap(),
        vec![24, 25, 26, 27, 28, 29, 30, 31]
    );

    // Row 4 doesn't exist
    assert!(KeySelector::Row(4).resolve(&device).is_err());
}

#[test]
fn test_selector_resolve_row_mini() {
    let device = mini_device();

    // Mini is 3 columns x 2 rows
    // Row 0: keys 0-2
    assert_eq!(KeySelector::Row(0).resolve(&device).unwrap(), vec![0, 1, 2]);

    // Row 1: keys 3-5
    assert_eq!(KeySelector::Row(1).resolve(&device).unwrap(), vec![3, 4, 5]);

    // Row 2 doesn't exist
    assert!(KeySelector::Row(2).resolve(&device).is_err());
}

#[test]
fn test_selector_resolve_column_xl() {
    let device = xl_device();

    // XL is 8 columns x 4 rows
    // Column 0: keys 0, 8, 16, 24
    assert_eq!(
        KeySelector::Column(0).resolve(&device).unwrap(),
        vec![0, 8, 16, 24]
    );

    // Column 7: keys 7, 15, 23, 31
    assert_eq!(
        KeySelector::Column(7).resolve(&device).unwrap(),
        vec![7, 15, 23, 31]
    );

    // Column 8 doesn't exist
    assert!(KeySelector::Column(8).resolve(&device).is_err());
}

#[test]
fn test_selector_resolve_column_mini() {
    let device = mini_device();

    // Mini is 3 columns x 2 rows
    // Column 0: keys 0, 3
    assert_eq!(KeySelector::Column(0).resolve(&device).unwrap(), vec![0, 3]);

    // Column 2: keys 2, 5
    assert_eq!(KeySelector::Column(2).resolve(&device).unwrap(), vec![2, 5]);

    // Column 3 doesn't exist
    assert!(KeySelector::Column(3).resolve(&device).is_err());
}

#[test]
fn test_selector_resolve_mk2() {
    let device = mk2_device();

    // MK2 is 5 columns x 3 rows (15 keys)
    // Row 0: keys 0-4
    assert_eq!(
        KeySelector::Row(0).resolve(&device).unwrap(),
        vec![0, 1, 2, 3, 4]
    );

    // Column 0: keys 0, 5, 10
    assert_eq!(
        KeySelector::Column(0).resolve(&device).unwrap(),
        vec![0, 5, 10]
    );

    // All keys
    let all_keys: Vec<u8> = (0..15).collect();
    assert_eq!(
        KeySelector::Range { start: 0, end: 14 }
            .resolve(&device)
            .unwrap(),
        all_keys
    );
}

#[test]
fn test_selector_resolve_default() {
    let device = xl_device();

    // Default selector returns empty (handled specially during config resolution)
    let empty: Vec<u8> = vec![];
    assert_eq!(KeySelector::Default.resolve(&device).unwrap(), empty);
}

// ===== Priority Tests =====

#[test]
fn test_selector_priority_ordering() {
    // Single has highest priority (0)
    assert_eq!(KeySelector::Single(0).priority(), 0);

    // Range has second highest (1)
    assert_eq!(KeySelector::Range { start: 0, end: 5 }.priority(), 1);

    // Row/Column have same priority (2)
    assert_eq!(KeySelector::Row(0).priority(), 2);
    assert_eq!(KeySelector::Column(0).priority(), 2);

    // Default has lowest priority (255)
    assert_eq!(KeySelector::Default.priority(), 255);

    // Verify ordering
    assert!(KeySelector::Single(0).priority() < KeySelector::Range { start: 0, end: 5 }.priority());
    assert!(KeySelector::Range { start: 0, end: 5 }.priority() < KeySelector::Row(0).priority());
    assert!(KeySelector::Row(0).priority() < KeySelector::Default.priority());
}

// ===== might_match Tests =====

#[test]
fn test_selector_might_match() {
    // Single matches only exact key
    assert!(KeySelector::Single(5).might_match(5));
    assert!(!KeySelector::Single(5).might_match(4));
    assert!(!KeySelector::Single(5).might_match(6));

    // Range matches within bounds
    assert!(KeySelector::Range { start: 5, end: 10 }.might_match(5));
    assert!(KeySelector::Range { start: 5, end: 10 }.might_match(7));
    assert!(KeySelector::Range { start: 5, end: 10 }.might_match(10));
    assert!(!KeySelector::Range { start: 5, end: 10 }.might_match(4));
    assert!(!KeySelector::Range { start: 5, end: 10 }.might_match(11));

    // Row/Column/Default always return true (need device for exact check)
    assert!(KeySelector::Row(0).might_match(100));
    assert!(KeySelector::Column(0).might_match(100));
    assert!(KeySelector::Default.might_match(100));
}

// ===== Display and Serialization Tests =====

#[test]
fn test_selector_display() {
    assert_eq!(KeySelector::Single(5).to_string(), "5");
    assert_eq!(KeySelector::Range { start: 8, end: 15 }.to_string(), "8-15");
    assert_eq!(KeySelector::Row(2).to_string(), "row-2");
    assert_eq!(KeySelector::Column(3).to_string(), "col-3");
    assert_eq!(KeySelector::Default.to_string(), "default");
}

#[test]
fn test_selector_from_str() {
    use std::str::FromStr;

    assert_eq!(KeySelector::from_str("5").unwrap(), KeySelector::Single(5));
    assert_eq!(
        KeySelector::from_str("0-7").unwrap(),
        KeySelector::Range { start: 0, end: 7 }
    );
    assert_eq!(KeySelector::from_str("row-0").unwrap(), KeySelector::Row(0));
}

#[test]
fn test_selector_json_roundtrip() {
    let selectors = vec![
        KeySelector::Single(5),
        KeySelector::Range { start: 8, end: 15 },
        KeySelector::Row(2),
        KeySelector::Column(3),
        KeySelector::Default,
    ];

    for selector in selectors {
        let json = serde_json::to_string(&selector).unwrap();
        let parsed: KeySelector = serde_json::from_str(&json).unwrap();
        assert_eq!(selector, parsed);
    }
}

#[test]
fn test_selector_yaml_parsing() {
    // YAML deserializes strings similarly
    let yaml = r#""row-0""#;
    let selector: KeySelector = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(selector, KeySelector::Row(0));

    let yaml = r#""8-15""#;
    let selector: KeySelector = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(selector, KeySelector::Range { start: 8, end: 15 });
}

// ===== Cross-Device Validation Tests =====

#[test]
fn test_selector_cross_device_validation() {
    // A selector valid for XL might not be valid for Mini
    let xl = xl_device();
    let mini = mini_device();

    // Key 31 valid on XL, invalid on Mini
    assert!(KeySelector::Single(31).resolve(&xl).is_ok());
    assert!(KeySelector::Single(31).resolve(&mini).is_err());

    // Row 3 valid on XL, invalid on Mini
    assert!(KeySelector::Row(3).resolve(&xl).is_ok());
    assert!(KeySelector::Row(3).resolve(&mini).is_err());

    // Column 7 valid on XL, invalid on Mini (only 3 cols)
    assert!(KeySelector::Column(7).resolve(&xl).is_ok());
    assert!(KeySelector::Column(7).resolve(&mini).is_err());
}

#[test]
fn test_full_device_coverage() {
    // Verify that row + column selectors cover all keys
    let device = xl_device();

    // All rows should cover all keys
    let mut from_rows: Vec<u8> = Vec::new();
    for row in 0..4 {
        from_rows.extend(KeySelector::Row(row).resolve(&device).unwrap());
    }
    from_rows.sort();
    let expected: Vec<u8> = (0..32).collect();
    assert_eq!(from_rows, expected);

    // All columns should also cover all keys
    let mut from_cols: Vec<u8> = Vec::new();
    for col in 0..8 {
        from_cols.extend(KeySelector::Column(col).resolve(&device).unwrap());
    }
    from_cols.sort();
    assert_eq!(from_cols, expected);
}
