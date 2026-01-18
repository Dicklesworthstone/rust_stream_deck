//! Device interface wrapping elgato-streamdeck crate.

use std::path::Path;
use std::time::{Duration, Instant};

use elgato_streamdeck::info::Kind;
use elgato_streamdeck::{StreamDeck, StreamDeckInput};
use serde::Serialize;

use crate::error::{Result, SdError};

/// Information about a discovered Stream Deck device.
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

/// Wrapper around the StreamDeck providing a simplified interface.
pub struct Device {
    inner: StreamDeck,
    info: DeviceInfo,
}

/// List all connected Stream Deck devices.
pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let hid = elgato_streamdeck::new_hidapi()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

    let devices = elgato_streamdeck::list_devices(&hid);

    let mut result = Vec::new();
    for (kind, serial) in devices {
        let image_format = kind.key_image_format();
        result.push(DeviceInfo {
            serial,
            product_name: kind_to_name(kind),
            firmware_version: String::new(), // Need to open device to get this
            key_count: kind.key_count(),
            key_width: image_format.size.0,
            key_height: image_format.size.1,
            rows: kind.row_count(),
            cols: kind.column_count(),
            kind: format!("{kind:?}"),
        });
    }

    Ok(result)
}

/// Open a Stream Deck device, optionally by serial number.
pub fn open_device(serial: Option<&str>) -> Result<Device> {
    let hid = elgato_streamdeck::new_hidapi()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

    let devices = elgato_streamdeck::list_devices(&hid);

    if devices.is_empty() {
        return Err(SdError::NoDevicesFound);
    }

    // Find the target device
    let (kind, target_serial) = if let Some(serial) = serial {
        devices
            .iter()
            .find(|(_, s)| s == serial)
            .cloned()
            .ok_or_else(|| SdError::DeviceNotFound {
                serial: serial.to_string(),
            })?
    } else if devices.len() == 1 {
        devices[0].clone()
    } else {
        let serials: Vec<_> = devices.iter().map(|(_, s)| s.clone()).collect();
        return Err(SdError::MultipleDevices { serials });
    };

    // Connect to the device
    let inner = StreamDeck::connect(&hid, kind, &target_serial)
        .map_err(|e| SdError::DeviceOpenFailed {
            serial: target_serial.clone(),
            reason: e.to_string(),
        })?;

    // Get firmware version
    let firmware = inner
        .firmware_version()
        .unwrap_or_else(|_| "unknown".to_string());

    let image_format = kind.key_image_format();
    let info = DeviceInfo {
        serial: target_serial,
        product_name: kind_to_name(kind),
        firmware_version: firmware,
        key_count: kind.key_count(),
        key_width: image_format.size.0,
        key_height: image_format.size.1,
        rows: kind.row_count(),
        cols: kind.column_count(),
        kind: format!("{kind:?}"),
    };

    Ok(Device { inner, info })
}

/// Get detailed device information.
pub fn get_device_info(device: &Device) -> Result<DeviceInfo> {
    Ok(device.info.clone())
}

/// Set display brightness (0-100).
pub fn set_brightness(device: &Device, level: u8) -> Result<()> {
    device
        .inner
        .set_brightness(level)
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))
}

/// Set a key's image from a file.
pub fn set_key_image(device: &Device, key: u8, path: &Path) -> Result<()> {
    if key >= device.info.key_count {
        return Err(SdError::InvalidKeyIndex {
            index: key,
            max: device.info.key_count,
            max_idx: device.info.key_count - 1,
        });
    }

    if !path.exists() {
        return Err(SdError::ImageNotFound {
            path: path.display().to_string(),
        });
    }

    let img = image::open(path).map_err(|e| SdError::ImageProcessing(e.to_string()))?;

    // Resize to key dimensions
    let resized = img.resize_exact(
        device.info.key_width as u32,
        device.info.key_height as u32,
        image::imageops::FilterType::Lanczos3,
    );

    device
        .inner
        .set_button_image(key, resized)
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

    // Flush changes to device
    device
        .inner
        .flush()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))
}

/// Clear a specific key (set to black).
pub fn clear_key(device: &Device, key: u8) -> Result<()> {
    if key >= device.info.key_count {
        return Err(SdError::InvalidKeyIndex {
            index: key,
            max: device.info.key_count,
            max_idx: device.info.key_count - 1,
        });
    }

    device
        .inner
        .clear_button_image(key)
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

    device
        .inner
        .flush()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))
}

