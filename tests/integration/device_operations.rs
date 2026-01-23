//! Integration tests for device operations using MockDevice.
//!
//! Tests verify that device operations work correctly in combination,
//! including error handling, state tracking, and edge cases.

use std::path::Path;

use sd::device::mock::{MockDevice, MockDeviceBuilder, Operation};
use sd::device::DeviceOperations;
use sd::error::SdError;
use sd::image_ops::ResizeStrategy;

/// Test that brightness bounds are enforced correctly.
#[test]
fn test_brightness_bounds() {
    let mock = MockDevice::xl();

    // Valid brightness values
    mock.set_brightness(0).unwrap();
    assert_eq!(mock.get_brightness(), 0);

    mock.set_brightness(100).unwrap();
    assert_eq!(mock.get_brightness(), 100);

    mock.set_brightness(50).unwrap();
    assert_eq!(mock.get_brightness(), 50);

    // Values > 100 are clamped
    mock.set_brightness(150).unwrap();
    assert_eq!(mock.get_brightness(), 100);
}

/// Test that key images are recorded correctly.
#[test]
fn test_key_image_recording() {
    let mock = MockDevice::xl();
    let test_path = Path::new("/tmp/test_icon.png");

    mock.set_key_image(0, test_path, ResizeStrategy::Fit).unwrap();
    mock.assert_key_has_image(0);

    mock.assert_contains(&Operation::SetKeyImage {
        key: 0,
        path: "/tmp/test_icon.png".to_string(),
    });
}

/// Test that error injection works correctly.
#[test]
fn test_error_injection() {
    let mock = MockDevice::xl();
    mock.inject_error(SdError::DeviceCommunication("test error".to_string()));

    let result = mock.set_brightness(50);
    assert!(result.is_err());

    // Error should be consumed, next operation should work
    mock.set_brightness(75).unwrap();
    assert_eq!(mock.get_brightness(), 75);
}

/// Test sequence of operations and their recording.
#[test]
fn test_operation_sequence() {
    let mock = MockDevice::xl();

    mock.set_brightness(50).unwrap();
    mock.fill_key_color(0, (255, 0, 0)).unwrap();
    mock.fill_key_color(1, (0, 255, 0)).unwrap();
    mock.clear_key(0).unwrap();

    mock.assert_operations(&[
        Operation::SetBrightness { level: 50 },
        Operation::FillKeyColor {
            key: 0,
            r: 255,
            g: 0,
            b: 0,
        },
        Operation::FillKeyColor {
            key: 1,
            r: 0,
            g: 255,
            b: 0,
        },
        Operation::ClearKey { key: 0 },
    ]);
}

/// Test clear_all_keys resets all key states.
#[test]
fn test_clear_all_keys_integration() {
    let mock = MockDevice::xl();

    // Set up some keys
    mock.fill_key_color(0, (255, 0, 0)).unwrap();
    mock.fill_key_color(15, (0, 255, 0)).unwrap();
    mock.fill_key_color(31, (0, 0, 255)).unwrap();

    // Clear all
    mock.clear_all_keys().unwrap();

    // All keys should be cleared
    mock.assert_key_cleared(0);
    mock.assert_key_cleared(15);
    mock.assert_key_cleared(31);
}

/// Test fill_all_keys_color sets all keys.
#[test]
fn test_fill_all_keys_color_integration() {
    let mock = MockDevice::xl();

    mock.fill_all_keys_color((100, 100, 100)).unwrap();

    // Sample several keys
    mock.assert_key_color(0, 100, 100, 100);
    mock.assert_key_color(15, 100, 100, 100);
    mock.assert_key_color(31, 100, 100, 100);
}

/// Test that disconnected device fails operations.
#[test]
fn test_disconnected_device_operations() {
    let mock = MockDeviceBuilder::xl().disconnected().build();

    assert!(mock.set_brightness(50).is_err());
    assert!(mock.fill_key_color(0, (255, 0, 0)).is_err());
    assert!(mock.clear_key(0).is_err());
    assert!(mock.clear_all_keys().is_err());
}

/// Test reconnect behavior.
#[test]
fn test_reconnect_operations() {
    let mock = MockDevice::xl();

    mock.set_brightness(50).unwrap();
    mock.disconnect();
    assert!(mock.set_brightness(75).is_err());

    mock.reconnect();
    mock.set_brightness(100).unwrap();
    assert_eq!(mock.get_brightness(), 100);
}

