//! Mock device implementation for unit testing.
//!
//! This module provides a mock Stream Deck device that records
//! all operations and supports assertions for testing.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_stream_deck::device::mock::{MockDevice, Operation};
//! use rust_stream_deck::device::DeviceOperations;
//!
//! let mut mock = MockDevice::xl();
//!
//! // Perform operations
//! mock.set_brightness(50).unwrap();
//! mock.fill_key_color(0, (255, 0, 0)).unwrap();
//!
//! // Assert what happened
//! mock.assert_operations(&[
//!     Operation::SetBrightness { level: 50 },
//!     Operation::FillKeyColor { key: 0, r: 255, g: 0, b: 0 },
//! ]);
//! ```

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use tracing::{debug, trace};

use super::DeviceOperations;
use super::info::{DeviceInfo, DeviceModel};
use crate::error::{Result, SdError};

/// Recorded operation for assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    SetBrightness {
        level: u8,
    },
    SetKeyImage {
        key: u8,
        path: String,
    },
    ClearKey {
        key: u8,
    },
    ClearAllKeys,
    FillKeyColor {
        key: u8,
        r: u8,
        g: u8,
        b: u8,
    },
    FillAllKeysColor {
        r: u8,
        g: u8,
        b: u8,
    },
    ReadButtonStates,
    WatchButtons {
        json_output: bool,
        once: bool,
        timeout_secs: u64,
    },
}

/// State of a key on the mock device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyState {
    /// Key is cleared (black)
    Clear,
    /// Key has an image (path stored for reference)
    Image(String),
    /// Key is filled with a solid color
    Color { r: u8, g: u8, b: u8 },
}

/// Configuration for mock behavior.
#[derive(Debug, Clone, Default)]
pub struct MockConfig {
    /// Fail after N operations (for testing error recovery).
    pub fail_after_ops: Option<usize>,
    /// Specific keys that should fail on operations.
    pub failing_keys: Vec<u8>,
    /// Initial connection state.
    pub connected: bool,
}

impl MockConfig {
    /// Create a connected mock configuration.
    #[must_use]
    pub fn connected() -> Self {
        Self {
            connected: true,
            ..Default::default()
        }
    }
}

/// Mock device for testing without real hardware.
///
/// Records all operations for later assertion and provides
/// various ways to simulate device behavior and errors.
pub struct MockDevice {
    info: DeviceInfo,
    brightness: AtomicU8,
    keys: Mutex<Vec<KeyState>>,
    button_states: Mutex<Vec<bool>>,
    input_queue: Mutex<VecDeque<(u8, bool)>>,
    operation_log: Mutex<Vec<Operation>>,
    error_injection: Mutex<Option<SdError>>,
    config: MockConfig,
    op_count: Mutex<usize>,
    connected: AtomicBool,
}

impl MockDevice {
    /// Create a new mock device for the specified model.
    #[must_use]
    pub fn new(model: DeviceModel) -> Self {
        let (cols, rows) = model.layout();
        let (width, height) = model.key_dimensions();
        let key_count = model.key_count();

        debug!(?model, "Creating mock device");

        let keys = vec![KeyState::Clear; key_count as usize];
        let button_states = vec![false; key_count as usize];

        Self {
            info: DeviceInfo {
                serial: format!("MOCK-{:?}-001", model),
                product_name: model.display_name().to_string(),
                firmware_version: "1.0.0-mock".to_string(),
                key_count,
                key_width: width as usize,
                key_height: height as usize,
                rows,
                cols,
                kind: format!("{model:?}"),
            },
            brightness: AtomicU8::new(100),
            keys: Mutex::new(keys),
            button_states: Mutex::new(button_states),
            input_queue: Mutex::new(VecDeque::new()),
            operation_log: Mutex::new(Vec::new()),
            error_injection: Mutex::new(None),
            config: MockConfig::connected(),
            op_count: Mutex::new(0),
            connected: AtomicBool::new(true),
        }
    }

