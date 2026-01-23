//! Stream Deck CLI - Cross-platform control for Elgato Stream Deck devices.
//!
//! Provides both human-friendly and agent-friendly (robot mode) interfaces.
#![forbid(unsafe_code)]

mod batch;
mod cli;
mod config;
mod device;
mod error;
mod image_ops;
mod logging;
mod output;
mod snapshot;
mod state;
mod theme;

use std::io;

use clap::Parser;
use image::GenericImageView;
use rich_rust::prelude::{Color, Console, Style, Text};
use serde::Serialize;

use cli::{Cli, Commands};
use device::DeviceOperations;
use error::{Result, SdError};
use output::{
    BatchKeyResult, BatchSummary, BrightnessDryRunDetails, ClearAllDryRunDetails,
    ClearKeyDryRunDetails, ClearKeysDryRunDetails, DeviceContext, DryRunResponse,
    FillKeyDryRunDetails, ImageSourceInfo, Output, OutputMode, ProcessingInfo, SetKeyDryRunDetails,
    ValidationError,
};

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

    // Initialize structured logging based on CLI flags
    logging::init_logging(cli.use_json(), cli.verbose, cli.quiet);

    // Note: no-color handling is now managed by rich_rust through OutputMode

    // Prepare output handler
    let output = OutputMode::from_cli(&cli).into_output();

    // Run the command
    let result = run(&cli, output.as_ref());

    // Handle errors
    if let Err(e) = result {
        output.error(&e);
        std::process::exit(1);
    }
}

