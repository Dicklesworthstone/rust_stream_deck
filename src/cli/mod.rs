//! CLI argument definitions and command dispatch.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

/// Stream Deck CLI - Cross-platform control for Elgato Stream Deck devices.
///
/// Robot Mode: Use --robot or --json for machine-parseable output optimized for AI agents.
#[derive(Parser, Debug)]
#[command(name = "sd", version, about, long_about = None)]
#[command(propagate_version = true)]
#[allow(clippy::struct_excessive_bools)] // CLI flags naturally use multiple bools
pub struct Cli {
    /// Output format (text for humans, json for agents/scripts)
    #[arg(
        long,
        short = 'f',
        default_value = "text",
        global = true,
        env = "SD_FORMAT"
    )]
    pub format: OutputFormat,

    /// Robot mode: equivalent to --format=json (optimized for AI agents)
    #[arg(long, global = true)]
    pub robot: bool,

    /// Verbose output (-v = debug, -vv = trace)
    #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Quiet mode (suppress non-essential output)
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Preview what would happen without making changes
    #[arg(long, short = 'n', global = true)]
    pub dry_run: bool,

    /// Target device by serial number (required if multiple devices connected)
    #[arg(long, short = 's', global = true, env = "SD_SERIAL")]
    pub serial: Option<String>,

    /// Retry N times on connection failure (default: 0 = no retry)
    #[arg(long, global = true, default_value = "0", env = "SD_RETRY")]
    pub retry: u32,

    /// Initial delay between retries in milliseconds (default: 1000)
    #[arg(long, global = true, default_value = "1000", env = "SD_RETRY_DELAY")]
    pub retry_delay: u64,

    /// Maximum delay cap for exponential backoff in milliseconds (default: 10000)
    #[arg(
        long,
        global = true,
        default_value = "10000",
        env = "SD_RETRY_MAX_DELAY"
    )]
    pub retry_max_delay: u64,

    /// Backoff multiplier for retry delay (default: 1.5)
    #[arg(long, global = true, default_value = "1.5", env = "SD_RETRY_BACKOFF")]
    pub retry_backoff: f32,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Output format selection.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text with optional color
    #[default]
    Text,
    /// JSON output for scripts and agents
    Json,
    /// Compact JSON (single line)
    JsonCompact,
}

impl Cli {
    /// Returns true if output should be JSON (robot mode or explicit --format=json).
    pub const fn use_json(&self) -> bool {
        self.robot || matches!(self.format, OutputFormat::Json | OutputFormat::JsonCompact)
    }

    /// Returns true if output should be compact JSON.
    pub const fn use_compact_json(&self) -> bool {
        matches!(self.format, OutputFormat::JsonCompact)
    }

    /// Returns true if retry is enabled.
    pub const fn retry_enabled(&self) -> bool {
        self.retry > 0
    }

    /// Returns true if dry-run mode is enabled.
    ///
    /// When dry-run is enabled, commands should show what would happen
    /// without actually making changes to the device.
    pub const fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Build connection options from CLI flags.
    ///
    /// When retry is 0, returns options for a single attempt.
    /// When retry > 0, returns options for N retries with exponential backoff.
    #[must_use]
    pub fn connection_options(&self) -> ConnectionOptions {
        if self.retry == 0 {
            // No retry - single attempt
            ConnectionOptions {
                max_retries: 1,
                retry_delay: Duration::ZERO,
                backoff_factor: 1.0,
                max_delay: Duration::ZERO,
            }
        } else {
            ConnectionOptions {
                max_retries: self.retry,
                retry_delay: Duration::from_millis(self.retry_delay),
                backoff_factor: self.retry_backoff,
                max_delay: Duration::from_millis(self.retry_max_delay),
            }
        }
    }
}

/// Re-export ConnectionOptions from device module for convenience.
pub use crate::device::ConnectionOptions;

/// Available commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    // === Device Discovery & Info ===
    /// List connected Stream Deck devices
    List(ListArgs),

    /// Show detailed device information
    Info(InfoArgs),

    // === Display Control ===
    /// Set display brightness (0-100)
    Brightness(BrightnessArgs),

    /// Set a key's image
    SetKey(SetKeyArgs),

    /// Set multiple keys from a directory of images
    #[command(visible_alias = "batch")]
    SetKeys(SetKeysArgs),

    /// Clear a key (set to black)
    ClearKey(ClearKeyArgs),

    /// Clear all keys
    ClearAll(ClearAllArgs),

    /// Fill a key with a solid color
    FillKey(FillKeyArgs),

    /// Fill all keys with a solid color
    FillAll(FillAllArgs),

    /// Fill multiple specific keys with a solid color
    FillKeys(FillKeysArgs),

    /// Clear multiple specific keys (set to black)
    ClearKeys(ClearKeysArgs),

    // === Input Monitoring ===
    /// Watch for button presses (streams events)
    Watch(WatchArgs),

    /// Read current button states once
    Read(ReadArgs),

    // === Configuration ===
    /// Initialize configuration directory
    Init(InitArgs),

    /// Show current configuration
    Config(ConfigArgs),

    // === Snapshots ===
    /// Save current device state as a named snapshot
    Save(SaveArgs),

    /// Restore a saved snapshot to the device
    Restore(RestoreArgs),

    /// List all saved snapshots
    Snapshots(SnapshotsArgs),

    /// Manage snapshots (show, delete)
    Snapshot(SnapshotCommand),

    // === Web Interface ===
    /// Start local web server for GUI control
    Serve(ServeArgs),

    // === Utilities ===
    /// Show version and build information
    Version,

    /// Generate shell completions
    Completions(CompletionsArgs),
}

