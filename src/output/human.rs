//! Human-friendly output implementation using rich_rust.
#![allow(dead_code)]

use std::path::Path;

use rich_rust::prelude::*;
use tracing::{debug, instrument, trace};

use crate::device::{ButtonEvent, DeviceInfo};
use crate::error::SdError;
use crate::theme::SdTheme;

use super::Output;

/// Styled terminal output implementation for human users.
pub struct HumanOutput {
    console: Console,
    theme: SdTheme,
}

impl HumanOutput {
    #[instrument(skip(console))]
    pub fn new(console: Console) -> Self {
        debug!("Creating HumanOutput");
        Self {
            console,
            theme: SdTheme::default(),
        }
    }

    fn width(&self) -> usize {
        self.console.width()
    }

    #[instrument(skip(self, content))]
    fn accent_panel<'a>(&self, content: &'a str, title: Option<&str>) -> Panel<'a> {
        trace!(title, "Creating accent panel");
        let mut panel = Panel::from_text(content)
            .border_style(Style::new().color(self.theme.accent.clone()))
            .box_style(self.theme.box_style);
        if let Some(t) = title {
            panel = panel.title(t);
        }
        panel
    }

    #[instrument(skip(self, content))]
    fn error_panel<'a>(&self, content: &'a Text, title: Option<&str>) -> Panel<'a> {
        trace!(title, "Creating error panel");
        let width = self.width().saturating_sub(4);
        let mut panel = Panel::from_rich_text(content, width)
            .border_style(Style::new().color(self.theme.error.clone()))
            .box_style(self.theme.box_style);
        if let Some(t) = title {
            panel = panel.title(t);
        }
        panel
    }

    fn success_panel<'a>(&self, content: &'a str) -> Panel<'a> {
        Panel::from_text(content)
            .border_style(Style::new().color(self.theme.success.clone()))
            .box_style(self.theme.box_style)
    }
}

impl Output for HumanOutput {
    #[instrument(skip(self))]
    fn success(&self, message: &str) {
        debug!(message, "Outputting success");
        let mut text = Text::new("");
        text.append_styled(
            "[OK] ",
            Style::new().bold().color(self.theme.success.clone()),
        );
        text.append(message);
        self.console.print_text(&text);
    }

    #[instrument(skip(self))]
    fn error(&self, error: &SdError) {
        debug!(
            error = %error,
            recoverable = error.is_user_recoverable(),
            "Outputting error"
        );
        let mut content = Text::new("\n");
        content.append("  ");
        content.append_styled(
            "[ERR] ",
            Style::new().bold().color(self.theme.error.clone()),
        );
        content.append_styled(&error.to_string(), Style::new().bold());
        content.append("\n");

        if let SdError::MultipleDevices { serials } = error {
            if !serials.is_empty() {
                content.append("\n");
                content.append_styled("  Available devices:\n", self.theme.label.clone());
                for serial in serials {
                    content.append_styled(
                        &format!("    - {serial}\n"),
                        self.theme.device_serial.clone(),
                    );
                }
            }
        }

        if let Some(suggestion) = error.suggestion() {
            trace!(suggestion, "Adding suggestion");
            content.append("\n");
            content.append_styled("  Suggestion:\n", self.theme.label.clone());
            content.append_styled(
                &format!("  {suggestion}"),
                Style::new().color(self.theme.muted.clone()),
            );
            content.append("\n");
        }

        content.append("\n");

        let panel = self.error_panel(&content, Some("Error"));
        self.console.print_renderable(&panel);
    }

    #[instrument(skip(self))]
    fn warning(&self, message: &str) {
        debug!(message, "Outputting warning");
        let mut text = Text::new("");
        text.append_styled(
            "[WARN] ",
            Style::new().bold().color(self.theme.warning.clone()),
        );
        text.append(message);
        self.console.print_text(&text);
    }

    #[instrument(skip(self))]
    fn info(&self, message: &str) {
        debug!(message, "Outputting info");
        let mut text = Text::new("");
        text.append_styled(
            "[INFO] ",
            Style::new().bold().color(self.theme.accent.clone()),
        );
        text.append(message);
        self.console.print_text(&text);
    }

    #[instrument(skip(self, devices), fields(device_count = devices.len()))]
    fn device_list(&self, devices: &[DeviceInfo]) {
        debug!("Outputting device list");
        if devices.is_empty() {
            trace!("No devices - showing warning");
            self.warning("No Stream Deck devices found");
            return;
        }

        let panel = self.accent_panel("Connected Devices:", None);
        self.console.print_renderable(&panel);
        for (idx, device) in devices.iter().enumerate() {
            trace!(index = idx, serial = %device.serial, "Listing device");
            self.console
                .print(&format!("  {} ({})", device.product_name, device.serial));
        }
    }

