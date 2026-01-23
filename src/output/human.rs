//! Human-friendly output implementation using rich_rust.
#![allow(dead_code)]

use std::path::Path;

use rich_rust::prelude::*;
use tracing::{debug, instrument, trace};

use crate::device::{ButtonEvent, DeviceInfo};
use crate::error::SdError;
use crate::theme::SdTheme;

use super::{BatchKeyResult, BatchSummary, Output, ValidationResult};

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

    /// Generate ASCII art key layout grid for device info display.
    fn render_key_layout(&self, rows: u8, cols: u8) -> String {
        let mut grid = String::new();
        let cell_width = 2;

        // Top border
        grid.push_str("  ┌");
        for c in 0..cols {
            grid.push_str(&"─".repeat(cell_width));
            if c < cols - 1 {
                grid.push('┬');
            }
        }
        grid.push_str("┐\n");

        // Rows with content
        for r in 0..rows {
            grid.push_str("  │");
            for c in 0..cols {
                let key_num = r * cols + c;
                grid.push_str(&format!("{key_num:>2}"));
                if c < cols - 1 {
                    grid.push('│');
                }
            }
            grid.push_str("│\n");

            // Row separator (except after last row)
            if r < rows - 1 {
                grid.push_str("  ├");
                for c in 0..cols {
                    grid.push_str(&"─".repeat(cell_width));
                    if c < cols - 1 {
                        grid.push('┼');
                    }
                }
                grid.push_str("┤\n");
            }
        }

        // Bottom border
        grid.push_str("  └");
        for c in 0..cols {
            grid.push_str(&"─".repeat(cell_width));
            if c < cols - 1 {
                grid.push('┴');
            }
        }
        grid.push_str("┘\n");

        grid
    }

    /// Render a brightness bar using block characters.
    fn render_brightness_bar(&self, level: u8, width: usize) -> String {
        let filled = (usize::from(level) * width) / 100;
        let empty = width.saturating_sub(filled);
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
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
            trace!("No devices - showing warning panel");
            let mut content = Text::new("\n");
            content.append_styled(
                "  No devices found\n\n",
                Style::new().italic().color(self.theme.muted.clone()),
            );
            content.append_styled(
                "  Ensure Stream Deck is connected via USB\n",
                Style::new().color(self.theme.muted.clone()),
            );
            content.append("\n");

            let panel = Panel::from_rich_text(&content, self.width().saturating_sub(4))
                .title("Stream Deck Devices")
                .border_style(Style::new().color(self.theme.warning.clone()))
                .box_style(self.theme.box_style);

            self.console.print_renderable(&panel);
            return;
        }

        // Build content with device cards
        let mut content = Text::new("\n");

        for (i, device) in devices.iter().enumerate() {
            trace!(index = i, serial = %device.serial, "Listing device");

            // Device number and name (header style)
            content.append_styled(
                &format!("  {}. {}\n", i + 1, device.product_name),
                self.theme.header.clone(),
            );

            // Serial (indented, muted)
            content.append_styled("     Serial: ", self.theme.label.clone());
            content.append_styled(&format!("{}\n", device.serial), self.theme.device_serial.clone());

            // Keys and firmware on same line
            content.append_styled("     Keys: ", self.theme.label.clone());
            content.append_styled(
                &format!("{} ({}×{})", device.key_count, device.cols, device.rows),
                self.theme.value.clone(),
            );
            content.append_styled("  │  ", Style::new().color(self.theme.muted.clone()));
            content.append_styled("Firmware: ", self.theme.label.clone());
            content.append_styled(&device.firmware_version, self.theme.value.clone());

            // Add spacing between devices (except last)
            if i < devices.len() - 1 {
                content.append("\n\n");
            } else {
                content.append("\n\n");
            }
        }

        let panel = Panel::from_rich_text(&content, self.width().saturating_sub(4))
            .title("Stream Deck Devices")
            .border_style(Style::new().color(self.theme.accent.clone()))
            .box_style(self.theme.box_style);

        self.console.print_renderable(&panel);
    }

    #[instrument(skip(self, info), fields(serial = %info.serial))]
    fn device_info(&self, info: &DeviceInfo) {
        debug!("Outputting device info");

        // Build specification display
        let mut content = Text::new("\n");

        // Serial
        content.append_styled("  Serial      ", self.theme.label.clone());
        content.append_styled(&info.serial, self.theme.device_serial.clone());
        content.append("\n");

        // Firmware
        content.append_styled("  Firmware    ", self.theme.label.clone());
        content.append_styled(&info.firmware_version, self.theme.value.clone());
        content.append("\n");

        // Keys
        content.append_styled("  Keys        ", self.theme.label.clone());
        content.append_styled(
            &format!("{} ({} columns × {} rows)", info.key_count, info.cols, info.rows),
            self.theme.value.clone(),
        );
        content.append("\n");

        // Key size
        content.append_styled("  Key Size    ", self.theme.label.clone());
        content.append_styled(
            &format!("{}×{} pixels", info.key_width, info.key_height),
            self.theme.value.clone(),
        );
        content.append("\n");

        // Device type
        content.append_styled("  Type        ", self.theme.label.clone());
        content.append_styled(&info.kind, self.theme.value.clone());
        content.append("\n\n");

        // Key layout grid
        content.append_styled("  Key Layout:\n", self.theme.label.clone());
        let key_layout = self.render_key_layout(info.rows, info.cols);
        content.append_styled(&key_layout, self.theme.key_index.clone());
        content.append("\n");

        let panel = Panel::from_rich_text(&content, self.width().saturating_sub(4))
            .title(info.product_name.as_str())
            .border_style(Style::new().color(self.theme.accent.clone()))
            .box_style(self.theme.box_style);

        self.console.print_renderable(&panel);
    }

    #[instrument(skip(self, event), fields(key = event.key, pressed = event.pressed))]
    fn button_event(&self, event: &ButtonEvent) {
        trace!("Outputting button event");
        let mut text = Text::new("  ");

        // Key number with styled color
        text.append_styled(&format!("Key {:>2}", event.key), self.theme.key_index.clone());
        text.append(" ");

        // Action with appropriate color
        if event.pressed {
            text.append_styled("▼ pressed ", self.theme.button_pressed.clone());
        } else {
            text.append_styled("▲ released", self.theme.button_released.clone());
        }

        // Timestamp (muted)
        text.append_styled(
            &format!("  [{}ms]", event.timestamp_ms),
            Style::new().color(self.theme.muted.clone()),
        );

        self.console.print_text(&text);
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

        // Calculate bar width (leave room for label and percentage)
        // "  Brightness: " = 14 chars, " XXX%" = 5 chars, padding = 4
        let bar_width = self.width().saturating_sub(27).min(40);

        // Build content with label and progress bar
        let mut content = Text::new("");
        content.append_styled("  Brightness: ", self.theme.label.clone());
        let bar_str = self.render_brightness_bar(level, bar_width);
        content.append_styled(&bar_str, self.theme.brightness.clone());
        content.append_styled(&format!(" {:>3}%  ", level), self.theme.value.clone());

        let panel = Panel::from_rich_text(&content, self.width().saturating_sub(4))
            .border_style(Style::new().color(self.theme.success.clone()))
            .box_style(self.theme.box_style);

        self.console.print_renderable(&panel);
    }

    #[instrument(skip(self, image))]
    fn key_set(&self, key: u8, image: &Path) {
        let filename = image
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| image.display().to_string());
        debug!(key, filename = %filename, "Outputting key set");

        let mut text = Text::new("");
        text.append_styled("✓ ", Style::new().bold().color(self.theme.success.clone()));
        text.append("Key ");
        text.append_styled(&format!("{key}"), self.theme.key_index.clone());
        text.append(" set to ");
        text.append_styled(&filename, self.theme.value.clone());
        self.console.print_text(&text);
    }

    #[instrument(skip(self))]
    fn key_cleared(&self, key: u8) {
        debug!(key, "Outputting key cleared");

        let mut text = Text::new("");
        text.append_styled("✓ ", Style::new().bold().color(self.theme.success.clone()));
        text.append("Key ");
        text.append_styled(&format!("{key}"), self.theme.key_index.clone());
        text.append(" cleared");
        self.console.print_text(&text);
    }

    #[instrument(skip(self))]
    fn key_filled(&self, key: u8, color: &str) {
        debug!(key, color, "Outputting key filled");

        let mut text = Text::new("");
        text.append_styled("✓ ", Style::new().bold().color(self.theme.success.clone()));
        text.append("Key ");
        text.append_styled(&format!("{key}"), self.theme.key_index.clone());
        text.append(" filled with ");
        text.append_styled(color, self.theme.value.clone());
        self.console.print_text(&text);
    }

    #[instrument(skip(self))]
    fn all_cleared(&self) {
        debug!("Outputting all cleared");

        let mut text = Text::new("");
        text.append_styled("✓ ", Style::new().bold().color(self.theme.success.clone()));
        text.append("All keys cleared");
        self.console.print_text(&text);
    }

    #[instrument(skip(self))]
    fn all_filled(&self, color: &str) {
        debug!(color, "Outputting all filled");

        let mut text = Text::new("");
        text.append_styled("✓ ", Style::new().bold().color(self.theme.success.clone()));
        text.append("All keys filled with ");
        text.append_styled(color, self.theme.value.clone());
        self.console.print_text(&text);
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

    #[instrument(skip(self, results, summary), fields(total = summary.total, success = summary.success))]
    fn batch_set_keys(&self, results: &[BatchKeyResult], summary: &BatchSummary) {
        debug!("Outputting batch set-keys results");

        // Show per-key results
        for result in results {
            if result.ok {
                if let Some(ref path) = result.path {
                    let filename = Path::new(path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.clone());
                    self.console
                        .print(&format!("  Key {}: {}", result.key, filename));
                }
            } else if let Some(ref err) = result.error {
                let mut text = Text::new("");
                text.append_styled(
                    &format!("  Key {}: ", result.key),
                    Style::new().color(self.theme.error.clone()),
                );
                text.append_styled(err, Style::new().color(self.theme.muted.clone()));
                self.console.print_text(&text);
            }
        }

        // Show summary
        if summary.failed == 0 {
            self.success(&format!("Set {} keys", summary.success));
        } else {
            self.warning(&format!(
                "Set {} keys ({} errors)",
                summary.success, summary.failed
            ));
        }
    }

    #[instrument(skip(self, results, summary), fields(total = summary.total, success = summary.success))]
    fn batch_fill_keys(&self, color: &str, results: &[BatchKeyResult], summary: &BatchSummary) {
        debug!(color, "Outputting batch fill-keys results");

        // Show per-key results
        for result in results {
            if result.ok {
                self.console
                    .print(&format!("  Key {}: filled with {}", result.key, color));
            } else if let Some(ref err) = result.error {
                let mut text = Text::new("");
                text.append_styled(
                    &format!("  Key {}: ", result.key),
                    Style::new().color(self.theme.error.clone()),
                );
                text.append_styled(err, Style::new().color(self.theme.muted.clone()));
                self.console.print_text(&text);
            }
        }

        // Show summary
        if summary.failed == 0 {
            self.success(&format!("Filled {} keys with {}", summary.success, color));
        } else {
            self.warning(&format!(
                "Filled {} keys with {} ({} errors)",
                summary.success, color, summary.failed
            ));
        }
    }

    #[instrument(skip(self, results, summary), fields(total = summary.total, success = summary.success))]
    fn batch_clear_keys(&self, results: &[BatchKeyResult], summary: &BatchSummary) {
        debug!("Outputting batch clear-keys results");

        // Show per-key results
        for result in results {
            if result.ok {
                self.console
                    .print(&format!("  Key {}: cleared", result.key));
            } else if let Some(ref err) = result.error {
                let mut text = Text::new("");
                text.append_styled(
                    &format!("  Key {}: ", result.key),
                    Style::new().color(self.theme.error.clone()),
                );
                text.append_styled(err, Style::new().color(self.theme.muted.clone()));
                self.console.print_text(&text);
            }
        }

        // Show summary
        if summary.failed == 0 {
            self.success(&format!("Cleared {} keys", summary.success));
        } else {
            self.warning(&format!(
                "Cleared {} keys ({} errors)",
                summary.success, summary.failed
            ));
        }
    }

    #[instrument(skip(self, result), fields(valid = result.valid, errors = result.summary.error_count))]
    fn validation_result(&self, result: &ValidationResult) {
        debug!("Outputting validation result");

        let mut content = Text::new("\n");

        // Config file path
        content.append_styled("  Config: ", self.theme.label.clone());
        content.append_styled(&result.config_path, self.theme.value.clone());
        content.append("\n");

        // Profile name if present
        if let Some(ref name) = result.config_name {
            content.append_styled("  Name:   ", self.theme.label.clone());
            content.append_styled(name, self.theme.value.clone());
            content.append("\n");
        }

        // Summary stats
        if let Some(key_count) = result.summary.key_count {
            content.append_styled("  Keys:   ", self.theme.label.clone());
            content.append_styled(&key_count.to_string(), self.theme.value.clone());
            content.append("\n");
        }
        if let Some(brightness) = result.summary.brightness {
            content.append_styled("  Brightness: ", self.theme.label.clone());
            content.append_styled(&format!("{}%", brightness), self.theme.value.clone());
            content.append("\n");
        }

        content.append("\n");

        // Show errors
        let errors = result.errors();
        if !errors.is_empty() {
            content.append_styled("  ERRORS:\n", Style::new().bold().color(self.theme.error.clone()));
            for issue in errors {
                content.append_styled("    ✗ ", Style::new().color(self.theme.error.clone()));
                content.append_styled(&issue.field, self.theme.label.clone());
                content.append(": ");
                content.append(&issue.message);
                content.append("\n");
                if let Some(ref suggestion) = issue.suggestion {
                    content.append_styled(
                        &format!("      Suggestion: {}\n", suggestion),
                        Style::new().color(self.theme.muted.clone()),
                    );
                }
            }
            content.append("\n");
        }

        // Show warnings
        let warnings = result.warnings();
        if !warnings.is_empty() {
            content.append_styled("  WARNINGS:\n", Style::new().bold().color(self.theme.warning.clone()));
            for issue in warnings {
                content.append_styled("    ⚠ ", Style::new().color(self.theme.warning.clone()));
                content.append_styled(&issue.field, self.theme.label.clone());
                content.append(": ");
                content.append(&issue.message);
                content.append("\n");
                if let Some(ref suggestion) = issue.suggestion {
                    content.append_styled(
                        &format!("      Suggestion: {}\n", suggestion),
                        Style::new().color(self.theme.muted.clone()),
                    );
                }
            }
            content.append("\n");
        }

        // Final status line
        if result.is_valid() {
            content.append_styled(
                "  ✓ Configuration is valid",
                Style::new().bold().color(self.theme.success.clone()),
            );
            if result.summary.warning_count > 0 {
                content.append_styled(
                    &format!(" ({} warning{})",
                        result.summary.warning_count,
                        if result.summary.warning_count == 1 { "" } else { "s" }
                    ),
                    Style::new().color(self.theme.warning.clone()),
                );
            }
        } else {
            content.append_styled(
                &format!("  ✗ Configuration has {} error{}",
                    result.summary.error_count,
                    if result.summary.error_count == 1 { "" } else { "s" }
                ),
                Style::new().bold().color(self.theme.error.clone()),
            );
            if result.summary.warning_count > 0 {
                content.append_styled(
                    &format!(" and {} warning{}",
                        result.summary.warning_count,
                        if result.summary.warning_count == 1 { "" } else { "s" }
                    ),
                    Style::new().color(self.theme.warning.clone()),
                );
            }
        }
        content.append("\n\n");

        // Choose panel border color based on result
        let border_color = if result.is_valid() {
            self.theme.success.clone()
        } else {
            self.theme.error.clone()
        };

        let panel = Panel::from_rich_text(&content, self.width().saturating_sub(4))
            .title("Validation")
            .border_style(Style::new().color(border_color))
            .box_style(self.theme.box_style);

        self.console.print_renderable(&panel);
    }
}