fn run(cli: &Cli, output: &dyn Output) -> Result<()> {
    match &cli.command {
        None => print_quick_start(cli),
        Some(Commands::List(args)) => cmd_list(cli, args, output),
        Some(Commands::Info(args)) => cmd_info(cli, args, output),
        Some(Commands::Brightness(args)) => cmd_brightness(cli, args, output),
        Some(Commands::SetKey(args)) => cmd_set_key(cli, args, output),
        Some(Commands::SetKeys(args)) => cmd_set_keys(cli, args, output),
        Some(Commands::ClearKey(args)) => cmd_clear_key(cli, args, output),
        Some(Commands::ClearAll(args)) => cmd_clear_all(cli, args, output),
        Some(Commands::FillKey(args)) => cmd_fill_key(cli, args, output),
        Some(Commands::FillAll(args)) => cmd_fill_all(cli, args, output),
        Some(Commands::FillKeys(args)) => cmd_fill_keys(cli, args, output),
        Some(Commands::ClearKeys(args)) => cmd_clear_keys(cli, args, output),
        Some(Commands::Watch(args)) => cmd_watch(cli, args, output),
        Some(Commands::Read(args)) => cmd_read(cli, args, output),
        Some(Commands::Init(args)) => cmd_init(cli, args),
        Some(Commands::Config(args)) => cmd_config(cli, args),
        Some(Commands::Validate(args)) => cmd_validate(cli, args, output),
        Some(Commands::Apply(args)) => cmd_apply(cli, args, output),
        Some(Commands::Save(args)) => cmd_save(cli, args),
        Some(Commands::Restore(args)) => cmd_restore(cli, args),
        Some(Commands::Snapshots(args)) => cmd_snapshots(cli, args),
        Some(Commands::Snapshot(args)) => cmd_snapshot(cli, args),
        Some(Commands::Serve(args)) => cmd_serve(cli, args),
        Some(Commands::Version) => cmd_version(cli, output),
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
    let console = Console::new();
    let accent = Color::parse("#0080FF").expect("valid color");
    let success = Color::parse("#00D26A").expect("valid color");
    let warning = Color::parse("#FFA502").expect("valid color");

    // Title line
    let mut title = Text::new("");
    title.append_styled("sd", Style::new().bold().color(accent.clone()));
    title.append(&format!(" {} - Stream Deck CLI\n", build_info::VERSION));
    console.print_text(&title);

    // QUICK START section
    console.print_styled("QUICK START", Style::new().bold().underline());
    console.print("");

    let cmd_style = Style::new().color(success.clone());
    console.print_styled("  sd list                 ", cmd_style.clone());
    console.print("List devices");
    console.print_styled("  sd info                 ", cmd_style.clone());
    console.print("Device info");
    console.print_styled("  sd brightness 80        ", cmd_style.clone());
    console.print("Set brightness");
    console.print_styled("  sd set-key 0 icon.png   ", cmd_style.clone());
    console.print("Set key image");
    console.print_styled("  sd fill-key 0 ff0000    ", cmd_style.clone());
    console.print("Fill key with color");
    console.print_styled("  sd clear-all            ", cmd_style.clone());
    console.print("Clear all keys");
    console.print_styled("  sd watch                ", cmd_style);
    console.print("Watch button presses");
    console.print("");

    // ROBOT MODE section
    console.print_styled(
        "ROBOT MODE (for AI agents)",
        Style::new().bold().underline(),
    );
    console.print("");

    let robot_style = Style::new().color(accent.clone());
    console.print_styled("  sd --robot <command>    ", robot_style.clone());
    console.print("JSON output");
    console.print_styled("  sd --robot              ", robot_style);
    console.print("Quick-start JSON");
    console.print("");

    // KEY LAYOUT section
    console.print_styled(
        "KEY LAYOUT (Stream Deck XL 32-key)",
        Style::new().bold().underline(),
    );
    console.print("");
    console.print("  Row 0: [0] [1] [2] [3] [4] [5] [6] [7]");
    console.print("  Row 1: [8] [9] [10][11][12][13][14][15]");
    console.print("  Row 2: [16][17][18][19][20][21][22][23]");
    console.print("  Row 3: [24][25][26][27][28][29][30][31]");
    console.print("");

    // Help hint
    let mut help = Text::new("Run ");
    help.append_styled("sd --help", Style::new().color(warning));
    help.append(" for full help");
    console.print_text(&help);
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

// === Device Opening Helper ===

/// Opens a Stream Deck device, using retry logic if enabled via CLI flags.
fn open_device(cli: &Cli) -> Result<device::Device> {
    if cli.retry_enabled() {
        let opts = cli.connection_options();
        tracing::debug!(
            retry = opts.max_retries,
            delay_ms = opts.retry_delay.as_millis(),
            backoff = opts.backoff_factor,
            "Opening device with retry"
        );
        device::open_device_with_retry(cli.serial.as_deref(), &opts)
    } else {
        device::open_device(cli.serial.as_deref())
    }
}

// === Command Implementations ===

fn cmd_list(cli: &Cli, _args: &cli::ListArgs, output: &dyn Output) -> Result<()> {
    let _ = cli; // Will be used later for args.long formatting
    let devices = device::list_devices()?;
    output.device_list(&devices);
    Ok(())
}

fn cmd_info(cli: &Cli, _args: &cli::InfoArgs, output: &dyn Output) -> Result<()> {
    let device = open_device(cli)?;
    let info = device::get_device_info(&device);
    output.device_info(&info);
    Ok(())
}

fn cmd_brightness(cli: &Cli, args: &cli::BrightnessArgs, output: &dyn Output) -> Result<()> {
    // Validate brightness level
    if args.level > 100 {
        return Err(SdError::InvalidBrightness { value: args.level });
    }

    // Handle dry-run mode
    if cli.is_dry_run() {
        return cmd_brightness_dry_run(cli, args);
    }

    let device = open_device(cli)?;
    device::set_brightness(&device, args.level)?;

    // Track state change
    state::record::brightness(args.level);

    output.brightness_set(args.level);
    Ok(())
}

/// Dry-run handler for brightness command.
#[allow(clippy::unnecessary_wraps)] // Consistent return type
fn cmd_brightness_dry_run(cli: &Cli, args: &cli::BrightnessArgs) -> Result<()> {
    // Try to get device info for context (may fail if no device connected)
    let device_result = open_device(cli);

    if cli.use_json() {
        let details = BrightnessDryRunDetails::new(args.level, None);

        let response = match device_result {
            Ok(device) => {
                let info = device::get_device_info(&device);
                let ctx = DeviceContext::from_info(&info);
                DryRunResponse::success("set_brightness", details, ctx)
            }
            Err(ref e) => {
                // Device not connected - still valid dry-run
                let ctx = DeviceContext::disconnected(cli.serial.clone());
                DryRunResponse::success("set_brightness", details, ctx)
                    .with_warnings(vec![format!("Device not connected: {e}")])
            }
        };

        output_json(cli, &response);
    } else {
        // Human-readable dry-run output
        println!("DRY RUN: Would set brightness to {}%", args.level);

        match device_result {
            Ok(device) => {
                let info = device::get_device_info(&device);
                println!("  Device: {} (serial: {})", info.product_name, info.serial);
            }
            Err(e) => {
                println!("  Device: not connected ({})", e);
            }
        }
    }

    Ok(())
}

fn cmd_set_key(cli: &Cli, args: &cli::SetKeyArgs, output: &dyn Output) -> Result<()> {
    // Handle dry-run mode
    if cli.is_dry_run() {
        return cmd_set_key_dry_run(cli, args);
    }

    let device = open_device(cli)?;
    device::set_key_image(&device, args.key, &args.image, args.resize)?;

    // Track state change
    state::record::set_key(args.key, args.image.clone());

    output.key_set(args.key, &args.image);
    Ok(())
}

/// Dry-run handler for set-key command.
#[allow(clippy::unnecessary_wraps)] // Consistent return type
fn cmd_set_key_dry_run(cli: &Cli, args: &cli::SetKeyArgs) -> Result<()> {
    // Try to get device info for context
    let device_result = open_device(cli);

    // Analyze the source image
    let source_info = analyze_image_source(&args.image);

    if cli.use_json() {
        let (device_ctx, device_info) = match &device_result {
            Ok(device) => {
                let info = device::get_device_info(device);
                (DeviceContext::from_info(&info), Some(info))
            }
            Err(_) => (DeviceContext::disconnected(cli.serial.clone()), None),
        };

        // Calculate processing info
        let target_dims = device_info
            .as_ref()
            .map(|i| (i.key_width as u32, i.key_height as u32))
            .unwrap_or((96, 96)); // Default XL dimensions

        let resize_needed = source_info
            .dimensions
            .map(|(w, h)| w != target_dims.0 || h != target_dims.1)
            .unwrap_or(false);

        let processing = ProcessingInfo {
            resize_needed,
            target_dimensions: target_dims,
        };

        let details = SetKeyDryRunDetails::new(args.key, source_info.clone(), processing);

        // Build response based on validation
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if image exists
        if !source_info.exists {
            errors.push(ValidationError {
                field: "image".to_string(),
                error: format!("Image file not found: {}", args.image.display()),
                suggestion: Some(
                    "Check the file path. Use absolute paths or paths relative to current directory."
                        .to_string(),
                ),
            });
        }

        // Check key index if device connected
        if let Some(ref info) = device_info {
            if args.key >= info.key_count {
                errors.push(ValidationError {
                    field: "key".to_string(),
                    error: format!(
                        "Key index {} is out of range (device has {} keys, valid: 0-{})",
                        args.key,
                        info.key_count,
                        info.key_count - 1
                    ),
                    suggestion: Some(format!("Use a key index from 0 to {}", info.key_count - 1)),
                });
            }
        }

        // Add resize warning
        if resize_needed {
            if let Some((w, h)) = source_info.dimensions {
                warnings.push(format!(
                    "Image will be resized from {}x{} to {}x{}",
                    w, h, target_dims.0, target_dims.1
                ));
            }
        }

        // Add device warning if not connected
        if let Err(ref e) = device_result {
            warnings.push(format!("Device not connected: {e}"));
        }

        let response = if errors.is_empty() {
            DryRunResponse::success("set_key", details, device_ctx).with_warnings(warnings)
        } else {
            let reason = if !source_info.exists {
                "Image file not found"
            } else {
                "Validation failed"
            };
            DryRunResponse::failure("set_key", reason, errors, details, device_ctx)
                .with_warnings(warnings)
        };

        output_json(cli, &response);
    } else {
        // Human-readable dry-run output
        println!(
            "DRY RUN: Would set key {} to {}",
            args.key,
            args.image.display()
        );

        if source_info.exists {
            if let Some((w, h)) = source_info.dimensions {
                println!("  Image: {}x{}", w, h);
            }
            if let Some(format) = &source_info.format {
                println!("  Format: {}", format.to_uppercase());
            }
            if let Some(size) = source_info.size_bytes {
                println!("  Size: {} bytes", size);
            }
        } else {
            println!("  WARNING: Image file not found!");
        }

        match device_result {
            Ok(device) => {
                let info = device::get_device_info(&device);
                println!("  Device: {} (serial: {})", info.product_name, info.serial);
                if args.key >= info.key_count {
                    println!(
                        "  WARNING: Key {} is out of range (max: {})",
                        args.key,
                        info.key_count - 1
                    );
                }
            }
            Err(e) => {
                println!("  Device: not connected ({})", e);
            }
        }
    }

    Ok(())
}

/// Analyze an image source file without fully loading it.
fn analyze_image_source(path: &std::path::Path) -> ImageSourceInfo {
    let exists = path.exists();
    let metadata = std::fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(|m| m.len());

    // Get format from extension
    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    // Try to read dimensions if file exists
    let dimensions = if exists {
        image::image_dimensions(path).ok()
    } else {
        None
    };

    ImageSourceInfo {
        path: path.display().to_string(),
        exists,
        readable: exists && metadata.map(|m| m.is_file()).unwrap_or(false),
        format,
        dimensions,
        size_bytes,
    }
}

#[allow(clippy::too_many_lines)] // Batch operations are inherently complex
fn cmd_set_keys(cli: &Cli, args: &cli::SetKeysArgs, output: &dyn Output) -> Result<()> {
    // Open device to get key count
    let device = open_device(cli)?;
    let device_info = device::get_device_info(&device);

    // Scan directory for matching files
    let scan_result = batch::scan_directory(&args.dir, &args.pattern, device_info.key_count)
        .map_err(|e| SdError::Other(e.to_string()))?;

    // Handle dry-run mode (check both global and local flag)
    if cli.is_dry_run() || args.dry_run {
        return cmd_set_keys_dry_run(cli, args, &device_info, &scan_result);
    }

    // Check if we have any files to process
    if scan_result.mappings.is_empty() {
        output.warning(&format!(
            "No files matching pattern '{}' found in {}",
            args.pattern,
            args.dir.display()
        ));
        if !scan_result.unmatched.is_empty() {
            output.info(&format!(
                "{} files didn't match pattern",
                scan_result.unmatched.len()
            ));
        }
        if !scan_result.invalid.is_empty() {
            output.info(&format!(
                "{} files had invalid key indices",
                scan_result.invalid.len()
            ));
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

        let result = device::set_key_image(&device, mapping.key, &mapping.path, args.resize);

        match result {
            Ok(()) => {
                success_count += 1;
                // Track state change
                state::record::set_key(mapping.key, mapping.path.clone());
                results.push(BatchKeyResult::set_key_success(mapping.key, &mapping.path));
            }
            Err(e) => {
                error_count += 1;
                results.push(BatchKeyResult::set_key_failure(
                    mapping.key,
                    &mapping.path,
                    &e.to_string(),
                ));

                if !args.continue_on_error {
                    // Output results so far before returning error
                    let summary = BatchSummary::new(results.len(), success_count, error_count);
                    output.batch_set_keys(&results, &summary);
                    return Err(e);
                }
            }
        }
    }

    // Output final results
    let skipped = scan_result.mappings.len() - success_count - error_count;
    let summary =
        BatchSummary::new(results.len(), success_count, error_count).with_skipped(skipped);
    if !cli.quiet {
        output.batch_set_keys(&results, &summary);
    }

    Ok(())
}

/// Dry-run details for set-keys batch command.
#[derive(Serialize)]
struct SetKeysDryRunDetails {
    directory: String,
    pattern: String,
    operations: Vec<SetKeysDryRunOperation>,
    summary: SetKeysDryRunSummary,
}

/// Per-key dry-run operation details.
#[derive(Serialize)]
struct SetKeysDryRunOperation {
    key: u8,
    source: Option<String>,
    would_succeed: bool,
    resize_needed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Summary stats for set-keys dry-run.
#[derive(Serialize)]
struct SetKeysDryRunSummary {
    total_keys: usize,
    matching_files: usize,
    would_succeed: usize,
    would_fail: usize,
    unmatched: usize,
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
        let mut operations = Vec::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut resize_count = 0;

        let in_scope = |key: u8, range: Option<&String>, start_key: u8| {
            if let Some(range_str) = range {
                if !key_in_range(key, range_str) {
                    return false;
                }
            }
            key >= start_key
        };

        for mapping in &scan_result.mappings {
            if !in_scope(mapping.key, args.key_range.as_ref(), args.start_key) {
                continue;
            }

            let mut op = SetKeysDryRunOperation {
                key: mapping.key,
                source: Some(mapping.path.display().to_string()),
                would_succeed: true,
                resize_needed: None,
                error: None,
            };

            match image::open(&mapping.path) {
                Ok(img) => {
                    let (w, h) = img.dimensions();
                    let resize_needed =
                        w != device_info.key_width as u32 || h != device_info.key_height as u32;
                    if resize_needed {
                        resize_count += 1;
                    }
                    op.resize_needed = Some(resize_needed);
                }
                Err(e) => {
                    let error = format!("Image processing failed: {e}");
                    op.would_succeed = false;
                    op.error = Some(error.clone());
                    errors.push(ValidationError {
                        field: format!("image[{}]", mapping.key),
                        error,
                        suggestion: Some(
                            "Use a supported image format: png, jpg, jpeg, gif, bmp, webp"
                                .to_string(),
                        ),
                    });
                }
            }

            operations.push(op);
        }

        let has_any_matches = !scan_result.mappings.is_empty();

        if !has_any_matches {
            errors.push(ValidationError {
                field: "pattern".to_string(),
                error: format!(
                    "No files matching pattern '{}' found in {}",
                    args.pattern,
                    args.dir.display()
                ),
                suggestion: Some("Check the directory and filename pattern".to_string()),
            });
        }

        if !scan_result.unmatched.is_empty() {
            warnings.push(format!(
                "{} files didn't match the pattern",
                scan_result.unmatched.len()
            ));
        }

        if !scan_result.invalid.is_empty() {
            warnings.push(format!(
                "{} files had out-of-range key indices (max {})",
                scan_result.invalid.len(),
                device_info.key_count.saturating_sub(1)
            ));
        }

        if resize_count > 0 {
            warnings.push(format!(
                "{resize_count} images will be resized to {}x{}",
                device_info.key_width, device_info.key_height
            ));
        }

        let total_keys = (0..device_info.key_count)
            .filter(|key| in_scope(*key, args.key_range.as_ref(), args.start_key))
            .count();
        let matching_files = operations.len();
        let would_succeed = operations.iter().filter(|op| op.would_succeed).count();
        let would_fail = matching_files.saturating_sub(would_succeed);
        let unmatched = total_keys.saturating_sub(matching_files);

        if has_any_matches && matching_files == 0 {
            warnings.push("No matching files within the specified key range/start_key".to_string());
        }

        let details = SetKeysDryRunDetails {
            directory: args.dir.display().to_string(),
            pattern: args.pattern.clone(),
            operations,
            summary: SetKeysDryRunSummary {
                total_keys,
                matching_files,
                would_succeed,
                would_fail,
                unmatched,
            },
        };

        let device = DeviceContext::from_info(device_info);
        let response = if errors.is_empty() && would_fail == 0 {
            DryRunResponse::success("set_keys_batch", details, device).with_warnings(warnings)
        } else {
            let reason = if !has_any_matches {
                "No matching files found"
            } else {
                "One or more operations would fail"
            };
            DryRunResponse::failure("set_keys_batch", reason, errors, details, device)
                .with_warnings(warnings)
        };

        output_json(cli, &response);
    } else {
        println!(
            "DRY RUN: Would set {} keys from {}",
            scan_result.mappings.len(),
            args.dir.display()
        );
        println!(
            "  Device: {} ({})",
            device_info.product_name, device_info.serial
        );
        println!("  Pattern: {}", args.pattern);
        println!();

        for mapping in &scan_result.mappings {
            println!(
                "  Key {}: {} ({} bytes)",
                mapping.key,
                mapping.path.display(),
                mapping.size_bytes
            );
        }

        if !scan_result.unmatched.is_empty() {
            println!();
            println!(
                "  {} files didn't match pattern",
                scan_result.unmatched.len()
            );
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

fn cmd_clear_key(cli: &Cli, args: &cli::ClearKeyArgs, output: &dyn Output) -> Result<()> {
    // Handle dry-run mode
    if cli.is_dry_run() {
        return cmd_clear_key_dry_run(cli, args);
    }

    let device = open_device(cli)?;
    device::clear_key(&device, args.key)?;

    // Track state change
    state::record::clear_key(args.key);

    output.key_cleared(args.key);
    Ok(())
}

/// Dry-run handler for clear-key command.
#[allow(clippy::unnecessary_wraps)] // Consistent return type
fn cmd_clear_key_dry_run(cli: &Cli, args: &cli::ClearKeyArgs) -> Result<()> {
    // Try to get device info for context
    let device_result = open_device(cli);

    if cli.use_json() {
        let (device_ctx, device_info) = match &device_result {
            Ok(device) => {
                let info = device::get_device_info(device);
                (DeviceContext::from_info(&info), Some(info))
            }
            Err(_) => (DeviceContext::disconnected(cli.serial.clone()), None),
        };

        let details = ClearKeyDryRunDetails::new(args.key);

        // Build response based on validation
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check key index if device connected
        if let Some(ref info) = device_info {
            if args.key >= info.key_count {
                errors.push(ValidationError {
                    field: "key".to_string(),
                    error: format!(
                        "Key index {} is out of range (device has {} keys, valid: 0-{})",
                        args.key,
                        info.key_count,
                        info.key_count - 1
                    ),
                    suggestion: Some(format!("Use a key index from 0 to {}", info.key_count - 1)),
                });
            }
        }

        // Add device warning if not connected
        if let Err(ref e) = device_result {
            warnings.push(format!("Device not connected: {e}"));
        }

        let response = if errors.is_empty() {
            DryRunResponse::success("clear_key", details, device_ctx).with_warnings(warnings)
        } else {
            DryRunResponse::failure(
                "clear_key",
                "Validation failed",
                errors,
                details,
                device_ctx,
            )
            .with_warnings(warnings)
        };

        output_json(cli, &response);
    } else {
        // Human-readable dry-run output
        println!("DRY RUN: Would clear key {} (set to black)", args.key);

        match device_result {
            Ok(device) => {
                let info = device::get_device_info(&device);
                println!("  Device: {} (serial: {})", info.product_name, info.serial);
                if args.key >= info.key_count {
                    println!(
                        "  WARNING: Key {} is out of range (max: {})",
                        args.key,
                        info.key_count - 1
                    );
                }
            }
            Err(e) => {
                println!("  Device: not connected ({})", e);
            }
        }
    }

    Ok(())
}

fn cmd_clear_all(cli: &Cli, _args: &cli::ClearAllArgs, output: &dyn Output) -> Result<()> {
    // Handle dry-run mode
    if cli.is_dry_run() {
        return cmd_clear_all_dry_run(cli);
    }

    let device = open_device(cli)?;
    let info = device::get_device_info(&device);
    device::clear_all_keys(&device)?;

    // Track state change
    state::record::clear_all(info.key_count);

    output.all_cleared();
    Ok(())
}

/// Dry-run handler for clear-all command.
#[allow(clippy::unnecessary_wraps)] // Consistent return type
fn cmd_clear_all_dry_run(cli: &Cli) -> Result<()> {
    // Try to get device info for context
    let device_result = open_device(cli);

    if cli.use_json() {
        let (device_ctx, key_count) = match &device_result {
            Ok(device) => {
                let info = device::get_device_info(device);
                (DeviceContext::from_info(&info), info.key_count)
            }
            Err(_) => (DeviceContext::disconnected(cli.serial.clone()), 0),
        };

        let details = ClearAllDryRunDetails::new(key_count);

        let mut warnings = Vec::new();

        // Add device warning if not connected
        if let Err(ref e) = device_result {
            warnings.push(format!("Device not connected: {e}"));
        }

        let response =
            DryRunResponse::success("clear_all", details, device_ctx).with_warnings(warnings);

        output_json(cli, &response);
    } else {
        // Human-readable dry-run output
        match device_result {
            Ok(device) => {
                let info = device::get_device_info(&device);
                println!(
                    "DRY RUN: Would clear all {} keys (set to black)",
                    info.key_count
                );
                println!("  Device: {} (serial: {})", info.product_name, info.serial);
            }
            Err(e) => {
                println!("DRY RUN: Would clear all keys (set to black)");
                println!("  Device: not connected ({})", e);
            }
        }
    }

    Ok(())
}

fn cmd_fill_key(cli: &Cli, args: &cli::FillKeyArgs, output: &dyn Output) -> Result<()> {
    // Handle dry-run mode
    if cli.is_dry_run() {
        return cmd_fill_key_dry_run(cli, args);
    }

    let device = open_device(cli)?;
    let color = parse_color(&args.color)?;
    device::fill_key_color(&device, args.key, color)?;

    // Track state change
    let color_str = format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2);
    state::record::fill_key(args.key, color_str.clone());

    output.key_filled(args.key, &color_str);
    Ok(())
}

/// Dry-run handler for fill-key command.
#[allow(clippy::unnecessary_wraps)] // Consistent return type
fn cmd_fill_key_dry_run(cli: &Cli, args: &cli::FillKeyArgs) -> Result<()> {
    // Validate color first
    let color = parse_color(&args.color)?;
    let color_str = format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2);

    // Try to get device info for context
    let device_result = open_device(cli);

    if cli.use_json() {
        let (device_ctx, device_info) = match &device_result {
            Ok(device) => {
                let info = device::get_device_info(device);
                (DeviceContext::from_info(&info), Some(info))
            }
            Err(_) => (DeviceContext::disconnected(cli.serial.clone()), None),
        };

        let details = FillKeyDryRunDetails::new(args.key, color_str.clone(), color);

        // Build response based on validation
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check key index if device connected
        if let Some(ref info) = device_info {
            if args.key >= info.key_count {
                errors.push(ValidationError {
                    field: "key".to_string(),
                    error: format!(
                        "Key index {} is out of range (device has {} keys, valid: 0-{})",
                        args.key,
                        info.key_count,
                        info.key_count - 1
                    ),
                    suggestion: Some(format!("Use a key index from 0 to {}", info.key_count - 1)),
                });
            }
        }

        // Add device warning if not connected
        if let Err(ref e) = device_result {
            warnings.push(format!("Device not connected: {e}"));
        }

        let response = if errors.is_empty() {
            DryRunResponse::success("fill_key", details, device_ctx).with_warnings(warnings)
        } else {
            DryRunResponse::failure("fill_key", "Validation failed", errors, details, device_ctx)
                .with_warnings(warnings)
        };

        output_json(cli, &response);
    } else {
        // Human-readable dry-run output
        println!(
            "DRY RUN: Would fill key {} with color {}",
            args.key, color_str
        );
        println!("  RGB: ({}, {}, {})", color.0, color.1, color.2);

        match device_result {
            Ok(device) => {
                let info = device::get_device_info(&device);
                println!("  Device: {} (serial: {})", info.product_name, info.serial);
                if args.key >= info.key_count {
                    println!(
                        "  WARNING: Key {} is out of range (max: {})",
                        args.key,
                        info.key_count - 1
                    );
                }
            }
            Err(e) => {
                println!("  Device: not connected ({})", e);
            }
        }
    }

    Ok(())
}

fn cmd_fill_all(cli: &Cli, args: &cli::FillAllArgs, output: &dyn Output) -> Result<()> {
    let device = open_device(cli)?;
    let info = device::get_device_info(&device);
    let color = parse_color(&args.color)?;
    device::fill_all_keys_color(&device, color)?;

    // Track state change for all keys
    let color_str = format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2);
    for key in 0..info.key_count {
        state::record::fill_key(key, color_str.clone());
    }

    output.all_filled(&color_str);
    Ok(())
}

fn cmd_fill_keys(cli: &Cli, args: &cli::FillKeysArgs, output: &dyn Output) -> Result<()> {
    let device = open_device(cli)?;
    let device_info = device::get_device_info(&device);
    let color = parse_color(&args.color)?;
    let color_str = format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2);

    // Determine which keys to fill
    let keys = resolve_key_selection(
        args.all,
        args.range.as_deref(),
        &args.keys,
        device_info.key_count,
    )?;

    if keys.is_empty() {
        return Err(SdError::Other(
            "No keys specified. Use --all, --range, or --keys".to_string(),
        ));
    }

    // Fill keys with color
    let mut results: Vec<BatchKeyResult> = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for key in &keys {
        match device::fill_key_color(&device, *key, color) {
            Ok(()) => {
                success_count += 1;
                // Track state change
                state::record::fill_key(*key, color_str.clone());
                results.push(BatchKeyResult::fill_success(*key, &color_str));
            }
            Err(e) => {
                error_count += 1;
                results.push(BatchKeyResult::fill_failure(
                    *key,
                    &color_str,
                    &e.to_string(),
                ));

                if !args.continue_on_error {
                    // Output results so far before returning error
                    let summary = BatchSummary::new(results.len(), success_count, error_count);
                    output.batch_fill_keys(&color_str, &results, &summary);
                    return Err(e);
                }
            }
        }
    }

    // Output final results
    let summary = BatchSummary::new(keys.len(), success_count, error_count);
    if !cli.quiet {
        output.batch_fill_keys(&color_str, &results, &summary);
    }

    Ok(())
}

fn cmd_clear_keys(cli: &Cli, args: &cli::ClearKeysArgs, output: &dyn Output) -> Result<()> {
    // Handle dry-run mode
    if cli.is_dry_run() {
        return cmd_clear_keys_dry_run(cli, args);
    }

    let device = open_device(cli)?;
    let device_info = device::get_device_info(&device);

    // Determine which keys to clear
    let keys = resolve_key_selection(
        args.all,
        args.range.as_deref(),
        &args.keys,
        device_info.key_count,
    )?;

    if keys.is_empty() {
        return Err(SdError::Other(
            "No keys specified. Use --all, --range, or --keys".to_string(),
        ));
    }

    // If --all is specified, use the optimized clear_all function
    if args.all {
        device::clear_all_keys(&device)?;
        // Track state change
        state::record::clear_all(device_info.key_count);
        let results: Vec<BatchKeyResult> = (0..device_info.key_count)
            .map(|k| BatchKeyResult::clear_success(k))
            .collect();
        let summary = BatchSummary::new(
            device_info.key_count as usize,
            device_info.key_count as usize,
            0,
        );
        if !cli.quiet {
            output.batch_clear_keys(&results, &summary);
        }
        return Ok(());
    }

    // Clear individual keys
    let mut results: Vec<BatchKeyResult> = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for key in &keys {
        match device::clear_key(&device, *key) {
            Ok(()) => {
                success_count += 1;
                // Track state change
                state::record::clear_key(*key);
                results.push(BatchKeyResult::clear_success(*key));
            }
            Err(e) => {
                error_count += 1;
                results.push(BatchKeyResult::clear_failure(*key, &e.to_string()));

                if args.continue_on_error {
                    // Continue processing other keys
                } else {
                    // Output results so far before returning error
                    let summary = BatchSummary::new(keys.len(), success_count, error_count);
                    if !cli.quiet {
                        output.batch_clear_keys(&results, &summary);
                    }
                    return Err(e);
                }
            }
        }
    }

    let summary = BatchSummary::new(keys.len(), success_count, error_count);
    if !cli.quiet {
        output.batch_clear_keys(&results, &summary);
    }

    Ok(())
}

/// Dry-run handler for clear-keys (batch) command.
#[allow(clippy::unnecessary_wraps)] // Consistent return type
fn cmd_clear_keys_dry_run(cli: &Cli, args: &cli::ClearKeysArgs) -> Result<()> {
    // Try to get device info for context
    let device_result = open_device(cli);

    if cli.use_json() {
        let (device_ctx, device_info) = match &device_result {
            Ok(device) => {
                let info = device::get_device_info(device);
                (DeviceContext::from_info(&info), Some(info))
            }
            Err(_) => (DeviceContext::disconnected(cli.serial.clone()), None),
        };

        // We need key_count to resolve selection - use device info if available
        let key_count = device_info.as_ref().map(|i| i.key_count).unwrap_or(32);

        // Resolve key selection
        let keys_result =
            resolve_key_selection(args.all, args.range.as_deref(), &args.keys, key_count);

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Add device warning if not connected
        if let Err(ref e) = device_result {
            warnings.push(format!("Device not connected: {e}"));
            warnings.push("Using default key count of 32 for validation".to_string());
        }

        match keys_result {
            Ok(keys) => {
                if keys.is_empty() {
                    errors.push(ValidationError {
                        field: "keys".to_string(),
                        error: "No keys specified".to_string(),
                        suggestion: Some(
                            "Use --all, --range, or --keys to specify which keys to clear"
                                .to_string(),
                        ),
                    });
                    let details = ClearKeysDryRunDetails::new(vec![]);
                    let response = DryRunResponse::failure(
                        "clear_keys",
                        "No keys specified",
                        errors,
                        details,
                        device_ctx,
                    )
                    .with_warnings(warnings);
                    output_json(cli, &response);
                } else {
                    let details = ClearKeysDryRunDetails::new(keys);
                    let response = DryRunResponse::success("clear_keys", details, device_ctx)
                        .with_warnings(warnings);
                    output_json(cli, &response);
                }
            }
            Err(e) => {
                errors.push(ValidationError {
                    field: "keys".to_string(),
                    error: e.to_string(),
                    suggestion: Some("Check the key range or indices specified".to_string()),
                });
                let details = ClearKeysDryRunDetails::new(vec![]);
                let response = DryRunResponse::failure(
                    "clear_keys",
                    "Invalid key selection",
                    errors,
                    details,
                    device_ctx,
                )
                .with_warnings(warnings);
                output_json(cli, &response);
            }
        }
    } else {
        // Human-readable dry-run output
        match device_result {
            Ok(device) => {
                let info = device::get_device_info(&device);
                let keys_result = resolve_key_selection(
                    args.all,
                    args.range.as_deref(),
                    &args.keys,
                    info.key_count,
                );

                match keys_result {
                    Ok(keys) => {
                        if keys.is_empty() {
                            println!("DRY RUN: No keys specified");
                            println!(
                                "  Use --all, --range, or --keys to specify which keys to clear"
                            );
                        } else if args.all {
                            println!(
                                "DRY RUN: Would clear all {} keys (set to black)",
                                info.key_count
                            );
                        } else {
                            println!("DRY RUN: Would clear {} keys: {:?}", keys.len(), keys);
                        }
                        println!("  Device: {} (serial: {})", info.product_name, info.serial);
                    }
                    Err(e) => {
                        println!("DRY RUN: Invalid key selection: {}", e);
                        println!("  Device: {} (serial: {})", info.product_name, info.serial);
                    }
                }
            }
            Err(e) => {
                println!("DRY RUN: Would clear keys (set to black)");
                println!("  Device: not connected ({})", e);

                // Try to show what would be cleared based on args
                if args.all {
                    println!("  Selection: all keys");
                } else if let Some(ref range) = args.range {
                    println!("  Selection: keys in range {}", range);
                } else if !args.keys.is_empty() {
                    println!("  Selection: keys {:?}", args.keys);
                } else {
                    println!("  Selection: none specified");
                }
            }
        }
    }

    Ok(())
}

/// Resolves key selection from --all, --range, or --keys options.
fn resolve_key_selection(
    all: bool,
    range: Option<&str>,
    keys: &[u8],
    key_count: u8,
) -> Result<Vec<u8>> {
    if all {
        return Ok((0..key_count).collect());
    }

    if let Some(range_str) = range {
        return parse_key_range(range_str, key_count);
    }

    if !keys.is_empty() {
        // Validate all keys are in range
        for key in keys {
            if *key >= key_count {
                return Err(SdError::InvalidKeyIndex {
                    index: *key,
                    max: key_count,
                    max_idx: key_count - 1,
                });
            }
        }
        return Ok(keys.to_vec());
    }

    Ok(vec![])
}

/// Parses a key range string like "0-7" into a vector of key indices.
fn parse_key_range(range: &str, key_count: u8) -> Result<Vec<u8>> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return Err(SdError::Other(format!(
            "Invalid range format '{range}': expected START-END (e.g., 0-7)"
        )));
    }

    let start: u8 = parts[0]
        .parse()
        .map_err(|_| SdError::Other(format!("Invalid range start '{}': not a number", parts[0])))?;

    let end: u8 = parts[1]
        .parse()
        .map_err(|_| SdError::Other(format!("Invalid range end '{}': not a number", parts[1])))?;

    if start > end {
        return Err(SdError::Other(format!(
            "Invalid range '{range}': start ({start}) must be <= end ({end})"
        )));
    }

    if end >= key_count {
        return Err(SdError::InvalidKeyIndex {
            index: end,
            max: key_count,
            max_idx: key_count - 1,
        });
    }

    Ok((start..=end).collect())
}