    #[instrument(skip(self, info), fields(serial = %info.serial))]
    fn device_info(&self, info: &DeviceInfo) {
        debug!("Outputting device info");
        self.console
            .print_styled(&info.product_name, self.theme.header.clone());
        self.console.print(&format!("  Serial: {}", info.serial));
        self.console
            .print(&format!("  Firmware: {}", info.firmware_version));
        self.console.print(&format!(
            "  Keys: {} ({}x{})",
            info.key_count, info.cols, info.rows
        ));
    }

    #[instrument(skip(self, event), fields(key = event.key, pressed = event.pressed))]
    fn button_event(&self, event: &ButtonEvent) {
        trace!("Outputting button event");
        let action = if event.pressed { "pressed" } else { "released" };
        self.console
            .print(&format!("  Key {} {}", event.key, action));
    }

    #[instrument(skip(self, states), fields(count = states.len()))]
    fn button_states(&self, states: &[bool]) {
        let pressed: Vec<_> = states
            .iter()
            .enumerate()
            .filter(|&(_, &pressed)| pressed)
            .map(|(i, _)| i.to_string())
            .collect();

        trace!(pressed_count = pressed.len(), "Outputting button states");

        if pressed.is_empty() {
            self.console.print("  No keys pressed");
        } else {
            self.console
                .print(&format!("  Keys pressed: {}", pressed.join(", ")));
        }
    }

    #[instrument(skip(self))]
    fn brightness_set(&self, level: u8) {
        debug!(level, "Outputting brightness set");
        self.success(&format!("Brightness set to {level}%"));
    }

    #[instrument(skip(self, image))]
    fn key_set(&self, key: u8, image: &Path) {
        let filename = image
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| image.display().to_string());
        debug!(key, filename = %filename, "Outputting key set");
        self.success(&format!("Key {key} set to {filename}"));
    }

    #[instrument(skip(self))]
    fn key_cleared(&self, key: u8) {
        debug!(key, "Outputting key cleared");
        self.success(&format!("Key {key} cleared"));
    }

    #[instrument(skip(self))]
    fn key_filled(&self, key: u8, color: &str) {
        debug!(key, color, "Outputting key filled");
        self.success(&format!("Key {key} filled with {color}"));
    }

    #[instrument(skip(self))]
    fn all_cleared(&self) {
        debug!("Outputting all cleared");
        self.success("All keys cleared");
    }

    #[instrument(skip(self))]
    fn all_filled(&self, color: &str) {
        debug!(color, "Outputting all filled");
        self.success(&format!("All keys filled with {color}"));
    }

    #[instrument(skip(self))]
    fn version_info(&self, version: &str, git_sha: Option<&str>, build_time: Option<&str>) {
        debug!(version, ?git_sha, ?build_time, "Outputting version info");
        let mut content = Text::new("\n");
        let label = |name: &str| format!("  {:<10}", name);

        content.append_styled(&label("Version"), self.theme.label.clone());
        content.append_styled(version, self.theme.value.clone());
        content.append("\n");

        if let Some(sha) = git_sha {
            let dirty =
                sha.contains("dirty") || matches!(option_env!("VERGEN_GIT_DIRTY"), Some("true"));
            let clean_sha = sha
                .replace(" (dirty)", "")
                .replace("(dirty)", "")
                .trim()
                .to_string();

            content.append_styled(&label("Git SHA"), self.theme.label.clone());
            content.append_styled(&clean_sha, self.theme.value.clone());
            if dirty {
                content.append_styled(" (dirty)", Style::new().color(self.theme.warning.clone()));
            }
            content.append("\n");
        }

        if let Some(time) = build_time {
            content.append_styled(&label("Built"), self.theme.label.clone());
            content.append_styled(time, Style::new().color(self.theme.muted.clone()));
            content.append("\n");
        }

        if let Some(rustc) = option_env!("VERGEN_RUSTC_SEMVER") {
            content.append_styled(&label("Rust"), self.theme.label.clone());
            content.append_styled(rustc, Style::new().color(self.theme.muted.clone()));
            content.append("\n");
        }

        if let Some(target) = option_env!("VERGEN_CARGO_TARGET_TRIPLE") {
            content.append_styled(&label("Target"), self.theme.label.clone());
            content.append_styled(target, Style::new().color(self.theme.muted.clone()));
            content.append("\n");
        }

        content.append("\n");

        let panel = Panel::from_rich_text(&content, self.width().saturating_sub(4))
            .title("sd")
            .border_style(Style::new().color(self.theme.accent.clone()))
            .box_style(self.theme.box_style);

        self.console.print_renderable(&panel);
    }

    #[instrument(skip(self))]
    fn rule(&self, title: Option<&str>) {
        trace!(?title, "Outputting rule");
        self.console.rule(title);
    }

    #[instrument(skip(self))]
    fn newline(&self) {
        self.console.print("");
    }
}
