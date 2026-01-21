//! Stream Deck CLI - Cross-platform control for Elgato Stream Deck devices.
//!
//! Provides both human-friendly and agent-friendly (robot mode) interfaces.
#![forbid(unsafe_code)]

mod batch;
mod cli;
mod config;
mod device;
mod error;

use std::io::{self, IsTerminal};

use clap::Parser;
use colored::Colorize;
use serde::Serialize;

use cli::{Cli, Commands};
use error::{Result, SdError};

/// Build information embedded at compile time.
mod build_info {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    pub fn git_sha() -> &'static str {
        option_env!("VERGEN_GIT_SHA").unwrap_or("unknown")
    }

    pub fn git_dirty() -> &'static str {
        option_env!("VERGEN_GIT_DIRTY").unwrap_or("false")
    }

    pub fn build_timestamp() -> &'static str {
        option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown")
    }

    pub fn rustc_semver() -> &'static str {
        option_env!("VERGEN_RUSTC_SEMVER").unwrap_or("unknown")
    }

    pub fn target() -> &'static str {
        option_env!("VERGEN_CARGO_TARGET_TRIPLE").unwrap_or("unknown")
    }
}

fn main() {
    let cli = Cli::parse();

    // Handle no-color flag or non-TTY
    if cli.no_color || !io::stdout().is_terminal() {
        colored::control::set_override(false);
    }

    // Run the command
    let result = run(&cli);

    // Handle errors
    if let Err(e) = result {
        output_error(&cli, &e);
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        None => print_quick_start(cli),
        Some(Commands::List(args)) => cmd_list(cli, args),
        Some(Commands::Info(args)) => cmd_info(cli, args),
        Some(Commands::Brightness(args)) => cmd_brightness(cli, args),
        Some(Commands::SetKey(args)) => cmd_set_key(cli, args),
        Some(Commands::SetKeys(args)) => cmd_set_keys(cli, args),
        Some(Commands::ClearKey(args)) => cmd_clear_key(cli, args),
        Some(Commands::ClearAll(args)) => cmd_clear_all(cli, args),
        Some(Commands::FillKey(args)) => cmd_fill_key(cli, args),
        Some(Commands::FillAll(args)) => cmd_fill_all(cli, args),
        Some(Commands::Watch(args)) => cmd_watch(cli, args),
        Some(Commands::Read(args)) => cmd_read(cli, args),
        Some(Commands::Init(args)) => cmd_init(cli, args),
        Some(Commands::Config(args)) => cmd_config(cli, args),
        Some(Commands::Serve(args)) => cmd_serve(cli, args),
        Some(Commands::Version) => cmd_version(cli),
        Some(Commands::Completions(args)) => cmd_completions(cli, args),
    }
}

// === Quick Start (Robot Mode Optimized) ===

/// Prints quick-start help optimized for both humans and AI agents.
#[allow(clippy::unnecessary_wraps)] // Consistent return type with other commands
fn print_quick_start(cli: &Cli) -> Result<()> {
    if cli.use_json() {
        print_robot_quick_start();
    } else {
        print_human_quick_start();
    }
    Ok(())
}

fn print_robot_quick_start() {
    let help = RobotQuickStart {
        tool: "sd",
        version: build_info::VERSION,
        description: "Cross-platform Stream Deck CLI with robot mode for AI agents",
        discovery: RobotDiscovery {
            list_devices: "sd list --robot",
            device_info: "sd info --robot",
            current_state: "sd read --robot",
        },
        display: RobotDisplay {
            set_brightness: "sd brightness <0-100>",
            set_key_image: "sd set-key <KEY> <IMAGE_PATH>",
            fill_key_color: "sd fill-key <KEY> <HEX_COLOR>",
            clear_key: "sd clear-key <KEY>",
            clear_all: "sd clear-all",
        },
        input: RobotInput {
            watch_buttons: "sd watch --robot",
            read_once: "sd read --robot",
            watch_single: "sd watch --once --robot",
        },
        key_layout: KeyLayout {
            note: "Keys indexed 0-31, left-to-right, top-to-bottom",
            example_xl: "Row 0: keys 0-7, Row 1: keys 8-15, Row 2: keys 16-23, Row 3: keys 24-31",
        },
        output_modes: OutputModes {
            human: "--format=text (default)",
            robot: "--robot or --format=json",
            compact: "--format=json-compact",
        },
        multi_device: "Use --serial <SERIAL> when multiple devices connected",
        web_ui: "sd serve --port 8420",
    };

    println!("{}", serde_json::to_string_pretty(&help).unwrap());
}

