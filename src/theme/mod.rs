//! Theme system for human-mode output.
#![allow(dead_code)]

use rich_rust::r#box::ROUNDED;
use rich_rust::prelude::{BoxChars, Color, Style};

/// Visual theme for Stream Deck CLI human-mode output.
///
/// Centralizes colors and styles for consistent rendering.
pub struct SdTheme {
    // Brand colors
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub muted: Color,

    // Component styles
    pub header: Style,
    pub label: Style,
    pub value: Style,
    pub key_index: Style,
    pub device_serial: Style,
    pub brightness: Style,
    pub button_pressed: Style,
    pub button_released: Style,

    // Box drawing
    pub box_style: &'static BoxChars,
}

impl Default for SdTheme {
    fn default() -> Self {
        let color = |hex: &str| Color::parse(hex).expect("invalid theme color");

        Self {
            accent: color("#0080FF"),
            success: color("#00D26A"),
            error: color("#FF4757"),
            warning: color("#FFA502"),
            muted: color("#747D8C"),
            header: Style::new().bold().color(color("#0080FF")),
            label: Style::new().dim(),
            value: Style::new().bold(),
            key_index: Style::new().bold().color(color("#FFA502")),
            device_serial: Style::new().italic().color(color("#747D8C")),
            brightness: Style::new().bold().color(color("#00D26A")),
            button_pressed: Style::new().bold().color(color("#00D26A")),
            button_released: Style::new().dim(),
            box_style: &ROUNDED,
        }
    }
}
