//! Mock data factories for tests.
#![allow(dead_code)]

use serde::Serialize;
use tracing::{debug, instrument, trace};

/// Mock device information (mirrors crate::device::DeviceInfo).
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub serial: String,
    pub product_name: String,
    pub firmware_version: String,
    pub key_count: u8,
    pub key_width: usize,
    pub key_height: usize,
    pub rows: u8,
    pub cols: u8,
    pub kind: String,
}

/// Mock button event (mirrors crate::device::ButtonEvent).
#[derive(Debug, Clone, Serialize)]
pub struct ButtonEvent {
    pub key: u8,
    pub pressed: bool,
    pub timestamp_ms: u64,
}

#[instrument]
pub fn mock_device_xl() -> DeviceInfo {
    debug!("Creating mock XL device");
    DeviceInfo {
        serial: "XL-TEST-0001".to_string(),
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

#[instrument]
pub fn mock_device_mini() -> DeviceInfo {
    debug!("Creating mock Mini device");
    DeviceInfo {
        serial: "MINI-TEST-0001".to_string(),
        product_name: "Stream Deck Mini".to_string(),
        firmware_version: "1.0.0".to_string(),
        key_count: 6,
        key_width: 72,
        key_height: 72,
        rows: 2,
        cols: 3,
        kind: "Mini".to_string(),
    }
}

#[instrument]
pub fn mock_device_with_serial(serial: &str) -> DeviceInfo {
    trace!(serial, "Creating mock device with custom serial");
    DeviceInfo {
        serial: serial.to_string(),
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

#[instrument]
pub fn mock_multiple_devices() -> Vec<DeviceInfo> {
    debug!("Creating mock device list");
    vec![
        mock_device_with_serial("XL-TEST-0001"),
        mock_device_with_serial("XL-TEST-0002"),
        mock_device_mini(),
    ]
}

#[instrument]
pub fn mock_button_press(key: u8) -> ButtonEvent {
    trace!(key, "Creating mock button press");
    ButtonEvent {
        key,
        pressed: true,
        timestamp_ms: 0,
    }
}

#[instrument]
pub fn mock_button_release(key: u8) -> ButtonEvent {
    trace!(key, "Creating mock button release");
    ButtonEvent {
        key,
        pressed: false,
        timestamp_ms: 0,
    }
}