/// Maximum backoff delay for reconnection (30 seconds).
const MAX_RECONNECT_DELAY_MS: u64 = 30_000;
/// Backoff multiplier for exponential backoff.
const RECONNECT_BACKOFF_FACTOR: f64 = 1.5;

fn cmd_watch(cli: &Cli, args: &cli::WatchArgs, output: &dyn Output) -> Result<()> {
    let mut device = open_device(cli)?;
    let serial = cli.serial.clone();

    if !cli.quiet && !cli.use_json() {
        output.info("Watching for button presses (Ctrl+C to stop)...");
        if args.reconnect {
            output.info("Auto-reconnect enabled");
        }
    }

    // Track reconnection state
    let mut reconnect_attempts: u32 = 0;
    let mut reconnect_delay = args.reconnect_delay;

    loop {
        // Try to watch for events using the output trait
        let result = watch_buttons_with_output(&device, output, args.once, args.timeout);

        match result {
            Ok(()) => {
                // Normal exit (timeout, --once, or clean shutdown)
                return Ok(());
            }
            Err(ref e) if e.is_connection_error() && args.reconnect => {
                reconnect_attempts += 1;

                // Check max attempts (0 = unlimited)
                if args.max_reconnect_attempts > 0
                    && reconnect_attempts > args.max_reconnect_attempts
                {
                    // Emit final disconnect event
                    output.warning(&format!("Disconnected: {} (reconnecting: false)", e));

                    if !cli.quiet && !cli.use_json() {
                        output.warning(&format!(
                            "Max reconnection attempts ({}) exceeded",
                            args.max_reconnect_attempts
                        ));
                    }
                    return Err(SdError::Other(format!(
                        "Connection lost after {} reconnection attempts: {}",
                        reconnect_attempts - 1,
                        e
                    )));
                }

                // Emit disconnect event
                output.warning(&format!(
                    "Connection lost ({}), reconnecting in {}ms (attempt {}{})...",
                    e,
                    reconnect_delay,
                    reconnect_attempts,
                    if args.max_reconnect_attempts > 0 {
                        format!("/{}", args.max_reconnect_attempts)
                    } else {
                        String::new()
                    }
                ));

                // Wait before reconnecting
                std::thread::sleep(std::time::Duration::from_millis(reconnect_delay));

                // Try to reconnect
                match device::open_device(serial.as_deref()) {
                    Ok(new_device) => {
                        device = new_device;

                        // Emit reconnected event
                        output.success(&format!(
                            "Reconnected successfully (attempt {})",
                            reconnect_attempts
                        ));

                        // Reset backoff on successful connection
                        reconnect_delay = args.reconnect_delay;
                        reconnect_attempts = 0;
                    }
                    Err(conn_err) => {
                        tracing::debug!(error = %conn_err, "Reconnection attempt failed");

                        // Increase backoff with cap
                        #[allow(clippy::cast_possible_truncation)]
                        #[allow(clippy::cast_sign_loss)]
                        {
                            reconnect_delay = ((reconnect_delay as f64 * RECONNECT_BACKOFF_FACTOR)
                                as u64)
                                .min(MAX_RECONNECT_DELAY_MS);
                        }
                        // Continue loop to try again
                    }
                }
            }
            Err(e) => {
                // Non-connection error or reconnect disabled
                return Err(e);
            }
        }
    }
}