fn print_human_quick_start() {
    println!(
        "{} {} - Stream Deck CLI\n",
        "sd".bold().cyan(),
        build_info::VERSION
    );

    println!("{}", "QUICK START".bold().underline());
    println!();

    println!("  {}  List devices", "sd list".green());
    println!("  {}  Device info", "sd info".green());
    println!("  {}  Set brightness", "sd brightness 80".green());
    println!("  {}  Set key image", "sd set-key 0 icon.png".green());
    println!("  {}  Fill key with color", "sd fill-key 0 ff0000".green());
    println!("  {}  Clear all keys", "sd clear-all".green());
    println!("  {}  Watch button presses", "sd watch".green());
    println!();

    println!("{}", "ROBOT MODE (for AI agents)".bold().underline());
    println!();
    println!("  {}  JSON output", "sd --robot <command>".cyan());
    println!("  {}  Quick-start JSON", "sd --robot".cyan());
    println!();

    println!(
        "{}",
        "KEY LAYOUT (Stream Deck XL 32-key)".bold().underline()
    );
    println!();
    println!("  Row 0: [0] [1] [2] [3] [4] [5] [6] [7]");
    println!("  Row 1: [8] [9] [10][11][12][13][14][15]");
    println!("  Row 2: [16][17][18][19][20][21][22][23]");
    println!("  Row 3: [24][25][26][27][28][29][30][31]");
    println!();

    println!("Run {} for full help", "sd --help".yellow());
}

// === Robot Mode JSON Structures ===

#[derive(Serialize)]
struct RobotQuickStart {
    tool: &'static str,
    version: &'static str,
    description: &'static str,
    discovery: RobotDiscovery,
    display: RobotDisplay,
    input: RobotInput,
    key_layout: KeyLayout,
    output_modes: OutputModes,
    multi_device: &'static str,
    web_ui: &'static str,
}

#[derive(Serialize)]
struct RobotDiscovery {
    list_devices: &'static str,
    device_info: &'static str,
    current_state: &'static str,
}

#[derive(Serialize)]
struct RobotDisplay {
    set_brightness: &'static str,
    set_key_image: &'static str,
    fill_key_color: &'static str,
    clear_key: &'static str,
    clear_all: &'static str,
}

#[derive(Serialize)]
struct RobotInput {
    watch_buttons: &'static str,
    read_once: &'static str,
    watch_single: &'static str,
}

#[derive(Serialize)]
struct KeyLayout {
    note: &'static str,
    example_xl: &'static str,
}

#[derive(Serialize)]
struct OutputModes {
    human: &'static str,
    robot: &'static str,
    compact: &'static str,
}

// === Command Implementations ===

fn cmd_list(cli: &Cli, args: &cli::ListArgs) -> Result<()> {
    let devices = device::list_devices()?;

    if cli.use_json() {
        output_json(cli, &devices);
    } else if devices.is_empty() {
        println!("{}", "No Stream Deck devices found".yellow());
        println!("Ensure device is connected via USB");
    } else {
        for d in &devices {
            if args.long {
                println!(
                    "{}: {} ({} keys, {}x{} px)",
                    d.serial.green(),
                    d.product_name,
                    d.key_count,
                    d.key_width,
                    d.key_height
                );
            } else {
                println!("{}", d.serial);
            }
        }
    }
    Ok(())
}

fn cmd_info(cli: &Cli, _args: &cli::InfoArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;
    let info = device::get_device_info(&device);

    if cli.use_json() {
        output_json(cli, &info);
    } else {
        println!("{}: {}", "Product".bold(), info.product_name);
        println!("{}: {}", "Serial".bold(), info.serial);
        println!("{}: {}", "Firmware".bold(), info.firmware_version);
        println!("{}: {}", "Keys".bold(), info.key_count);
        println!(
            "{}: {}x{} px",
            "Key Size".bold(),
            info.key_width,
            info.key_height
        );
        println!(
            "{}: {} cols x {} rows",
            "Layout".bold(),
            info.cols,
            info.rows
        );
    }
    Ok(())
}