/// Clear all keys.
pub fn clear_all_keys(device: &Device) -> Result<()> {
    device
        .inner
        .clear_all_button_images()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

    device
        .inner
        .flush()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))
}

/// Fill a key with a solid color.
pub fn fill_key_color(device: &Device, key: u8, color: (u8, u8, u8)) -> Result<()> {
    if key >= device.info.key_count {
        return Err(SdError::InvalidKeyIndex {
            index: key,
            max: device.info.key_count,
            max_idx: device.info.key_count - 1,
        });
    }

    let mut img = image::RgbImage::new(
        device.info.key_width as u32,
        device.info.key_height as u32,
    );
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb([color.0, color.1, color.2]);
    }

    device
        .inner
        .set_button_image(key, image::DynamicImage::ImageRgb8(img))
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

    device
        .inner
        .flush()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))
}

/// Fill all keys with a solid color.
pub fn fill_all_keys_color(device: &Device, color: (u8, u8, u8)) -> Result<()> {
    for key in 0..device.info.key_count {
        // Set image but don't flush yet
        let mut img = image::RgbImage::new(
            device.info.key_width as u32,
            device.info.key_height as u32,
        );
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([color.0, color.1, color.2]);
        }

        device
            .inner
            .set_button_image(key, image::DynamicImage::ImageRgb8(img))
            .map_err(|e| SdError::DeviceCommunication(e.to_string()))?;
    }

    // Flush all changes at once
    device
        .inner
        .flush()
        .map_err(|e| SdError::DeviceCommunication(e.to_string()))
}

/// Watch for button presses and print events.
pub fn watch_buttons(
    device: &Device,
    json_output: bool,
    once: bool,
    timeout_secs: u64,
) -> Result<()> {
    let start = Instant::now();
    let timeout = if timeout_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(timeout_secs))
    };

    loop {
        // Check timeout
        if let Some(t) = timeout {
            if start.elapsed() >= t {
                break;
            }
        }

        // Read input with timeout
        let read_timeout = Some(Duration::from_millis(50));
        match device.inner.read_input(read_timeout) {
            Ok(input) => {
                if let StreamDeckInput::ButtonStateChange(states) = input {
                    for (key, pressed) in states.iter().enumerate() {
                        if *pressed {
                            let event = ButtonEvent {
                                key: key as u8,
                                pressed: true,
                                timestamp_ms: start.elapsed().as_millis() as u64,
                            };

                            if json_output {
                                println!(
                                    "{}",
                                    serde_json::to_string(&event).unwrap_or_default()
                                );
                            } else {
                                println!("Key {key}: pressed");
                            }

                            if once {
                                return Ok(());
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout or no input, continue
            }
        }
    }

    Ok(())
}

/// Read current button states once.
pub fn read_button_states(device: &Device) -> Result<Vec<bool>> {
    let read_timeout = Some(Duration::from_millis(100));
    match device.inner.read_input(read_timeout) {
        Ok(input) => {
            if let StreamDeckInput::ButtonStateChange(states) = input {
                Ok(states)
            } else {
                // No button state change, return all false
                Ok(vec![false; device.info.key_count as usize])
            }
        }
        Err(_) => {
            // Timeout or error, return all false
            Ok(vec![false; device.info.key_count as usize])
        }
    }
}

/// Button press/release event.
#[derive(Debug, Clone, Serialize)]
pub struct ButtonEvent {
    pub key: u8,
    pub pressed: bool,
    pub timestamp_ms: u64,
}

/// Convert device kind to human-readable name.
fn kind_to_name(kind: Kind) -> String {
    match kind {
        Kind::Original => "Stream Deck (Original)".to_string(),
        Kind::OriginalV2 => "Stream Deck (Original V2)".to_string(),
        Kind::Mini => "Stream Deck Mini".to_string(),
        Kind::MiniMk2 => "Stream Deck Mini MK.2".to_string(),
        Kind::Xl => "Stream Deck XL".to_string(),
        Kind::XlV2 => "Stream Deck XL V2".to_string(),
        Kind::Mk2 => "Stream Deck MK.2".to_string(),
        Kind::Pedal => "Stream Deck Pedal".to_string(),
        Kind::Plus => "Stream Deck +".to_string(),
        Kind::Neo => "Stream Deck Neo".to_string(),
        _ => format!("{kind:?}"),
    }
}