/// Watch for button presses using the Output trait.
///
/// This function provides the watch loop that uses the Output trait
/// for all button event reporting, enabling both robot and human modes.
fn watch_buttons_with_output(
    device: &device::Device,
    output: &dyn Output,
    once: bool,
    timeout_secs: u64,
) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = if timeout_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(timeout_secs))
    };

    // We need to access device internals - use the existing watch function
    // but intercept events. For now, delegate to read_button_states in a loop.
    let mut last_states = vec![false; device.info().key_count as usize];

    loop {
        // Check timeout
        if let Some(t) = timeout {
            if start.elapsed() >= t {
                break;
            }
        }

        // Read current states
        let states = device::read_button_states(device);

        // Detect changes
        for (key, (&current, &previous)) in states.iter().zip(last_states.iter()).enumerate() {
            if current && !previous {
                // Button pressed
                #[allow(clippy::cast_possible_truncation)]
                let event = device::ButtonEvent {
                    key: key as u8,
                    pressed: true,
                    timestamp_ms: start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                };
                output.button_event(&event);

                if once {
                    return Ok(());
                }
            } else if !current && previous {
                // Button released
                #[allow(clippy::cast_possible_truncation)]
                let event = device::ButtonEvent {
                    key: key as u8,
                    pressed: false,
                    timestamp_ms: start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                };
                output.button_event(&event);
            }
        }

        last_states = states;

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

/// Connection events emitted during watch with reconnect.
#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum WatchConnectionEvent {
    Disconnected { reason: String, reconnecting: bool },
    Reconnecting { attempt: u32, delay_ms: u64 },
    Reconnected { attempt: u32 },
}