// === Argument Structs ===

#[derive(Parser, Debug)]
pub struct ListArgs {
    /// Show extended device information
    #[arg(long, short = 'l')]
    pub long: bool,
}

#[derive(Parser, Debug)]
pub struct InfoArgs {
    /// Show all available fields
    #[arg(long, short = 'a')]
    pub all: bool,
}

#[derive(Parser, Debug)]
pub struct BrightnessArgs {
    /// Brightness level (0-100)
    pub level: u8,
}

#[derive(Parser, Debug)]
pub struct SetKeyArgs {
    /// Key index (0-based, left-to-right, top-to-bottom)
    pub key: u8,

    /// Path to image file (PNG, JPEG, BMP, GIF)
    pub image: PathBuf,

    /// Resize strategy if image doesn't match key size
    #[arg(long, default_value = "fit")]
    pub resize: ResizeStrategy,
}

use crate::image_ops::ResizeStrategy;

/// Arguments for batch key setting from a directory.
///
/// # Examples
///
/// ```bash
/// # Set all keys from directory
/// sd set-keys ~/my-layout/
///
/// # Use custom naming pattern
/// sd set-keys ~/icons/ --pattern "icon-{index:02d}.png"
///
/// # Only set first row (keys 0-7 on XL)
/// sd set-keys ~/row1/ --key-range 0-7
///
/// # Preview changes first
/// sd set-keys ~/layout/ --dry-run
/// ```
#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)] // CLI flags naturally use multiple bools
pub struct SetKeysArgs {
    /// Directory containing key images
    #[arg(value_name = "DIR")]
    pub dir: PathBuf,

    /// Filename pattern with {index} placeholder.
    /// Supports: {index} (0,1,2...), {index:02d} (00,01,02...)
    #[arg(long, short = 'p', default_value = "key-{index}.png")]
    pub pattern: String,

    /// Continue setting other keys if one fails
    #[arg(long, short = 'c')]
    pub continue_on_error: bool,

    /// Starting key index (for partial layouts)
    #[arg(long, default_value = "0")]
    pub start_key: u8,

    /// Only process keys in this range (e.g., "0-7" for first row)
    #[arg(long)]
    pub key_range: Option<String>,

    /// Dry run - show what would happen without applying
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Clear keys that don't have matching images
    #[arg(long)]
    pub clear_missing: bool,

    /// Skip keys that already have the same image (compare by hash)
    #[arg(long)]
    pub skip_unchanged: bool,

    /// Resize strategy for images
    #[arg(long, default_value = "fit")]
    pub resize: ResizeStrategy,
}

#[derive(Parser, Debug)]
pub struct ClearKeyArgs {
    /// Key index to clear
    pub key: u8,
}

#[derive(Parser, Debug)]
pub struct ClearAllArgs {}

#[derive(Parser, Debug)]
pub struct FillKeyArgs {
    /// Key index
    pub key: u8,

    /// Color in hex format (e.g., "ff0000" for red, "#00ff00" for green)
    pub color: String,
}

#[derive(Parser, Debug)]
pub struct FillAllArgs {
    /// Color in hex format
    pub color: String,
}

/// Arguments for batch fill-keys command.
///
/// Fill multiple keys with a solid color in one operation.
///
/// # Examples
///
/// ```bash
/// # Fill all keys with red
/// sd fill-keys ff0000 --all
///
/// # Fill first row (keys 0-7)
/// sd fill-keys 00ff00 --range 0-7
///
/// # Fill specific keys
/// sd fill-keys 0000ff --keys 0 5 10 15
/// ```
#[derive(Parser, Debug)]
pub struct FillKeysArgs {
    /// Color in hex format (e.g., "ff0000" for red, "#00ff00" for green)
    pub color: String,

    /// Fill ALL keys on the device
    #[arg(long, conflicts_with_all = ["range", "keys"])]
    pub all: bool,

    /// Range of keys to fill (e.g., "0-7" for first row)
    #[arg(long, short = 'r')]
    pub range: Option<String>,

    /// Specific key indices to fill (space-separated)
    #[arg(long, short = 'k', num_args = 1..)]
    pub keys: Vec<u8>,

    /// Continue filling other keys if one fails
    #[arg(long, short = 'c')]
    pub continue_on_error: bool,
}

/// Arguments for batch clear-keys command.
///
/// Clear multiple keys (set to black) in one operation.
///
/// # Examples
///
/// ```bash
/// # Clear all keys
/// sd clear-keys --all
///
/// # Clear first row (keys 0-7)
/// sd clear-keys --range 0-7
///
/// # Clear specific keys
/// sd clear-keys --keys 0 5 10 15
/// ```
#[derive(Parser, Debug)]
pub struct ClearKeysArgs {
    /// Clear ALL keys on the device (same as clear-all)
    #[arg(long, conflicts_with_all = ["range", "keys"])]
    pub all: bool,