    /// Create mock for Stream Deck XL (most common for testing).
    #[must_use]
    pub fn xl() -> Self {
        Self::new(DeviceModel::Xl)
    }

    /// Create mock for Stream Deck Mini.
    #[must_use]
    pub fn mini() -> Self {
        Self::new(DeviceModel::Mini)
    }

    /// Create mock for Stream Deck MK.2.
    #[must_use]
    pub fn mk2() -> Self {
        Self::new(DeviceModel::Mk2)
    }

    // === Configuration ===

    /// Configure mock behavior.
    #[must_use]
    pub fn with_config(mut self, config: MockConfig) -> Self {
        self.connected.store(config.connected, Ordering::SeqCst);
        self.config = config;
        self
    }

    /// Inject an error for the next operation.
    pub fn inject_error(&self, error: SdError) {
        *self.error_injection.lock().unwrap() = Some(error);
    }

    /// Clear injected error.
    pub fn clear_error(&self) {
        *self.error_injection.lock().unwrap() = None;
    }

    /// Set device as disconnected.
    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::SeqCst);
    }

    /// Set device as connected.
    pub fn reconnect(&self) {
        self.connected.store(true, Ordering::SeqCst);
    }

    // === Input Simulation ===

    /// Queue a button press event.
    pub fn queue_press(&self, key: u8) {
        self.input_queue.lock().unwrap().push_back((key, true));
    }

    /// Queue a button release event.
    pub fn queue_release(&self, key: u8) {
        self.input_queue.lock().unwrap().push_back((key, false));
    }

    /// Queue a button tap (press + release).
    pub fn queue_tap(&self, key: u8) {
        let mut queue = self.input_queue.lock().unwrap();
        queue.push_back((key, true));
        queue.push_back((key, false));
    }

    /// Set a button's current state.
    pub fn set_button_state(&self, key: u8, pressed: bool) {
        let mut states = self.button_states.lock().unwrap();
        if (key as usize) < states.len() {
            states[key as usize] = pressed;
        }
    }

    // === Assertions ===

    /// Get all recorded operations.
    #[must_use]
    pub fn operations(&self) -> Vec<Operation> {
        self.operation_log.lock().unwrap().clone()
    }

    /// Get the number of operations performed.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.operation_log.lock().unwrap().len()
    }

    /// Assert specific operations were performed.
    ///
    /// # Panics
    ///
    /// Panics if the operations don't match.
    pub fn assert_operations(&self, expected: &[Operation]) {
        let actual = self.operations();
        assert_eq!(
            actual, expected,
            "Operation mismatch.\nExpected: {expected:#?}\nActual: {actual:#?}",
        );
    }

    /// Assert no operations were performed.
    ///
    /// # Panics
    ///
    /// Panics if any operations were recorded.
    pub fn assert_no_operations(&self) {
        let ops = self.operations();
        assert!(
            ops.is_empty(),
            "Expected no operations, but found: {ops:#?}",
        );
    }

    /// Assert a specific operation was performed at least once.
    ///
    /// # Panics
    ///
    /// Panics if the operation was not found.
    pub fn assert_contains(&self, expected: &Operation) {
        let ops = self.operations();
        assert!(
            ops.contains(expected),
            "Expected operation {expected:?} not found in: {ops:#?}",
        );
    }

    /// Get the state of a specific key.
    #[must_use]
    pub fn get_key_state(&self, key: u8) -> Option<KeyState> {
        let keys = self.keys.lock().unwrap();
        keys.get(key as usize).cloned()
    }

    /// Assert key has an image.
    ///
    /// # Panics
    ///
    /// Panics if the key doesn't have an image.
    pub fn assert_key_has_image(&self, key: u8) {
        match self.get_key_state(key) {
            Some(KeyState::Image(_)) => {}
            other => panic!("Key {key} expected to have image, but has: {other:?}"),
        }
    }

    /// Assert key has a specific color.
    ///
    /// # Panics
    ///
    /// Panics if the key doesn't have the expected color.
    pub fn assert_key_color(&self, key: u8, r: u8, g: u8, b: u8) {
        match self.get_key_state(key) {
            Some(KeyState::Color {
                r: kr,
                g: kg,
                b: kb,
            }) if kr == r && kg == g && kb == b => {}
            other => panic!("Key {key} expected color ({r}, {g}, {b}), but has: {other:?}",),
        }
    }

    /// Assert key is cleared.
    ///
    /// # Panics
    ///
    /// Panics if the key is not cleared.
    pub fn assert_key_cleared(&self, key: u8) {
        match self.get_key_state(key) {
            None | Some(KeyState::Clear) => {}
            other => panic!("Key {key} expected to be cleared, but has: {other:?}"),
        }
    }

    /// Get current brightness level.
    #[must_use]
    pub fn get_brightness(&self) -> u8 {
        self.brightness.load(Ordering::SeqCst)
    }

    /// Clear the operation log for fresh assertions.
    pub fn clear_operations(&self) {
        self.operation_log.lock().unwrap().clear();
        *self.op_count.lock().unwrap() = 0;
    }

    // === Internal Helpers ===

    fn record_op(&self, op: Operation) {
        trace!(?op, "Recording operation");
        self.operation_log.lock().unwrap().push(op);
        *self.op_count.lock().unwrap() += 1;
    }

    fn check_error(&self) -> Result<()> {
        // Check for injected error
        if let Some(error) = self.error_injection.lock().unwrap().take() {
            return Err(error);
        }

        // Check for connection
        if !self.connected.load(Ordering::SeqCst) {
            return Err(SdError::DeviceCommunication(
                "Mock device disconnected".to_string(),
            ));
        }

        // Check for fail_after_ops
        if let Some(limit) = self.config.fail_after_ops {
            let count = *self.op_count.lock().unwrap();
            if count >= limit {
                return Err(SdError::DeviceCommunication(
                    "Mock failure after ops limit".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn check_key(&self, key: u8) -> Result<()> {
        if self.config.failing_keys.contains(&key) {
            return Err(SdError::DeviceCommunication(format!(
                "Mock key {key} configured to fail"
            )));
        }
        if key >= self.info.key_count {
            return Err(SdError::InvalidKeyIndex {
                index: key,
                max: self.info.key_count,
                max_idx: self.info.key_count - 1,
            });
        }
        Ok(())
    }
}

impl DeviceOperations for MockDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn set_brightness(&self, level: u8) -> Result<()> {
        self.check_error()?;
        self.record_op(Operation::SetBrightness { level });
        self.brightness.store(level.min(100), Ordering::SeqCst);
        Ok(())
    }

    fn set_key_image(&self, key: u8, path: &Path) -> Result<()> {
        self.check_error()?;
        self.check_key(key)?;

        self.record_op(Operation::SetKeyImage {
            key,
            path: path.display().to_string(),
        });

        let mut keys = self.keys.lock().unwrap();
        keys[key as usize] = KeyState::Image(path.display().to_string());

        Ok(())
    }

    fn clear_key(&self, key: u8) -> Result<()> {
        self.check_error()?;
        self.check_key(key)?;

        self.record_op(Operation::ClearKey { key });

        let mut keys = self.keys.lock().unwrap();
        keys[key as usize] = KeyState::Clear;

        Ok(())
    }

    fn clear_all_keys(&self) -> Result<()> {
        self.check_error()?;
        self.record_op(Operation::ClearAllKeys);

        let mut keys = self.keys.lock().unwrap();
        for key in keys.iter_mut() {
            *key = KeyState::Clear;
        }

        Ok(())
    }

    fn fill_key_color(&self, key: u8, color: (u8, u8, u8)) -> Result<()> {
        self.check_error()?;
        self.check_key(key)?;

        let (r, g, b) = color;
        self.record_op(Operation::FillKeyColor { key, r, g, b });

        let mut keys = self.keys.lock().unwrap();
        keys[key as usize] = KeyState::Color { r, g, b };

        Ok(())
    }

    fn fill_all_keys_color(&self, color: (u8, u8, u8)) -> Result<()> {
        self.check_error()?;

        let (r, g, b) = color;
        self.record_op(Operation::FillAllKeysColor { r, g, b });

        let mut keys = self.keys.lock().unwrap();
        for key in keys.iter_mut() {
            *key = KeyState::Color { r, g, b };
        }

        Ok(())
    }

    fn read_button_states(&self) -> Vec<bool> {
        self.record_op(Operation::ReadButtonStates);

        // Process any queued events first
        let mut states = self.button_states.lock().unwrap();
        let mut queue = self.input_queue.lock().unwrap();

        while let Some((key, pressed)) = queue.pop_front() {
            if (key as usize) < states.len() {
                states[key as usize] = pressed;
            }
        }

        states.clone()
    }

    fn watch_buttons(&self, json_output: bool, once: bool, timeout_secs: u64) -> Result<()> {
        self.check_error()?;

        self.record_op(Operation::WatchButtons {
            json_output,
            once,
            timeout_secs,
        });

        // For mock, just return immediately (tests will use queue_press/etc.)
        Ok(())
    }
}

/// Builder for creating `MockDevice` with common configurations.
pub struct MockDeviceBuilder {
    model: DeviceModel,
    config: MockConfig,
    initial_brightness: u8,
    initial_keys: Vec<KeyState>,
}

impl MockDeviceBuilder {
    /// Create a new builder for the specified model.
    #[must_use]
    pub fn new(model: DeviceModel) -> Self {
        let key_count = model.key_count() as usize;
        Self {
            model,
            config: MockConfig::connected(),
            initial_brightness: 100,
            initial_keys: vec![KeyState::Clear; key_count],
        }
    }

    /// Create a builder for Stream Deck XL.
    #[must_use]
    pub fn xl() -> Self {
        Self::new(DeviceModel::Xl)
    }

    /// Create a builder for Stream Deck Mini.
    #[must_use]
    pub fn mini() -> Self {
        Self::new(DeviceModel::Mini)
    }

    /// Set device to fail after N operations.
    #[must_use]
    pub fn fail_after(mut self, ops: usize) -> Self {
        self.config.fail_after_ops = Some(ops);
        self
    }

    /// Set specific keys to fail.
    #[must_use]
    pub fn with_failing_keys(mut self, keys: Vec<u8>) -> Self {
        self.config.failing_keys = keys;
        self
    }

    /// Create device in disconnected state.
    #[must_use]
    pub fn disconnected(mut self) -> Self {
        self.config.connected = false;
        self
    }

    /// Set initial brightness.
    #[must_use]
    pub fn with_brightness(mut self, level: u8) -> Self {
        self.initial_brightness = level;
        self
    }

    /// Build the mock device.
    #[must_use]
    pub fn build(self) -> MockDevice {
        let mut device = MockDevice::new(self.model).with_config(self.config);
        device
            .brightness
            .store(self.initial_brightness, Ordering::SeqCst);
        *device.keys.lock().unwrap() = self.initial_keys;
        device
    }
}

/// Create a thread-safe mock device wrapped in an `Arc`.
///
/// Useful for tests that need to share the mock across threads.
#[must_use]
pub fn arc_mock_xl() -> Arc<MockDevice> {
    Arc::new(MockDevice::xl())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_device_creation() {
        let mock = MockDevice::xl();
        assert_eq!(mock.info().key_count, 32);
        assert_eq!(mock.get_brightness(), 100);
        assert!(mock.is_connected());
    }

    #[test]
    fn test_set_brightness() {
        let mock = MockDevice::xl();
        mock.set_brightness(50).unwrap();

        assert_eq!(mock.get_brightness(), 50);
        mock.assert_operations(&[Operation::SetBrightness { level: 50 }]);
    }

    #[test]
    fn test_set_key_image() {
        let mock = MockDevice::xl();
        mock.set_key_image(5, Path::new("/test/image.png")).unwrap();

        mock.assert_key_has_image(5);
        mock.assert_contains(&Operation::SetKeyImage {
            key: 5,
            path: "/test/image.png".to_string(),
        });
    }

    #[test]
    fn test_clear_key() {
        let mock = MockDevice::xl();
        mock.fill_key_color(3, (255, 0, 0)).unwrap();
        mock.clear_key(3).unwrap();

        mock.assert_key_cleared(3);
    }

    #[test]
    fn test_clear_all_keys() {
        let mock = MockDevice::xl();
        mock.fill_key_color(0, (255, 0, 0)).unwrap();
        mock.fill_key_color(1, (0, 255, 0)).unwrap();
        mock.clear_all_keys().unwrap();

        mock.assert_key_cleared(0);
        mock.assert_key_cleared(1);
    }

    #[test]
    fn test_fill_key_color() {
        let mock = MockDevice::xl();
        mock.fill_key_color(7, (255, 128, 64)).unwrap();

        mock.assert_key_color(7, 255, 128, 64);
    }

    #[test]
    fn test_fill_all_keys_color() {
        let mock = MockDevice::xl();
        mock.fill_all_keys_color((100, 100, 100)).unwrap();

        mock.assert_key_color(0, 100, 100, 100);
        mock.assert_key_color(15, 100, 100, 100);
        mock.assert_key_color(31, 100, 100, 100);
    }

    #[test]
    fn test_error_injection() {
        let mock = MockDevice::xl();
        mock.inject_error(SdError::DeviceCommunication("test error".to_string()));

        let result = mock.set_brightness(50);
        assert!(result.is_err());
    }

    #[test]
    fn test_disconnected_device() {
        let mock = MockDeviceBuilder::xl().disconnected().build();

        let result = mock.set_brightness(50);
        assert!(result.is_err());
    }

    #[test]
    fn test_failing_keys() {
        let mock = MockDeviceBuilder::xl()
            .with_failing_keys(vec![5, 10])
            .build();

        // Key 0 should work
        mock.clear_key(0).unwrap();

        // Key 5 should fail
        let result = mock.clear_key(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_fail_after_ops() {
        let mock = MockDeviceBuilder::xl().fail_after(3).build();

        mock.set_brightness(50).unwrap();
        mock.set_brightness(60).unwrap();
        mock.set_brightness(70).unwrap();

        // 4th operation should fail
        let result = mock.set_brightness(80);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_index() {
        let mock = MockDevice::xl(); // 32 keys, indices 0-31

        let result = mock.clear_key(32);
        assert!(matches!(result, Err(SdError::InvalidKeyIndex { .. })));
    }

    #[test]
    fn test_button_states() {
        let mock = MockDevice::xl();
        mock.set_button_state(5, true);

        let states = mock.read_button_states();
        assert!(states[5]);
        assert!(!states[0]);
    }

    #[test]
    fn test_clear_operations() {
        let mock = MockDevice::xl();
        mock.set_brightness(50).unwrap();
        assert_eq!(mock.operation_count(), 1);

        mock.clear_operations();
        assert_eq!(mock.operation_count(), 0);
    }

    #[test]
    fn test_disconnect_reconnect() {
        let mock = MockDevice::xl();
        assert!(mock.is_connected());

        mock.disconnect();
        assert!(!mock.is_connected());
        assert!(mock.set_brightness(50).is_err());

        mock.reconnect();
        assert!(mock.is_connected());
        mock.set_brightness(50).unwrap();
    }

    #[test]
    fn test_builder_with_brightness() {
        let mock = MockDeviceBuilder::xl().with_brightness(75).build();
        assert_eq!(mock.get_brightness(), 75);
    }

    #[test]
    fn test_different_models() {
        let mini = MockDevice::mini();
        assert_eq!(mini.info().key_count, 6);

        let mk2 = MockDevice::mk2();
        assert_eq!(mk2.info().key_count, 15);
    }
}
