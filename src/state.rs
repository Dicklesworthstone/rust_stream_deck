//! Session state tracking for device changes.
//!
//! Tracks brightness and key state changes during a session for
//! snapshot save/restore functionality.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace};

/// State of a single key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyState {
    /// Key has an image set (source path).
    Image {
        /// Path to the source image file.
        path: PathBuf,
    },
    /// Key filled with solid color (hex).
    Color {
        /// Hex color string (e.g., "#ff0000").
        hex: String,
    },
    /// Key explicitly cleared.
    Cleared,
}

/// Tracked session state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    /// Current brightness (if set during session).
    pub brightness: Option<u8>,
    /// Per-key state (only keys modified during session).
    pub keys: HashMap<u8, KeyState>,
}

impl SessionState {
    /// Create a new empty session state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a brightness change.
    pub fn record_brightness(&mut self, level: u8) {
        debug!(level = %level, "Recording brightness change");
        self.brightness = Some(level);
    }

    /// Record setting a key image.
    pub fn record_set_key(&mut self, key: u8, path: PathBuf) {
        trace!(key = %key, path = %path.display(), "Recording key image");
        self.keys.insert(key, KeyState::Image { path });
    }

    /// Record filling a key with color.
    pub fn record_fill_key(&mut self, key: u8, color: String) {
        trace!(key = %key, color = %color, "Recording key color");
        self.keys.insert(key, KeyState::Color { hex: color });
    }

    /// Record clearing a key.
    pub fn record_clear_key(&mut self, key: u8) {
        trace!(key = %key, "Recording key clear");
        self.keys.insert(key, KeyState::Cleared);
    }

    /// Record clearing all keys.
    pub fn record_clear_all(&mut self, key_count: u8) {
        trace!(key_count = %key_count, "Recording clear all");
        for key in 0..key_count {
            self.keys.insert(key, KeyState::Cleared);
        }
    }

    /// Clear all tracked state.
    pub fn reset(&mut self) {
        info!("Session state reset");
        self.brightness = None;
        self.keys.clear();
    }

    /// Check if any state has been tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.brightness.is_none() && self.keys.is_empty()
    }

    /// Get count of tracked keys.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Get a summary of the tracked state.
    #[must_use]
    pub fn summary(&self) -> StateSummary {
        let mut image_count = 0;
        let mut color_count = 0;
        let mut cleared_count = 0;

        for state in self.keys.values() {
            match state {
                KeyState::Image { .. } => image_count += 1,
                KeyState::Color { .. } => color_count += 1,
                KeyState::Cleared => cleared_count += 1,
            }
        }

        StateSummary {
            brightness: self.brightness,
            total_keys: self.keys.len(),
            image_keys: image_count,
            color_keys: color_count,
            cleared_keys: cleared_count,
        }
    }
}

/// Summary of session state for reporting.
#[derive(Debug, Clone, Serialize)]
pub struct StateSummary {
    /// Brightness level if set.
    pub brightness: Option<u8>,
    /// Total number of keys with tracked state.
    pub total_keys: usize,
    /// Number of keys with images.
    pub image_keys: usize,
    /// Number of keys with solid colors.
    pub color_keys: usize,
    /// Number of cleared keys.
    pub cleared_keys: usize,
}

/// Global session state with thread-safe access.
static SESSION_STATE: LazyLock<RwLock<SessionState>> =
    LazyLock::new(|| RwLock::new(SessionState::default()));

/// Get read access to session state.
///
/// # Panics
///
/// Panics if the lock is poisoned.
#[must_use]
pub fn session_state() -> RwLockReadGuard<'static, SessionState> {
    SESSION_STATE.read().expect("session state lock poisoned")
}

/// Get write access to session state.
///
/// # Panics
///
/// Panics if the lock is poisoned.
#[must_use]
pub fn session_state_mut() -> RwLockWriteGuard<'static, SessionState> {
    SESSION_STATE.write().expect("session state lock poisoned")
}