fn cmd_brightness(cli: &Cli, args: &cli::BrightnessArgs) -> Result<()> {
    if args.level > 100 {
        return Err(SdError::InvalidBrightness { value: args.level });
    }

    let device = device::open_device(cli.serial.as_deref())?;
    device::set_brightness(&device, args.level)?;

    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({ "brightness": args.level, "ok": true }),
        );
    } else if !cli.quiet {
        println!("Brightness set to {}%", args.level);
    }
    Ok(())
}

fn cmd_set_key(cli: &Cli, args: &cli::SetKeyArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;
    device::set_key_image(&device, args.key, &args.image)?;

    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "key": args.key,
                "image": args.image.display().to_string(),
                "ok": true
            }),
        );
    } else if !cli.quiet {
        println!("Key {} updated", args.key);
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Batch operations are inherently complex
fn cmd_set_keys(cli: &Cli, args: &cli::SetKeysArgs) -> Result<()> {
    // Open device to get key count
    let device = device::open_device(cli.serial.as_deref())?;
    let device_info = device::get_device_info(&device);

    // Scan directory for matching files
    let scan_result = batch::scan_directory(&args.dir, &args.pattern, device_info.key_count)
        .map_err(|e| SdError::Other(e.to_string()))?;

    // Handle dry-run mode
    if args.dry_run {
        return cmd_set_keys_dry_run(cli, args, &device_info, &scan_result);
    }

    // Check if we have any files to process
    if scan_result.mappings.is_empty() {
        if cli.use_json() {
            output_json(
                cli,
                &serde_json::json!({
                    "ok": false,
                    "error": "no_matching_files",
                    "message": format!("No files matching pattern '{}' found in {}", args.pattern, args.dir.display()),
                    "unmatched_count": scan_result.unmatched.len(),
                    "invalid_count": scan_result.invalid.len(),
                }),
            );
        } else {
            eprintln!(
                "No files matching pattern '{}' found in {}",
                args.pattern,
                args.dir.display()
            );
            if !scan_result.unmatched.is_empty() {
                eprintln!("  {} files didn't match pattern", scan_result.unmatched.len());
            }
            if !scan_result.invalid.is_empty() {
                eprintln!("  {} files had invalid key indices", scan_result.invalid.len());
            }
        }
        return Ok(());
    }

    // Apply images to keys
    let mut results: Vec<BatchKeyResult> = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for mapping in &scan_result.mappings {
        // Check key range filter if specified
        if let Some(ref range) = args.key_range {
            if !key_in_range(mapping.key, range) {
                continue;
            }
        }

        // Skip if key is below start_key
        if mapping.key < args.start_key {
            continue;
        }

        let result = device::set_key_image(&device, mapping.key, &mapping.path);

        match result {
            Ok(()) => {
                success_count += 1;
                results.push(BatchKeyResult {
                    key: mapping.key,
                    path: mapping.path.display().to_string(),
                    ok: true,
                    error: None,
                });
                if !cli.quiet && !cli.use_json() {
                    println!("Key {}: {}", mapping.key, mapping.path.display());
                }
            }
            Err(e) => {
                error_count += 1;
                results.push(BatchKeyResult {
                    key: mapping.key,
                    path: mapping.path.display().to_string(),
                    ok: false,
                    error: Some(e.to_string()),
                });

                if args.continue_on_error {
                    if !cli.quiet && !cli.use_json() {
                        eprintln!("Key {} failed: {}", mapping.key, e);
                    }
                } else {
                    // Return immediately on first error if not continuing
                    if cli.use_json() {
                        output_json(
                            cli,
                            &BatchSetKeysResult {
                                ok: false,
                                results,
                                success_count,
                                error_count,
                                skipped_count: 0,
                            },
                        );
                    }
                    return Err(e);
                }
            }
        }
    }

    // Output final results
    if cli.use_json() {
        output_json(
            cli,
            &BatchSetKeysResult {
                ok: error_count == 0,
                results,
                success_count,
                error_count,
                skipped_count: scan_result.mappings.len() - success_count - error_count,
            },
        );
    } else if !cli.quiet {
        println!("Set {success_count} keys ({error_count} errors)");
    }

    Ok(())
}

/// Result for a single key in batch operation.
#[derive(Serialize)]
struct BatchKeyResult {
    key: u8,
    path: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Result of batch set-keys operation.
#[derive(Serialize)]
struct BatchSetKeysResult {
    ok: bool,
    results: Vec<BatchKeyResult>,
    success_count: usize,
    error_count: usize,
    skipped_count: usize,
}

/// Dry-run handler for set-keys command.
#[allow(clippy::unnecessary_wraps)] // Consistent return type
fn cmd_set_keys_dry_run(
    cli: &Cli,
    args: &cli::SetKeysArgs,
    device_info: &device::DeviceInfo,
    scan_result: &batch::ScanResult,
) -> Result<()> {
    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "dry_run": true,
                "device": {
                    "serial": device_info.serial,
                    "model": device_info.product_name,
                    "key_count": device_info.key_count,
                },
                "directory": args.dir.display().to_string(),
                "pattern": args.pattern,
                "matched": scan_result.mappings.iter().map(|m| {
                    serde_json::json!({
                        "key": m.key,
                        "path": m.path.display().to_string(),
                        "size_bytes": m.size_bytes,
                    })
                }).collect::<Vec<_>>(),
                "unmatched_count": scan_result.unmatched.len(),
                "invalid": scan_result.invalid.iter().map(|(p, reason)| {
                    serde_json::json!({
                        "path": p.display().to_string(),
                        "reason": reason,
                    })
                }).collect::<Vec<_>>(),
            }),
        );
    } else {
        println!("DRY RUN: Would set {} keys from {}", scan_result.mappings.len(), args.dir.display());
        println!("  Device: {} ({})", device_info.product_name, device_info.serial);
        println!("  Pattern: {}", args.pattern);
        println!();

        for mapping in &scan_result.mappings {
            println!("  Key {}: {} ({} bytes)", mapping.key, mapping.path.display(), mapping.size_bytes);
        }

        if !scan_result.unmatched.is_empty() {
            println!();
            println!("  {} files didn't match pattern", scan_result.unmatched.len());
        }

        if !scan_result.invalid.is_empty() {
            println!();
            println!("  Invalid files:");
            for (path, reason) in &scan_result.invalid {
                println!("    {}: {}", path.display(), reason);
            }
        }
    }
    Ok(())
}

