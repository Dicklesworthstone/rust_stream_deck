//! Integration tests for session state management.
//!
//! Tests verify state tracking, persistence, and integration
//! with device operations.

use std::path::PathBuf;

use sd::state::{KeyState, SessionState, StateSummary};

// ===== Basic State Operations Tests =====

#[test]
fn test_session_state_empty() {
    let state = SessionState::new();
    assert!(state.is_empty());
    assert_eq!(state.key_count(), 0);
    assert!(state.brightness.is_none());
}

#[test]
fn test_record_brightness() {
    let mut state = SessionState::new();

    state.record_brightness(50);
    assert_eq!(state.brightness, Some(50));
    assert!(!state.is_empty());

    // Overwrite
    state.record_brightness(75);
    assert_eq!(state.brightness, Some(75));
}

#[test]
fn test_record_set_key() {
    let mut state = SessionState::new();

    state.record_set_key(0, PathBuf::from("/tmp/icon.png"));
    assert_eq!(state.key_count(), 1);

    match state.keys.get(&0) {
        Some(KeyState::Image { path }) => {
            assert_eq!(path, &PathBuf::from("/tmp/icon.png"));
        }
        other => panic!("Expected Image state, got {:?}", other),
    }
}

#[test]
fn test_record_fill_key() {
    let mut state = SessionState::new();

    state.record_fill_key(5, "#ff0000".to_string());
    assert_eq!(state.key_count(), 1);

    match state.keys.get(&5) {
        Some(KeyState::Color { hex }) => {
            assert_eq!(hex, "#ff0000");
        }
        other => panic!("Expected Color state, got {:?}", other),
    }
}

#[test]
fn test_record_clear_key() {
    let mut state = SessionState::new();

    state.record_clear_key(10);
    assert_eq!(state.key_count(), 1);

    assert!(matches!(state.keys.get(&10), Some(KeyState::Cleared)));
}

#[test]
fn test_record_clear_all() {
    let mut state = SessionState::new();

    // Simulate clearing all keys on a Mini (6 keys)
    state.record_clear_all(6);
    assert_eq!(state.key_count(), 6);

    for key in 0..6 {
        assert!(matches!(state.keys.get(&key), Some(KeyState::Cleared)));
    }
}

// ===== State Overwrite Tests =====

#[test]
fn test_latest_state_wins() {
    let mut state = SessionState::new();

    // Set image
    state.record_set_key(0, PathBuf::from("/tmp/a.png"));
    assert!(matches!(state.keys.get(&0), Some(KeyState::Image { .. })));

    // Fill with color
    state.record_fill_key(0, "#00ff00".to_string());
    assert!(matches!(state.keys.get(&0), Some(KeyState::Color { .. })));

    // Clear
    state.record_clear_key(0);
    assert!(matches!(state.keys.get(&0), Some(KeyState::Cleared)));

    // Only one entry for key 0
    assert_eq!(state.key_count(), 1);
}

#[test]
fn test_multiple_keys_independent() {
    let mut state = SessionState::new();

    state.record_set_key(0, PathBuf::from("/tmp/a.png"));
    state.record_fill_key(1, "#ff0000".to_string());
    state.record_clear_key(2);

    assert_eq!(state.key_count(), 3);
    assert!(matches!(state.keys.get(&0), Some(KeyState::Image { .. })));
    assert!(matches!(state.keys.get(&1), Some(KeyState::Color { .. })));
    assert!(matches!(state.keys.get(&2), Some(KeyState::Cleared)));
}

// ===== Reset Tests =====

#[test]
fn test_reset_clears_all() {
    let mut state = SessionState::new();

    state.record_brightness(50);
    state.record_set_key(0, PathBuf::from("/tmp/icon.png"));
    state.record_fill_key(1, "#ff0000".to_string());
    state.record_clear_key(2);

    assert!(!state.is_empty());

    state.reset();

    assert!(state.is_empty());
    assert!(state.brightness.is_none());
    assert_eq!(state.key_count(), 0);
}

// ===== Summary Tests =====

#[test]
fn test_summary_empty() {
    let state = SessionState::new();
    let summary = state.summary();

    assert!(summary.brightness.is_none());
    assert_eq!(summary.total_keys, 0);
    assert_eq!(summary.image_keys, 0);
    assert_eq!(summary.color_keys, 0);
    assert_eq!(summary.cleared_keys, 0);
}

#[test]
fn test_summary_with_brightness() {
    let mut state = SessionState::new();
    state.record_brightness(75);

    let summary = state.summary();
    assert_eq!(summary.brightness, Some(75));
    assert_eq!(summary.total_keys, 0);
}

#[test]
fn test_summary_counts() {
    let mut state = SessionState::new();

    // 3 images
    state.record_set_key(0, PathBuf::from("/tmp/a.png"));
    state.record_set_key(1, PathBuf::from("/tmp/b.png"));
    state.record_set_key(2, PathBuf::from("/tmp/c.png"));

    // 2 colors
    state.record_fill_key(3, "#ff0000".to_string());
    state.record_fill_key(4, "#00ff00".to_string());

    // 1 cleared
    state.record_clear_key(5);

    let summary = state.summary();
    assert_eq!(summary.total_keys, 6);
    assert_eq!(summary.image_keys, 3);
    assert_eq!(summary.color_keys, 2);
    assert_eq!(summary.cleared_keys, 1);
}