/// Record operations using the global state.
///
/// These are convenience functions for recording state changes
/// without manually acquiring the lock.
pub mod record {
    use super::*;

    /// Record a brightness change.
    pub fn brightness(level: u8) {
        session_state_mut().record_brightness(level);
    }

    /// Record setting a key image.
    pub fn set_key(key: u8, path: PathBuf) {
        session_state_mut().record_set_key(key, path);
    }

    /// Record filling a key with color.
    pub fn fill_key(key: u8, color: String) {
        session_state_mut().record_fill_key(key, color);
    }

    /// Record clearing a key.
    pub fn clear_key(key: u8) {
        session_state_mut().record_clear_key(key);
    }

    /// Record clearing all keys.
    pub fn clear_all(key_count: u8) {
        session_state_mut().record_clear_all(key_count);
    }

    /// Reset all tracked state.
    pub fn reset() {
        session_state_mut().reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(state.is_empty());
        assert_eq!(state.key_count(), 0);
        assert_eq!(state.brightness, None);
    }

    #[test]
    fn test_record_brightness() {
        let mut state = SessionState::new();
        state.record_brightness(75);
        assert_eq!(state.brightness, Some(75));
        assert!(!state.is_empty());
    }

    #[test]
    fn test_record_set_key() {
        let mut state = SessionState::new();
        state.record_set_key(5, PathBuf::from("/tmp/icon.png"));
        assert_eq!(state.key_count(), 1);
        assert!(matches!(
            state.keys.get(&5),
            Some(KeyState::Image { path }) if path == &PathBuf::from("/tmp/icon.png")
        ));
    }

    #[test]
    fn test_record_fill_key() {
        let mut state = SessionState::new();
        state.record_fill_key(3, "#ff0000".to_string());
        assert_eq!(state.key_count(), 1);
        assert!(matches!(
            state.keys.get(&3),
            Some(KeyState::Color { hex }) if hex == "#ff0000"
        ));
    }

    #[test]
    fn test_record_clear_key() {
        let mut state = SessionState::new();
        state.record_clear_key(7);
        assert_eq!(state.key_count(), 1);
        assert!(matches!(state.keys.get(&7), Some(KeyState::Cleared)));
    }

    #[test]
    fn test_record_clear_all() {
        let mut state = SessionState::new();
        state.record_clear_all(6); // Mini has 6 keys
        assert_eq!(state.key_count(), 6);
        for key in 0..6 {
            assert!(matches!(state.keys.get(&key), Some(KeyState::Cleared)));
        }
    }

    #[test]
    fn test_latest_wins() {
        let mut state = SessionState::new();
        state.record_set_key(0, PathBuf::from("/tmp/a.png"));
        state.record_fill_key(0, "#00ff00".to_string());
        state.record_clear_key(0);

        // Latest operation should win
        assert_eq!(state.key_count(), 1);
        assert!(matches!(state.keys.get(&0), Some(KeyState::Cleared)));
    }

    #[test]
    fn test_reset() {
        let mut state = SessionState::new();
        state.record_brightness(50);
        state.record_set_key(0, PathBuf::from("/tmp/icon.png"));
        state.record_fill_key(1, "#ff0000".to_string());

        state.reset();

        assert!(state.is_empty());
        assert_eq!(state.brightness, None);
        assert_eq!(state.key_count(), 0);
    }

    #[test]
    fn test_summary() {
        let mut state = SessionState::new();
        state.record_brightness(80);
        state.record_set_key(0, PathBuf::from("/tmp/a.png"));
        state.record_set_key(1, PathBuf::from("/tmp/b.png"));
        state.record_fill_key(2, "#ff0000".to_string());
        state.record_clear_key(3);
        state.record_clear_key(4);
        state.record_clear_key(5);

        let summary = state.summary();
        assert_eq!(summary.brightness, Some(80));
        assert_eq!(summary.total_keys, 6);
        assert_eq!(summary.image_keys, 2);
        assert_eq!(summary.color_keys, 1);
        assert_eq!(summary.cleared_keys, 3);
    }
}