/// Emits a watch connection event in robot mode.
fn emit_watch_event(cli: &Cli, event: WatchConnectionEvent) {
    if cli.use_json() {
        let json = if cli.use_compact_json() {
            serde_json::to_string(&event).unwrap_or_default()
        } else {
            serde_json::to_string_pretty(&event).unwrap_or_default()
        };
        println!("{json}");
    }
}

fn cmd_read(cli: &Cli, _args: &cli::ReadArgs, output: &dyn Output) -> Result<()> {
    let device = open_device(cli)?;
    let states = device::read_button_states(&device);
    output.button_states(&states);
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

/// Validate a declarative configuration file without applying it.
fn cmd_validate(_cli: &Cli, args: &cli::ValidateArgs, output: &dyn Output) -> Result<()> {
    use config::declarative::{ConfigFormat, load_config};
    use output::ValidationResult;
    use tracing::{debug, info, warn};

    info!(config = %args.config.display(), "Validating configuration file");

    let mut result = ValidationResult::new(&args.config);

    // Phase 1: Check file exists
    if !args.config.exists() {
        result.add_error(
            "config_file",
            format!("File not found: {}", args.config.display()),
        );
        output.validation_result(&result);
        return if args.strict || !result.is_valid() {
            Err(SdError::ConfigNotFound {
                path: args.config.display().to_string(),
            })
        } else {
            Ok(())
        };
    }

    // Phase 2: Detect format
    let format = ConfigFormat::from_extension(&args.config);
    if format.is_none() {
        result.add_error(
            "config_file",
            "Unknown file extension. Expected .yaml, .yml, or .toml",
        );
        output.validation_result(&result);
        return Err(SdError::ConfigParse("Unknown config format".to_string()));
    }

    // Phase 3: Load and parse
    let config = match load_config(&args.config) {
        Ok(c) => c,
        Err(e) => {
            result.add_error("syntax", e.to_string());
            output.validation_result(&result);
            return Err(e);
        }
    };

    debug!(name = ?config.name, keys = config.keys.len(), "Config parsed successfully");

    // Set config name
    result.config_name = config.name.clone();

    // Set summary stats
    result.summary.key_count = Some(config.keys.len());
    result.summary.brightness = config.brightness;

    // Phase 4: Validate brightness
    if let Some(brightness) = config.brightness {
        if brightness > 100 {
            result.add_error(
                "brightness",
                format!("Brightness {} exceeds maximum of 100", brightness),
            );
        }
    }

    // Phase 5: Validate key configurations
    for (selector_str, key_config) in &config.keys {
        // Validate selector
        match config::KeySelector::parse(selector_str) {
            Ok(_) => {}
            Err(e) => {
                result.add_error(
                    format!("key[{}]", selector_str),
                    format!("Invalid selector: {}", e),
                );
            }
        }

        // Validate key config
        if let Err(e) = key_config.validate() {
            result.add_error(format!("key[{}]", selector_str), e.to_string());
        }

        // Validate image paths exist (if image type)
        match key_config {
            config::KeyConfig::Image { image, .. } => {
                let resolved = if image.starts_with("~") {
                    if let Some(home) = config::home_dir() {
                        home.join(image.strip_prefix("~").unwrap_or(image))
                    } else {
                        image.clone()
                    }
                } else if image.is_relative() {
                    args.config
                        .parent()
                        .map(|p| p.join(image))
                        .unwrap_or_else(|| image.clone())
                } else {
                    image.clone()
                };

                if !resolved.exists() {
                    result.add_error(
                        format!("key[{}]", selector_str),
                        format!("Image not found: {}", image.display()),
                    );
                }
            }
            config::KeyConfig::Pattern { pattern, .. } => {
                if !pattern.contains("{index}") {
                    result.add_error(
                        format!("key[{}]", selector_str),
                        "Pattern must contain {index} placeholder",
                    );
                }
            }
            config::KeyConfig::Color { color } => {
                if color.to_rgb().is_err() {
                    result.add_error(
                        format!("key[{}]", selector_str),
                        format!("Invalid color: {:?}", color),
                    );
                }
            }
            config::KeyConfig::Clear { clear } => {
                if !clear {
                    result.add_warning(
                        format!("key[{}]", selector_str),
                        "clear: false is redundant; omit the key instead",
                    );
                }
            }
        }
    }

    // Phase 6: Device-specific validation (optional, if device connected)
    // Try to get device info, but don't fail if no device
    match list_devices() {
        Ok(devices) if !devices.is_empty() => {
            let device_info = &devices[0];
            let key_count = device_info.key_count;

            for (selector_str, _) in &config.keys {
                if let Ok(selector) = config::KeySelector::parse(selector_str) {
                    // Check if selector indices are valid for this device
                    match &selector {
                        config::KeySelector::Single(idx) => {
                            if *idx >= key_count {
                                result.add_error(
                                    format!("key[{}]", selector_str),
                                    format!(
                                        "Key index {} out of range for {} (0-{})",
                                        idx,
                                        device_info.product_name,
                                        key_count - 1
                                    ),
                                );
                            }
                        }
                        config::KeySelector::Range { start, end } => {
                            if *end >= key_count {
                                result.add_error(
                                    format!("key[{}]", selector_str),
                                    format!(
                                        "Range end {} out of range for {} (0-{})",
                                        end,
                                        device_info.product_name,
                                        key_count - 1
                                    ),
                                );
                            }
                            if start > end {
                                result.add_error(
                                    format!("key[{}]", selector_str),
                                    format!("Invalid range: start {} > end {}", start, end),
                                );
                            }
                        }
                        config::KeySelector::Row(row) => {
                            if *row >= device_info.rows {
                                result.add_error(
                                    format!("key[{}]", selector_str),
                                    format!(
                                        "Row {} out of range for {} (0-{})",
                                        row,
                                        device_info.product_name,
                                        device_info.rows - 1
                                    ),
                                );
                            }
                        }
                        config::KeySelector::Column(col) => {
                            if *col >= device_info.cols {
                                result.add_error(
                                    format!("key[{}]", selector_str),
                                    format!(
                                        "Column {} out of range for {} (0-{})",
                                        col,
                                        device_info.product_name,
                                        device_info.cols - 1
                                    ),
                                );
                            }
                        }
                        config::KeySelector::Default => {}
                    }
                }
            }
        }
        _ => {
            result.add_warning(
                "device",
                "No device connected; skipping device-specific validation",
            );
        }
    }

    info!(
        valid = result.is_valid(),
        errors = result.summary.error_count,
        warnings = result.summary.warning_count,
        "Validation complete"
    );

    output.validation_result(&result);

    // Exit with error if not valid or if strict mode and warnings exist
    if !result.is_valid() {
        Err(SdError::ConfigInvalid(format!(
            "{} error(s) found",
            result.summary.error_count
        )))
    } else if args.strict && result.summary.warning_count > 0 {
        Err(SdError::ConfigInvalid(format!(
            "{} warning(s) found (strict mode)",
            result.summary.warning_count
        )))
    } else {
        Ok(())
    }
}

/// Apply a declarative configuration to the device.
fn cmd_apply(cli: &Cli, args: &cli::ApplyArgs, output: &dyn Output) -> Result<()> {
    use config::KeySelector;
    use config::declarative::load_config;
    use tracing::{debug, info, warn};

    info!(config = %args.config.display(), "Applying configuration");

    // Phase 1: Load and validate config
    if !args.config.exists() {
        return Err(SdError::ConfigNotFound {
            path: args.config.display().to_string(),
        });
    }

    let config = load_config(&args.config)?;
    debug!(
        name = ?config.name,
        keys = config.keys.len(),
        brightness = ?config.brightness,
        "Config loaded"
    );

    // Phase 2: Validate config (unless --force)
    if !args.force {
        // Run validation to check for errors
        let mut has_errors = false;

        // Check brightness
        if let Some(brightness) = config.brightness {
            if brightness > 100 {
                output.error(&SdError::InvalidBrightness { value: brightness });
                has_errors = true;
            }
        }

        // Check key configs
        for (selector_str, key_config) in &config.keys {
            if let Err(e) = KeySelector::parse(selector_str) {
                output.error(&SdError::ConfigParse(format!(
                    "Invalid selector '{}': {}",
                    selector_str, e
                )));
                has_errors = true;
            }
            if let Err(e) = key_config.validate() {
                output.error(&SdError::ConfigParse(format!(
                    "Invalid key config for '{}': {}",
                    selector_str, e
                )));
                has_errors = true;
            }
        }

        if has_errors {
            return Err(SdError::ConfigInvalid(
                "Config has errors. Use --force to apply anyway.".to_string(),
            ));
        }
    }

    // Phase 3: Handle dry-run mode
    if args.dry_run || args.diff {
        return cmd_apply_dry_run(cli, args, &config, output);
    }

    // Phase 4: Open device
    let device = open_device(cli)?;
    let device_info = device::get_device_info(&device);

    // Phase 5: Apply brightness (unless --no-brightness)
    if !args.no_brightness {
        if let Some(brightness) = config.brightness {
            debug!(brightness, "Setting brightness");
            device::set_brightness(&device, brightness)?;
            state::record::brightness(brightness);
            output.brightness_set(brightness);
        }
    }

    // Phase 6: Apply key configurations
    let mut results: Vec<BatchKeyResult> = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    // Process keys in selector priority order
    for (selector_str, key_config) in &config.keys {
        let selector = match KeySelector::parse(selector_str) {
            Ok(s) => s,
            Err(e) => {
                warn!(selector = selector_str, error = %e, "Skipping invalid selector");
                continue;
            }
        };

        let keys = match selector.resolve(&device_info) {
            Ok(k) => k,
            Err(e) => {
                warn!(selector = selector_str, error = %e, "Failed to resolve selector");
                continue;
            }
        };

        for key in keys {
            let result = apply_key_config(&device, &device_info, key, key_config, &args.config);
            match result {
                Ok(res) => {
                    success_count += 1;
                    results.push(res);
                }
                Err(e) => {
                    error_count += 1;
                    results.push(BatchKeyResult {
                        key,
                        path: None,
                        color: None,
                        ok: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
    }

    // Phase 7: Output results
    let summary = BatchSummary::new(results.len(), success_count, error_count);

    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "command": "apply",
                "config": args.config.display().to_string(),
                "config_name": config.name,
                "device": {
                    "serial": device_info.serial,
                    "product": device_info.product_name,
                },
                "results": results,
                "summary": summary,
            }),
        );
    } else {
        if let Some(name) = &config.name {
            output.info(&format!("Applied config: {}", name));
        }
        output.batch_set_keys(&results, &summary);
    }

    if error_count > 0 {
        Err(SdError::Other(format!(
            "{} key(s) failed to apply",
            error_count
        )))
    } else {
        Ok(())
    }
}

/// Apply a single key configuration to the device.
fn apply_key_config(
    device: &impl DeviceOperations,
    device_info: &device::DeviceInfo,
    key: u8,
    key_config: &config::KeyConfig,
    config_path: &std::path::Path,
) -> Result<BatchKeyResult> {
    match key_config {
        config::KeyConfig::Image { image, resize } => {
            // Resolve image path relative to config file
            let resolved = if image.starts_with("~") {
                if let Some(home) = config::home_dir() {
                    home.join(image.strip_prefix("~").unwrap_or(image))
                } else {
                    image.clone()
                }
            } else if image.is_relative() {
                config_path
                    .parent()
                    .map(|p| p.join(image))
                    .unwrap_or_else(|| image.clone())
            } else {
                image.clone()
            };

            device::set_key_image(device, key, &resolved, *resize)?;
            state::record::set_key(key, resolved.clone());
            Ok(BatchKeyResult::set_key_success(key, &resolved))
        }
        config::KeyConfig::Color { color } => {
            let (r, g, b) = color.to_rgb()?;
            device::fill_key_color(device, key, (r, g, b))?;
            let color_str = format!("#{:02x}{:02x}{:02x}", r, g, b);
            state::record::fill_key(key, color_str.clone());
            Ok(BatchKeyResult::fill_success(key, &color_str))
        }
        config::KeyConfig::Clear { clear } => {
            if *clear {
                device::clear_key(device, key)?;
                state::record::clear_key(key);
                Ok(BatchKeyResult::clear_success(key))
            } else {
                // clear: false means skip this key
                Ok(BatchKeyResult {
                    key,
                    path: None,
                    color: None,
                    ok: true,
                    error: None,
                })
            }
        }
        config::KeyConfig::Pattern { pattern, resize } => {
            // Resolve pattern by substituting {index}
            let filename = pattern
                .replace("{index}", &key.to_string())
                .replace("{index:02d}", &format!("{:02}", key))
                .replace("{index:03d}", &format!("{:03}", key));

            let resolved = if filename.starts_with("~") {
                if let Some(home) = config::home_dir() {
                    home.join(filename.strip_prefix("~").unwrap_or(&filename))
                } else {
                    std::path::PathBuf::from(&filename)
                }
            } else if std::path::Path::new(&filename).is_relative() {
                config_path
                    .parent()
                    .map(|p| p.join(&filename))
                    .unwrap_or_else(|| std::path::PathBuf::from(&filename))
            } else {
                std::path::PathBuf::from(&filename)
            };

            device::set_key_image(device, key, &resolved, *resize)?;
            state::record::set_key(key, resolved.clone());
            Ok(BatchKeyResult::set_key_success(key, &resolved))
        }
    }
}

/// Dry-run handler for apply command.
#[allow(clippy::unnecessary_wraps)]
fn cmd_apply_dry_run(
    cli: &Cli,
    args: &cli::ApplyArgs,
    config: &config::declarative::ProfileConfig,
    output: &dyn Output,
) -> Result<()> {
    use config::KeySelector;

    // Try to get device info
    let device_result = open_device(cli);
    let device_info = device_result.as_ref().ok().map(device::get_device_info);

    let mut operations = Vec::new();
    let mut warnings = Vec::new();

    // Build operation list
    for (selector_str, key_config) in &config.keys {
        let selector = match KeySelector::parse(selector_str) {
            Ok(s) => s,
            Err(e) => {
                warnings.push(format!("Invalid selector '{}': {}", selector_str, e));
                continue;
            }
        };

        let keys = if let Some(ref info) = device_info {
            match selector.resolve(info) {
                Ok(k) => k,
                Err(e) => {
                    warnings.push(format!("Cannot resolve '{}': {}", selector_str, e));
                    continue;
                }
            }
        } else {
            // No device - show selector as-is
            vec![]
        };

        let action = match key_config {
            config::KeyConfig::Image { image, .. } => format!("set image: {}", image.display()),
            config::KeyConfig::Color { color } => format!("fill color: {:?}", color),
            config::KeyConfig::Clear { clear } => {
                if *clear {
                    "clear".to_string()
                } else {
                    "skip".to_string()
                }
            }
            config::KeyConfig::Pattern { pattern, .. } => format!("pattern: {}", pattern),
        };

        operations.push(serde_json::json!({
            "selector": selector_str,
            "keys": keys,
            "action": action,
        }));
    }

    if cli.use_json() {
        let response = serde_json::json!({
            "dry_run": true,
            "command": "apply",
            "config": args.config.display().to_string(),
            "config_name": config.name,
            "brightness": config.brightness,
            "no_brightness": args.no_brightness,
            "device": device_info.as_ref().map(|i| serde_json::json!({
                "serial": i.serial,
                "product": i.product_name,
                "key_count": i.key_count,
            })),
            "operations": operations,
            "warnings": warnings,
        });
        output_json(cli, &response);
    } else {
        println!("DRY RUN: Would apply configuration");
        println!("  Config: {}", args.config.display());
        if let Some(name) = &config.name {
            println!("  Profile: {}", name);
        }

        if let Some(ref info) = device_info {
            println!("  Device: {} (serial: {})", info.product_name, info.serial);
        } else {
            println!("  Device: not connected");
        }

        if let Some(brightness) = config.brightness {
            if args.no_brightness {
                println!(
                    "  Brightness: {} (skipped with --no-brightness)",
                    brightness
                );
            } else {
                println!("  Brightness: would set to {}%", brightness);
            }
        }

        println!("\n  Key operations:");
        for op in &operations {
            println!(
                "    {} -> {}",
                op["selector"].as_str().unwrap_or("?"),
                op["action"].as_str().unwrap_or("?")
            );
        }

        if !warnings.is_empty() {
            println!("\n  Warnings:");
            for w in &warnings {
                println!("    - {}", w);
            }
        }
    }

    Ok(())
}

// === Snapshot Commands ===

fn cmd_save(cli: &Cli, args: &cli::SaveArgs) -> Result<()> {
    // Validate snapshot name
    if !is_valid_snapshot_name(&args.name) {
        return Err(SdError::Other(
            "Snapshot name must be 1-64 characters, alphanumeric with hyphens/underscores"
                .to_string(),
        ));
    }

    // Get device info for snapshot metadata
    let device = open_device(cli)?;
    let device_info = device::get_device_info(&device);

    // Open snapshot database
    let mut db = snapshot::SnapshotDb::open_default()?;

    // Check for existing snapshot
    if db.snapshot_exists(&args.name)? && !args.force {
        return Err(SdError::Other(format!(
            "Snapshot '{}' already exists. Use --force to overwrite.",
            args.name
        )));
    }

    // Get current session state
    let session = state::session_state();

    // Build keys list from session state
    let mut keys = Vec::new();
    let key_indices: Vec<u8> = if args.session_only {
        // Only keys modified in this session
        session.keys.keys().copied().collect()
    } else {
        // All keys from session (we can only save what we've tracked)
        session.keys.keys().copied().collect()
    };

    for key_index in key_indices {
        if let Some(session_key) = session.keys.get(&key_index) {
            let key_state = match session_key {
                state::KeyState::Image { path } => {
                    // Hash the image for content-addressable storage
                    let hash = hash_image_file(path)?;
                    // Cache the image
                    cache_image(&db, &hash, path)?;
                    snapshot::KeyState::Image {
                        source_path: Some(path.clone()),
                        image_hash: hash,
                    }
                }
                state::KeyState::Color { hex } => snapshot::KeyState::Color { hex: hex.clone() },
                state::KeyState::Cleared => snapshot::KeyState::Clear,
            };
            keys.push(snapshot::SnapshotKey {
                key_index,
                state: key_state,
            });
        }
    }

    // Get brightness from session or None
    let brightness = if args.no_brightness {
        None
    } else {
        session.brightness
    };

    // Create snapshot
    let mut snap = snapshot::Snapshot::new(
        args.name.clone(),
        device_info.product_name.clone(),
        device_info.key_count,
        device_info.key_width as u32,
        device_info.key_height as u32,
    );
    snap.brightness = brightness;
    snap.description = args.description.clone();
    snap.device_serial = Some(device_info.serial.clone());
    snap.keys = keys;

    // Save to database
    let id = db.save_snapshot(&snap)?;

    // Output result
    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "command": "save",
                "ok": true,
                "snapshot": {
                    "id": id,
                    "name": args.name,
                    "device_model": device_info.product_name,
                    "key_count": device_info.key_count,
                    "brightness": brightness,
                    "keys_saved": snap.keys.len(),
                }
            }),
        );
    } else if !cli.quiet {
        println!(
            "Saved snapshot '{}' ({} keys{})",
            args.name,
            snap.keys.len(),
            brightness.map_or(String::new(), |b| format!(", brightness {b}%"))
        );
    }

    Ok(())
}

