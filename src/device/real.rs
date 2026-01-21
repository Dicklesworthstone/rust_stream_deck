//! Real Stream Deck device implementation.
//!
//! This module wraps the `elgato-streamdeck` crate to provide
//! the concrete device implementation.

use std::path::Path;
use std::time::{Duration, Instant};

use elgato_streamdeck::info::Kind;
use elgato_streamdeck::{StreamDeck, StreamDeckInput};
use tracing::{debug, error, info, trace, warn};

use super::DeviceOperations;
use super::info::{ButtonEvent, ConnectionOptions, DeviceInfo};
use crate::error::{Result, SdError};

/// Real Stream Deck device wrapper.
pub struct Device {
    inner: StreamDeck,
    info: DeviceInfo,
}

impl DeviceOperations for Device {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn is_connected(&self) -> bool {
        // The elgato-streamdeck crate doesn't expose a direct "is_connected" check,
        // but we can try a simple operation. For now, assume connected if we have a device.
        true
    }

    fn set_brightness(&self, level: u8) -> Result<()> {
        set_brightness(self, level)
    }

    fn set_key_image(&self, key: u8, path: &Path) -> Result<()> {
        set_key_image(self, key, path)
    }

    fn clear_key(&self, key: u8) -> Result<()> {
        clear_key(self, key)
    }

    fn clear_all_keys(&self) -> Result<()> {
        clear_all_keys(self)
    }

    fn fill_key_color(&self, key: u8, color: (u8, u8, u8)) -> Result<()> {
        fill_key_color(self, key, color)
    }

    fn fill_all_keys_color(&self, color: (u8, u8, u8)) -> Result<()> {
        fill_all_keys_color(self, color)
    }

    fn read_button_states(&self) -> Vec<bool> {
        read_button_states(self)
    }

    fn watch_buttons(&self, json_output: bool, once: bool, timeout_secs: u64) -> Result<()> {
        watch_buttons(self, json_output, once, timeout_secs)
    }
}

/// List all connected Stream Deck devices.
pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let hid =
        elgato_streamdeck::new_hidapi().map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

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
    let hid =
        elgato_streamdeck::new_hidapi().map_err(|e| SdError::DeviceCommunication(e.to_string()))?;

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
    let inner =
        StreamDeck::connect(&hid, kind, &target_serial).map_err(|e| SdError::DeviceOpenFailed {
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

/// Open a Stream Deck device with retry/backoff options.
#[allow(dead_code)]
pub fn open_device_with_retry(serial: Option<&str>, opts: &ConnectionOptions) -> Result<Device> {
    let max_retries = opts.max_retries.max(1);
    let mut delay = opts.retry_delay;

    for attempt in 1..=max_retries {
        trace!(attempt, max_retries, "Retry loop iteration");
        debug!(attempt, max_retries, "Opening Stream Deck device");
        match open_device(serial) {
            Ok(device) => {
                info!(attempt, "Device connected successfully");
                return Ok(device);
            }
            Err(err) if err.is_connection_error() && attempt < max_retries => {
                warn!(
                    attempt,
                    max_retries,
                    error = %err,
                    delay_ms = delay.as_millis(),
                    "Connection failed, retrying"
                );
                std::thread::sleep(delay);
                let next_delay_secs =
                    (delay.as_secs_f32() * opts.backoff_factor).min(opts.max_delay.as_secs_f32());
                delay = Duration::from_secs_f32(next_delay_secs);
            }
            Err(err) => {
                error!(attempt, max_retries, error = %err, "Connection failed");
                return Err(err);
            }
        }
    }

    unreachable!("retry loop guarantees a return before exhaustion");
}

/// Get detailed device information.
pub fn get_device_info(device: &Device) -> DeviceInfo {
    device.info.clone()
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
    #[allow(clippy::cast_possible_truncation)] // Key dimensions are always small
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

    #[allow(clippy::cast_possible_truncation)] // Key dimensions are always small
    let mut img = image::RgbImage::new(device.info.key_width as u32, device.info.key_height as u32);
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
        #[allow(clippy::cast_possible_truncation)] // Key dimensions are always small
        let mut img =
            image::RgbImage::new(device.info.key_width as u32, device.info.key_height as u32);
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
#[allow(clippy::unnecessary_wraps)] // Consistent return type with other device functions
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
        if let Ok(StreamDeckInput::ButtonStateChange(states)) =
            device.inner.read_input(read_timeout)
        {
            for (key, pressed) in states.iter().enumerate() {
                if *pressed {
                    #[allow(clippy::cast_possible_truncation)] // Key count is always < 256
                    let event = ButtonEvent {
                        key: key as u8,
                        pressed: true,
                        timestamp_ms: start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    };

                    if json_output {
                        println!("{}", serde_json::to_string(&event).unwrap_or_default());
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

    Ok(())
}

/// Read current button states once.
pub fn read_button_states(device: &Device) -> Vec<bool> {
    let read_timeout = Some(Duration::from_millis(100));
    let default = || vec![false; device.info.key_count as usize];

    device
        .inner
        .read_input(read_timeout)
        .ok()
        .and_then(|input| {
            if let StreamDeckInput::ButtonStateChange(states) = input {
                Some(states)
            } else {
                None
            }
        })
        .unwrap_or_else(default)
}

/// Convert device kind to human-readable name.
#[allow(clippy::missing_const_for_fn)] // Returns String which requires allocation
fn kind_to_name(kind: Kind) -> String {
    match kind {
        Kind::Original => "Stream Deck (Original)",
        Kind::OriginalV2 => "Stream Deck (Original V2)",
        Kind::Mini => "Stream Deck Mini",
        Kind::MiniMk2 => "Stream Deck Mini MK.2",
        Kind::Xl => "Stream Deck XL",
        Kind::XlV2 => "Stream Deck XL V2",
        Kind::Mk2 => "Stream Deck MK.2",
        Kind::Pedal => "Stream Deck Pedal",
        Kind::Plus => "Stream Deck +",
        Kind::Neo => "Stream Deck Neo",
        _ => "Unknown Stream Deck",
    }
    .to_string()
}