/// Check if a key index is within the specified range (e.g., "0-7").
fn key_in_range(key: u8, range: &str) -> bool {
    if let Some((start, end)) = range.split_once('-') {
        if let (Ok(start), Ok(end)) = (start.parse::<u8>(), end.parse::<u8>()) {
            return key >= start && key <= end;
        }
    }
    // If range parsing fails, include all keys
    true
}

fn cmd_clear_key(cli: &Cli, args: &cli::ClearKeyArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;
    device::clear_key(&device, args.key)?;

    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({ "key": args.key, "cleared": true }),
        );
    } else if !cli.quiet {
        println!("Key {} cleared", args.key);
    }
    Ok(())
}

fn cmd_clear_all(cli: &Cli, _args: &cli::ClearAllArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;
    device::clear_all_keys(&device)?;

    if cli.use_json() {
        output_json(cli, &serde_json::json!({ "cleared": "all", "ok": true }));
    } else if !cli.quiet {
        println!("All keys cleared");
    }
    Ok(())
}

fn cmd_fill_key(cli: &Cli, args: &cli::FillKeyArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;
    let color = parse_color(&args.color)?;
    device::fill_key_color(&device, args.key, color)?;

    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "key": args.key,
                "color": format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2),
                "ok": true
            }),
        );
    } else if !cli.quiet {
        println!("Key {} filled with #{}", args.key, args.color);
    }
    Ok(())
}

fn cmd_fill_all(cli: &Cli, args: &cli::FillAllArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;
    let color = parse_color(&args.color)?;
    device::fill_all_keys_color(&device, color)?;

    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "filled": "all",
                "color": format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2),
                "ok": true
            }),
        );
    } else if !cli.quiet {
        println!("All keys filled with #{}", args.color);
    }
    Ok(())
}