fn cmd_restore(cli: &Cli, args: &cli::RestoreArgs) -> Result<()> {
    // Open snapshot database
    let db = snapshot::SnapshotDb::open_default()?;

    // Load snapshot
    let snap = db
        .load_snapshot(&args.name)?
        .ok_or_else(|| SdError::Other(format!("Snapshot '{}' not found", args.name)))?;

    // Open device
    let device = open_device(cli)?;
    let device_info = device::get_device_info(&device);

    // Check device compatibility
    if snap.key_count != device_info.key_count {
        return Err(SdError::Other(format!(
            "Snapshot was saved for {} keys, but device has {} keys",
            snap.key_count, device_info.key_count
        )));
    }

    // Apply brightness if present and not skipped
    if !args.no_brightness {
        if let Some(brightness) = snap.brightness {
            device::set_brightness(&device, brightness)?;
            state::record::brightness(brightness);
        }
    }

    // Apply keys
    let mut applied_count = 0;
    let mut error_count = 0;

    for key in &snap.keys {
        let result = match &key.state {
            snapshot::KeyState::Image {
                source_path,
                image_hash,
            } => {
                // Try to load from cache first, then source path
                apply_cached_image(&device, key.key_index, image_hash, source_path.as_ref())
            }
            snapshot::KeyState::Color { hex } => {
                let color = parse_color(hex)?;
                device::fill_key_color(&device, key.key_index, color).map(|()| {
                    state::record::fill_key(key.key_index, hex.clone());
                })
            }
            snapshot::KeyState::Clear => device::clear_key(&device, key.key_index).map(|()| {
                state::record::clear_key(key.key_index);
            }),
        };

        match result {
            Ok(()) => applied_count += 1,
            Err(e) => {
                error_count += 1;
                tracing::warn!(key = key.key_index, error = %e, "Failed to restore key");
            }
        }
    }

    // Output result
    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "command": "restore",
                "ok": error_count == 0,
                "snapshot": args.name,
                "keys_applied": applied_count,
                "keys_failed": error_count,
                "brightness_applied": !args.no_brightness && snap.brightness.is_some(),
            }),
        );
    } else if !cli.quiet {
        if error_count == 0 {
            println!("Restored snapshot '{}' ({} keys)", args.name, applied_count);
        } else {
            println!(
                "Restored snapshot '{}' ({} keys, {} failed)",
                args.name, applied_count, error_count
            );
        }
    }

    Ok(())
}

