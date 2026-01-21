//! CLI argument definitions and command dispatch.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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

    /// Verbose output (show debug information)
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    /// Quiet mode (suppress non-essential output)
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Target device by serial number (required if multiple devices connected)
    #[arg(long, short = 's', global = true, env = "SD_SERIAL")]
    pub serial: Option<String>,

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
}

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

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum ResizeStrategy {
    /// Fit within key, maintain aspect ratio (may have black bars)
    #[default]
    Fit,
    /// Fill key, maintain aspect ratio (may crop)
    Fill,
    /// Stretch to fill (may distort)
    Stretch,
}

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

#[derive(Parser, Debug)]
pub struct WatchArgs {
    /// Exit after first button press
    #[arg(long)]
    pub once: bool,

    /// Timeout in seconds (0 = no timeout)
    #[arg(long, short = 't', default_value = "0")]
    pub timeout: u64,
}

#[derive(Parser, Debug)]
pub struct ReadArgs {}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Force overwrite existing configuration
    #[arg(long, short = 'f')]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Show configuration file path
    #[arg(long)]
    pub path: bool,
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