fn cmd_watch(cli: &Cli, args: &cli::WatchArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;

    if !cli.quiet && !cli.use_json() {
        println!("Watching for button presses (Ctrl+C to stop)...");
    }

    device::watch_buttons(&device, cli.use_json(), args.once, args.timeout)?;
    Ok(())
}

fn cmd_read(cli: &Cli, _args: &cli::ReadArgs) -> Result<()> {
    let device = device::open_device(cli.serial.as_deref())?;
    let states = device::read_button_states(&device);

    if cli.use_json() {
        output_json(cli, &states);
    } else {
        let pressed: Vec<_> = states
            .iter()
            .enumerate()
            .filter(|&(_, pressed)| *pressed)
            .map(|(i, _)| i)
            .collect();

        if pressed.is_empty() {
            println!("No buttons pressed");
        } else {
            println!("Pressed: {pressed:?}");
        }
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Will return errors when implemented
fn cmd_init(cli: &Cli, args: &cli::InitArgs) -> Result<()> {
    let _ = (cli, args); // TODO: implement
    eprintln!("Config init not yet implemented");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Will return errors when implemented
fn cmd_config(cli: &Cli, args: &cli::ConfigArgs) -> Result<()> {
    let _ = (cli, args); // TODO: implement
    eprintln!("Config show not yet implemented");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Will return errors when implemented
fn cmd_serve(cli: &Cli, args: &cli::ServeArgs) -> Result<()> {
    let _ = (cli, args); // TODO: implement
    eprintln!("Web server not yet implemented");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Consistent return type with other commands
fn cmd_version(cli: &Cli) -> Result<()> {
    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "version": build_info::VERSION,
                "git_sha": build_info::git_sha(),
                "git_dirty": build_info::git_dirty() == "true",
                "build_timestamp": build_info::build_timestamp(),
                "rustc_version": build_info::rustc_semver(),
                "target": build_info::target(),
            }),
        );
    } else {
        println!("sd {}", build_info::VERSION);
        println!(
            "git: {}{}",
            build_info::git_sha(),
            if build_info::git_dirty() == "true" {
                " (dirty)"
            } else {
                ""
            }
        );
        println!("built: {}", build_info::build_timestamp());
        println!("rustc: {}", build_info::rustc_semver());
        println!("target: {}", build_info::target());
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Consistent return type with other commands
fn cmd_completions(_cli: &Cli, args: &cli::CompletionsArgs) -> Result<()> {
    use clap::CommandFactory;
    clap_complete::generate(args.shell, &mut Cli::command(), "sd", &mut io::stdout());
    Ok(())
}

// === Utility Functions ===

fn parse_color(s: &str) -> Result<(u8, u8, u8)> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return Err(SdError::Other(format!(
            "Invalid color format '{s}': expected 6 hex digits (e.g., ff0000)"
        )));
    }

    let r = u8::from_str_radix(&s[0..2], 16)
        .map_err(|_| SdError::Other(format!("Invalid red component in '{s}'")))?;
    let g = u8::from_str_radix(&s[2..4], 16)
        .map_err(|_| SdError::Other(format!("Invalid green component in '{s}'")))?;
    let b = u8::from_str_radix(&s[4..6], 16)
        .map_err(|_| SdError::Other(format!("Invalid blue component in '{s}'")))?;

    Ok((r, g, b))
}

fn output_json<T: Serialize>(cli: &Cli, data: &T) {
    let json = if cli.use_compact_json() {
        serde_json::to_string(data).unwrap()
    } else {
        serde_json::to_string_pretty(data).unwrap()
    };
    println!("{json}");
}

fn output_error(cli: &Cli, error: &SdError) {
    if cli.use_json() {
        let json = serde_json::json!({
            "error": true,
            "message": error.to_string(),
            "suggestion": error.suggestion(),
            "recoverable": error.is_user_recoverable(),
        });
        eprintln!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        eprintln!("{}: {}", "Error".red().bold(), error);
        if let Some(suggestion) = error.suggestion() {
            eprintln!("{}: {}", "Hint".yellow(), suggestion);
        }
    }
}