fn cmd_snapshots(cli: &Cli, args: &cli::SnapshotsArgs) -> Result<()> {
    // Open snapshot database
    let db = snapshot::SnapshotDb::open_default()?;

    // List snapshots
    let snapshots = db.list_snapshots()?;

    if cli.use_json() {
        output_json(cli, &snapshots);
    } else if snapshots.is_empty() {
        println!("No snapshots saved");
        println!("Use 'sd save <name>' to save the current device state");
    } else {
        let console = Console::new();
        let success = Color::parse("#00D26A").expect("valid color");
        let muted = Color::parse("#747D8C").expect("valid color");

        for snap in &snapshots {
            if args.long {
                let mut line = Text::new("");
                line.append_styled(&snap.name, Style::new().color(success.clone()));
                line.append(&format!(
                    ": {} ({} keys{})",
                    snap.device_model,
                    snap.key_count,
                    snap.brightness
                        .map_or(String::new(), |b| format!(", {b}% brightness"))
                ));
                console.print_text(&line);
                if let Some(ref desc) = snap.description {
                    console.print_styled(&format!("  {desc}"), Style::new().color(muted.clone()));
                }
                console.print(&format!(
                    "  Created: {}",
                    snap.created_at.format("%Y-%m-%d %H:%M")
                ));
            } else {
                console.print(&snap.name);
            }
        }
    }

    Ok(())
}

