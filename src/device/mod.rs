//! Device abstraction layer for Stream Deck devices.
//!
//! This module provides a trait-based abstraction over real and mock
//! Stream Deck implementations, enabling testability without hardware.

mod info;
pub mod mock;
mod real;

pub use info::{ButtonEvent, ConnectionOptions, DeviceInfo, DeviceModel};
pub use real::{
    clear_all_keys, clear_key, fill_all_keys_color, fill_key_color, get_device_info, list_devices,
    open_device, open_device_with_retry, read_button_states, set_brightness, set_key_image,
    watch_buttons, Device,
};

use std::path::Path;

use crate::error::Result;

/// Core device operations trait.
///
/// This trait abstracts over real hardware and mock implementations,
/// enabling unit testing without physical devices.
///
/// # Implementation Notes
///
/// - All operations that modify device state should call `flush()` internally
/// - Key indices are 0-based, left-to-right, top-to-bottom
/// - Images should be resized to match the device's key dimensions
#[allow(dead_code)]
pub trait DeviceOperations {
    /// Get device information.
    fn info(&self) -> &DeviceInfo;

    /// Check if the device is still connected.
    fn is_connected(&self) -> bool;

    /// Get device serial number.
    fn serial(&self) -> &str {
        &self.info().serial
    }

    /// Set display brightness (0-100).
    ///
    /// # Errors
    ///
    /// Returns an error if the brightness level is invalid or
    /// if there's a communication failure.
    fn set_brightness(&self, level: u8) -> Result<()>;

    /// Set a key's image from a file path.
    ///
    /// The image will be loaded, converted, and resized to match
    /// the device's key dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if the key index is out of range, the file
    /// doesn't exist, the image format is unsupported, or there's
    /// a communication failure.
    fn set_key_image(&self, key: u8, path: &Path) -> Result<()>;

    /// Clear a single key (set to black).
    ///
    /// # Errors
    ///
    /// Returns an error if the key index is out of range or
    /// there's a communication failure.
    fn clear_key(&self, key: u8) -> Result<()>;

    /// Clear all keys (set to black).
    ///
    /// # Errors
    ///
    /// Returns an error if there's a communication failure.
    fn clear_all_keys(&self) -> Result<()>;

    /// Fill a key with a solid color.
    ///
    /// # Arguments
    ///
    /// * `key` - Key index (0-based)
    /// * `color` - RGB color tuple (r, g, b)
    ///
    /// # Errors
    ///
    /// Returns an error if the key index is out of range or
    /// there's a communication failure.
    fn fill_key_color(&self, key: u8, color: (u8, u8, u8)) -> Result<()>;

    /// Fill all keys with a solid color.
    ///
    /// # Errors
    ///
    /// Returns an error if there's a communication failure.
    fn fill_all_keys_color(&self, color: (u8, u8, u8)) -> Result<()>;

    /// Read button states (non-blocking with timeout).
    ///
    /// Returns a vector of booleans where each index corresponds
    /// to a key, and the value indicates if the key is pressed.
    fn read_button_states(&self) -> Vec<bool>;

    /// Watch for button presses and invoke callback.
    ///
    /// This is a blocking operation that will loop until:
    /// - `once` is true and a button is pressed
    /// - The timeout expires (if non-zero)
    /// - The caller interrupts
    ///
    /// # Arguments
    ///
    /// * `json_output` - If true, output JSON format
    /// * `once` - If true, exit after first button press
    /// * `timeout_secs` - Timeout in seconds (0 = no timeout)
    ///
    /// # Errors
    ///
    /// Returns an error if there's a communication failure.
    fn watch_buttons(&self, json_output: bool, once: bool, timeout_secs: u64) -> Result<()>;
}

/// Type alias for boxed trait object.
#[allow(dead_code)]
pub type BoxedDevice = Box<dyn DeviceOperations>;

/// Open a device and return it as a boxed trait object.
///
/// This function provides a convenient way to get a `BoxedDevice`
/// that can be used with dependency injection for testing.
///
/// # Errors
///
/// Returns an error if no devices are found, the specified device
/// isn't connected, or there's a connection failure.
#[allow(dead_code)]
pub fn open_boxed_device(serial: Option<&str>) -> Result<BoxedDevice> {
    Ok(Box::new(open_device(serial)?))
}

/// Open a device with retry options and return it as a boxed trait object.
///
/// # Errors
///
/// Returns an error if all retry attempts fail.
#[allow(dead_code)]
pub fn open_boxed_device_with_retry(
    serial: Option<&str>,
    opts: &ConnectionOptions,
) -> Result<BoxedDevice> {
    Ok(Box::new(open_device_with_retry(serial, opts)?))
}