    /// Range of keys to clear (e.g., "0-7" for first row)
    #[arg(long, short = 'r')]
    pub range: Option<String>,

    /// Specific key indices to clear (space-separated)
    #[arg(long, short = 'k', num_args = 1..)]
    pub keys: Vec<u8>,

    /// Continue clearing other keys if one fails
    #[arg(long, short = 'c')]
    pub continue_on_error: bool,
}

/// Arguments for the watch command.
///
/// # Examples
///
/// ```bash
/// # Watch for button presses
/// sd watch
///
/// # Exit after first press
/// sd watch --once
///
/// # Auto-reconnect on disconnect
/// sd watch --reconnect
///
/// # Custom reconnect delay (2 seconds)
/// sd watch --reconnect --reconnect-delay 2000
///
/// # Limit reconnection attempts
/// sd watch --reconnect --max-reconnect-attempts 5
/// ```
#[derive(Parser, Debug)]
pub struct WatchArgs {
    /// Exit after first button press
    #[arg(long)]
    pub once: bool,

    /// Timeout in seconds (0 = no timeout)
    #[arg(long, short = 't', default_value = "0")]
    pub timeout: u64,

    /// Automatically reconnect on device disconnect
    #[arg(long)]
    pub reconnect: bool,

    /// Initial delay between reconnection attempts in milliseconds (default: 1000)
    #[arg(long, default_value = "1000")]
    pub reconnect_delay: u64,

    /// Maximum number of reconnection attempts (0 = unlimited)
    #[arg(long, default_value = "0")]
    pub max_reconnect_attempts: u32,
}

#[derive(Parser, Debug)]
pub struct ReadArgs {}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Force overwrite existing configuration
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Show configuration file path
    #[arg(long)]
    pub path: bool,
}

/// Arguments for the save command.
///
/// # Examples
///
/// ```bash
/// # Save current state with a name
/// sd save work-mode
///
/// # Save with description
/// sd save gaming-mode -d "OBS and Discord shortcuts"
///
/// # Overwrite existing snapshot
/// sd save work-mode --force
///
/// # Save only keys modified in this session
/// sd save quick-save --session-only
/// ```
#[derive(Parser, Debug)]
pub struct SaveArgs {
    /// Name for the snapshot (alphanumeric, hyphens, underscores)
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Description for the snapshot
    #[arg(long, short = 'd')]
    pub description: Option<String>,

    /// Overwrite existing snapshot without prompting
    #[arg(long)]
    pub force: bool,

    /// Save only keys modified in this session
    #[arg(long)]
    pub session_only: bool,

    /// Exclude brightness from snapshot
    #[arg(long)]
    pub no_brightness: bool,
}

/// Arguments for the restore command.
///
/// # Examples
///
/// ```bash
/// # Restore a saved snapshot
/// sd restore work-mode
///
/// # Restore without applying brightness
/// sd restore gaming-mode --no-brightness
/// ```
#[derive(Parser, Debug)]
pub struct RestoreArgs {
    /// Name of the snapshot to restore
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Skip brightness when restoring
    #[arg(long)]
    pub no_brightness: bool,
}

/// Arguments for the snapshots list command.
///
/// # Examples
///
/// ```bash
/// # List all snapshots
/// sd snapshots
///
/// # Show detailed info
/// sd snapshots --long
/// ```
#[derive(Parser, Debug)]
pub struct SnapshotsArgs {
    /// Show detailed snapshot information
    #[arg(long, short = 'l')]
    pub long: bool,
}

/// Snapshot management subcommands.
///
/// # Examples
///
/// ```bash
/// # Show snapshot details
/// sd snapshot show work-mode
///
/// # Delete a snapshot
/// sd snapshot delete old-layout
///
/// # Force delete without confirmation
/// sd snapshot delete old-layout --force
/// ```
#[derive(Parser, Debug)]
pub struct SnapshotCommand {
    #[command(subcommand)]
    pub command: SnapshotSubcommand,
}

/// Snapshot subcommands.
#[derive(Subcommand, Debug)]
pub enum SnapshotSubcommand {
    /// Show detailed information about a snapshot
    Show(SnapshotShowArgs),

    /// Delete a snapshot
    Delete(SnapshotDeleteArgs),
}

/// Arguments for snapshot show command.
#[derive(Parser, Debug)]
pub struct SnapshotShowArgs {
    /// Name of the snapshot to show
    #[arg(value_name = "NAME")]
    pub name: String,
}

/// Arguments for snapshot delete command.
#[derive(Parser, Debug)]
pub struct SnapshotDeleteArgs {
    /// Name of the snapshot to delete
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Delete without confirmation prompt
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(long, short = 'p', default_value = "8420")]
    pub port: u16,

    /// Bind address
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,

    /// Open browser automatically
    #[arg(long)]
    pub open: bool,
}

#[derive(Parser, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    pub shell: clap_complete::Shell,
}