fn cmd_snapshot(cli: &Cli, args: &cli::SnapshotCommand) -> Result<()> {
    match &args.command {
        cli::SnapshotSubcommand::Show(show_args) => cmd_snapshot_show(cli, show_args),
        cli::SnapshotSubcommand::Delete(delete_args) => cmd_snapshot_delete(cli, delete_args),
    }
}

fn cmd_snapshot_show(cli: &Cli, args: &cli::SnapshotShowArgs) -> Result<()> {
    // Open snapshot database
    let db = snapshot::SnapshotDb::open_default()?;

    // Load snapshot
    let snap = db
        .load_snapshot(&args.name)?
        .ok_or_else(|| SdError::Other(format!("Snapshot '{}' not found", args.name)))?;

    if cli.use_json() {
        output_json(cli, &snap);
    } else {
        let console = Console::new();
        let bold = Style::new().bold();

        console.print_styled("Snapshot Details", Style::new().bold().underline());
        console.print("");

        let mut line = Text::new("");
        line.append_styled("Name", bold.clone());
        line.append(&format!(": {}", snap.name));
        console.print_text(&line);

        if let Some(ref desc) = snap.description {
            let mut line = Text::new("");
            line.append_styled("Description", bold.clone());
            line.append(&format!(": {desc}"));
            console.print_text(&line);
        }

        let mut line = Text::new("");
        line.append_styled("Device", bold.clone());
        line.append(&format!(": {}", snap.device_model));
        console.print_text(&line);

        if let Some(ref serial) = snap.device_serial {
            let mut line = Text::new("");
            line.append_styled("Serial", bold.clone());
            line.append(&format!(": {serial}"));
            console.print_text(&line);
        }

        let mut line = Text::new("");
        line.append_styled("Keys", bold.clone());
        line.append(&format!(
            ": {} ({}x{} px)",
            snap.key_count, snap.key_width, snap.key_height
        ));
        console.print_text(&line);

        if let Some(brightness) = snap.brightness {
            let mut line = Text::new("");
            line.append_styled("Brightness", bold.clone());
            line.append(&format!(": {brightness}%"));
            console.print_text(&line);
        }

        let mut line = Text::new("");
        line.append_styled("Created", bold.clone());
        line.append(&format!(
            ": {}",
            snap.created_at.format("%Y-%m-%d %H:%M:%S")
        ));
        console.print_text(&line);

        let mut line = Text::new("");
        line.append_styled("Updated", bold.clone());
        line.append(&format!(
            ": {}",
            snap.updated_at.format("%Y-%m-%d %H:%M:%S")
        ));
        console.print_text(&line);

        if !snap.keys.is_empty() {
            console.print("");
            console.print_styled("Keys:", bold);
            for key in &snap.keys {
                let state_desc = match &key.state {
                    snapshot::KeyState::Image {
                        source_path,
                        image_hash,
                    } => {
                        let path_str = source_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(cached)".to_string());
                        format!(
                            "image: {} [{}...]",
                            path_str,
                            &image_hash[..8.min(image_hash.len())]
                        )
                    }
                    snapshot::KeyState::Color { hex } => format!("color: {hex}"),
                    snapshot::KeyState::Clear => "cleared".to_string(),
                };
                console.print(&format!("  Key {}: {}", key.key_index, state_desc));
            }
        }
    }

    Ok(())
}

fn cmd_snapshot_delete(cli: &Cli, args: &cli::SnapshotDeleteArgs) -> Result<()> {
    // Open snapshot database
    let mut db = snapshot::SnapshotDb::open_default()?;

    // Check if snapshot exists
    if !db.snapshot_exists(&args.name)? {
        return Err(SdError::Other(format!(
            "Snapshot '{}' not found",
            args.name
        )));
    }

    // Confirm deletion if not forced and not in robot mode
    if !args.force && !cli.use_json() {
        let console = Console::new();
        let warning = Color::parse("#FFA502").expect("valid color");
        console.print_styled(
            &format!("Delete snapshot '{}'? This cannot be undone.", args.name),
            Style::new().color(warning),
        );
        console.print("Use --force to skip this prompt");
        return Ok(());
    }

    // Delete snapshot
    let deleted = db.delete_snapshot(&args.name)?;

    if cli.use_json() {
        output_json(
            cli,
            &serde_json::json!({
                "command": "snapshot delete",
                "ok": deleted,
                "name": args.name,
                "deleted": deleted,
            }),
        );
    } else if !cli.quiet {
        if deleted {
            println!("Deleted snapshot '{}'", args.name);
        } else {
            println!("Snapshot '{}' not found", args.name);
        }
    }

    // Cleanup orphaned images
    let cleaned = db.cleanup_orphaned_images()?;
    if cleaned > 0 && !cli.quiet && !cli.use_json() {
        println!("Cleaned up {} orphaned cached images", cleaned);
    }

    Ok(())
}

/// Validates a snapshot name.
fn is_valid_snapshot_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Computes SHA256 hash of an image file.
fn hash_image_file(path: &std::path::Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let data = std::fs::read(path)
        .map_err(|e| SdError::Other(format!("Failed to read image {}: {e}", path.display())))?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// Caches an image to content-addressable storage.
fn cache_image(db: &snapshot::SnapshotDb, hash: &str, source_path: &std::path::Path) -> Result<()> {
    // Get cache path
    let cache_path = snapshot::image_cache_path(hash)?;

    // Skip if already cached
    if cache_path.exists() {
        return Ok(());
    }

    // Create cache directory
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SdError::Other(format!("Failed to create cache directory: {e}")))?;
    }

    // Copy image to cache (could convert to webp in future)
    std::fs::copy(source_path, &cache_path)
        .map_err(|e| SdError::Other(format!("Failed to cache image: {e}")))?;

    // Get file metadata for DB entry
    let metadata = std::fs::metadata(&cache_path)
        .map_err(|e| SdError::Other(format!("Failed to get cached image metadata: {e}")))?;

    // Save to database
    let cached = snapshot::CachedImage::new(
        hash.to_string(),
        Some(source_path.to_path_buf()),
        0, // Width/height could be extracted from image
        0,
        "png".to_string(), // Format detection could be added
        metadata.len(),
    );
    db.save_image(&cached)?;

    Ok(())
}

/// Applies a cached image to a key.
fn apply_cached_image(
    device: &device::Device,
    key: u8,
    hash: &str,
    source_path: Option<&std::path::PathBuf>,
) -> Result<()> {
    // Try cache first
    let cache_path = snapshot::image_cache_path(hash)?;
    if cache_path.exists() {
        device::set_key_image(device, key, &cache_path, image_ops::ResizeStrategy::Fit)?;
        if let Some(path) = source_path {
            state::record::set_key(key, path.clone());
        }
        return Ok(());
    }

    // Fall back to source path
    if let Some(path) = source_path {
        if path.exists() {
            device::set_key_image(device, key, path, image_ops::ResizeStrategy::Fit)?;
            state::record::set_key(key, path.clone());
            return Ok(());
        }
    }

    Err(SdError::Other(format!(
        "Image not found in cache or at original path (hash: {hash})"
    )))
}

#[allow(clippy::unnecessary_wraps)] // Will return errors when implemented
fn cmd_serve(cli: &Cli, args: &cli::ServeArgs) -> Result<()> {
    let _ = (cli, args); // TODO: implement
    eprintln!("Web server not yet implemented");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Consistent return type with other commands
fn cmd_version(_cli: &Cli, output: &dyn Output) -> Result<()> {
    let git_sha = if build_info::git_dirty() == "true" {
        format!("{} (dirty)", build_info::git_sha())
    } else {
        build_info::git_sha().to_string()
    };
    output.version_info(
        build_info::VERSION,
        Some(&git_sha),
        Some(build_info::build_timestamp()),
    );
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