/// Test that failing keys are handled correctly.
#[test]
fn test_failing_keys_selective() {
    let mock = MockDeviceBuilder::xl().with_failing_keys(vec![5, 10, 15]).build();

    // Non-failing keys should work
    mock.fill_key_color(0, (255, 0, 0)).unwrap();
    mock.fill_key_color(1, (0, 255, 0)).unwrap();

    // Failing keys should error
    assert!(mock.fill_key_color(5, (255, 0, 0)).is_err());
    assert!(mock.fill_key_color(10, (0, 255, 0)).is_err());
    assert!(mock.fill_key_color(15, (0, 0, 255)).is_err());

    // Other keys still work
    mock.fill_key_color(20, (255, 255, 0)).unwrap();
}

/// Test fail_after_ops limit.
#[test]
fn test_fail_after_ops_limit() {
    let mock = MockDeviceBuilder::xl().fail_after(5).build();

    // First 5 operations should succeed
    mock.set_brightness(10).unwrap();
    mock.set_brightness(20).unwrap();
    mock.set_brightness(30).unwrap();
    mock.set_brightness(40).unwrap();
    mock.set_brightness(50).unwrap();

    // 6th operation should fail
    assert!(mock.set_brightness(60).is_err());
}

/// Test invalid key index returns proper error.
#[test]
fn test_invalid_key_index_error() {
    let mock = MockDevice::xl(); // 32 keys (0-31)

    let result = mock.clear_key(32);
    assert!(matches!(
        result,
        Err(SdError::InvalidKeyIndex {
            index: 32,
            max: 32,
            ..
        })
    ));
}

/// Test button state queuing.
#[test]
fn test_button_state_queuing() {
    let mock = MockDevice::xl();

    // Queue some button events
    mock.queue_press(5);
    mock.queue_release(5);
    mock.queue_tap(10);

    // Read states processes the queue
    let states = mock.read_button_states();

    // After tap, key 10 should be released (false)
    assert!(!states[10]);

    // Verify operation was recorded
    mock.assert_contains(&Operation::ReadButtonStates);
}

/// Test that operations are recorded correctly after clear.
#[test]
fn test_clear_operations_fresh_start() {
    let mock = MockDevice::xl();

    mock.set_brightness(50).unwrap();
    mock.fill_key_color(0, (255, 0, 0)).unwrap();
    assert_eq!(mock.operation_count(), 2);

    mock.clear_operations();
    assert_eq!(mock.operation_count(), 0);

    mock.set_brightness(75).unwrap();
    assert_eq!(mock.operation_count(), 1);
    mock.assert_operations(&[Operation::SetBrightness { level: 75 }]);
}

/// Test different device models have correct key counts.
#[test]
fn test_device_model_key_counts() {
    let xl = MockDevice::xl();
    assert_eq!(xl.info().key_count, 32);
    assert_eq!(xl.info().rows, 4);
    assert_eq!(xl.info().cols, 8);

    let mini = MockDevice::mini();
    assert_eq!(mini.info().key_count, 6);
    assert_eq!(mini.info().rows, 2);
    assert_eq!(mini.info().cols, 3);

    let mk2 = MockDevice::mk2();
    assert_eq!(mk2.info().key_count, 15);
}

/// Test mixed operations on multiple keys.
#[test]
fn test_mixed_key_operations() {
    let mock = MockDevice::xl();

    // Mix of operations
    mock.fill_key_color(0, (255, 0, 0)).unwrap();
    mock.set_key_image(1, Path::new("/tmp/icon.png"), ResizeStrategy::Fit)
        .unwrap();
    mock.clear_key(2).unwrap();
    mock.fill_key_color(3, (0, 255, 0)).unwrap();

    // Verify final states
    mock.assert_key_color(0, 255, 0, 0);
    mock.assert_key_has_image(1);
    mock.assert_key_cleared(2);
    mock.assert_key_color(3, 0, 255, 0);
}

/// Test state persists across operations.
#[test]
fn test_state_persistence() {
    let mock = MockDevice::xl();

    mock.fill_key_color(0, (255, 0, 0)).unwrap();
    mock.set_brightness(50).unwrap();

    // State should persist
    mock.assert_key_color(0, 255, 0, 0);
    assert_eq!(mock.get_brightness(), 50);

    // More operations
    mock.fill_key_color(1, (0, 255, 0)).unwrap();

    // Previous state still there
    mock.assert_key_color(0, 255, 0, 0);
    mock.assert_key_color(1, 0, 255, 0);
}

/// Test that watch_buttons is recorded.
#[test]
fn test_watch_buttons_recording() {
    let mock = MockDevice::xl();

    mock.watch_buttons(true, false, 5).unwrap();

    mock.assert_contains(&Operation::WatchButtons {
        json_output: true,
        once: false,
        timeout_secs: 5,
    });
}