#[test]
fn test_summary_after_overwrites() {
    let mut state = SessionState::new();

    // Start with 3 images
    state.record_set_key(0, PathBuf::from("/tmp/a.png"));
    state.record_set_key(1, PathBuf::from("/tmp/b.png"));
    state.record_set_key(2, PathBuf::from("/tmp/c.png"));

    // Convert one to color
    state.record_fill_key(0, "#ff0000".to_string());

    // Clear one
    state.record_clear_key(1);

    let summary = state.summary();
    assert_eq!(summary.total_keys, 3);
    assert_eq!(summary.image_keys, 1); // Only key 2
    assert_eq!(summary.color_keys, 1); // Key 0
    assert_eq!(summary.cleared_keys, 1); // Key 1
}

// ===== Serialization Tests =====

#[test]
fn test_session_state_json_roundtrip() {
    let mut state = SessionState::new();
    state.record_brightness(80);
    state.record_set_key(0, PathBuf::from("/tmp/icon.png"));
    state.record_fill_key(1, "#ff0000".to_string());
    state.record_clear_key(2);

    let json = serde_json::to_string(&state).unwrap();
    let parsed: SessionState = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.brightness, Some(80));
    assert_eq!(parsed.key_count(), 3);
}

#[test]
fn test_key_state_json_serialization() {
    // Image state
    let image_state = KeyState::Image {
        path: PathBuf::from("/tmp/test.png"),
    };
    let json = serde_json::to_string(&image_state).unwrap();
    assert!(json.contains("\"type\":\"image\""));

    // Color state
    let color_state = KeyState::Color {
        hex: "#ff0000".to_string(),
    };
    let json = serde_json::to_string(&color_state).unwrap();
    assert!(json.contains("\"type\":\"color\""));

    // Cleared state
    let cleared_state = KeyState::Cleared;
    let json = serde_json::to_string(&cleared_state).unwrap();
    assert!(json.contains("\"type\":\"cleared\""));
}

// ===== Edge Cases =====

#[test]
fn test_boundary_key_indices() {
    let mut state = SessionState::new();

    // Key 0 (first)
    state.record_set_key(0, PathBuf::from("/tmp/first.png"));

    // Key 255 (max u8)
    state.record_set_key(255, PathBuf::from("/tmp/last.png"));

    assert_eq!(state.key_count(), 2);
    assert!(state.keys.contains_key(&0));
    assert!(state.keys.contains_key(&255));
}

#[test]
fn test_brightness_boundary_values() {
    let mut state = SessionState::new();

    state.record_brightness(0);
    assert_eq!(state.brightness, Some(0));

    state.record_brightness(100);
    assert_eq!(state.brightness, Some(100));

    // Note: Validation of 0-100 range is done at device level,
    // state just records what was set
    state.record_brightness(255);
    assert_eq!(state.brightness, Some(255));
}

#[test]
fn test_empty_hex_color() {
    let mut state = SessionState::new();

    // Edge case: empty hex string
    state.record_fill_key(0, String::new());

    match state.keys.get(&0) {
        Some(KeyState::Color { hex }) => {
            assert!(hex.is_empty());
        }
        other => panic!("Expected Color state, got {:?}", other),
    }
}

#[test]
fn test_unicode_in_paths() {
    let mut state = SessionState::new();

    // Path with unicode characters
    state.record_set_key(0, PathBuf::from("/tmp/アイコン.png"));
    state.record_set_key(1, PathBuf::from("/tmp/icône.png"));
    state.record_set_key(2, PathBuf::from("/tmp/图标.png"));

    assert_eq!(state.key_count(), 3);
}

// ===== Large State Tests =====

#[test]
fn test_all_keys_xl() {
    let mut state = SessionState::new();

    // Record all 32 keys (XL)
    for key in 0..32 {
        state.record_set_key(key, PathBuf::from(format!("/tmp/key-{}.png", key)));
    }

    assert_eq!(state.key_count(), 32);

    let summary = state.summary();
    assert_eq!(summary.total_keys, 32);
    assert_eq!(summary.image_keys, 32);
}

#[test]
fn test_mixed_large_state() {
    let mut state = SessionState::new();
    state.record_brightness(50);

    // 10 images
    for key in 0..10 {
        state.record_set_key(key, PathBuf::from(format!("/tmp/key-{}.png", key)));
    }

    // 10 colors
    for key in 10..20 {
        state.record_fill_key(key, format!("#{:02x}{:02x}{:02x}", key, key, key));
    }

    // 12 cleared
    for key in 20..32 {
        state.record_clear_key(key);
    }

    let summary = state.summary();
    assert_eq!(summary.brightness, Some(50));
    assert_eq!(summary.total_keys, 32);
    assert_eq!(summary.image_keys, 10);
    assert_eq!(summary.color_keys, 10);
    assert_eq!(summary.cleared_keys, 12);
}

// ===== Clone Tests =====

#[test]
fn test_session_state_clone() {
    let mut state = SessionState::new();
    state.record_brightness(75);
    state.record_set_key(0, PathBuf::from("/tmp/icon.png"));

    let cloned = state.clone();

    // Cloned should have same values
    assert_eq!(cloned.brightness, Some(75));
    assert_eq!(cloned.key_count(), 1);

    // Modify original
    state.record_brightness(100);

    // Clone should be independent
    assert_eq!(cloned.brightness, Some(75));
}
