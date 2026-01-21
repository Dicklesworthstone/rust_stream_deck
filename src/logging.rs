//! Structured logging initialization for the Stream Deck CLI.
//!
//! Supports both human-friendly and machine-readable (JSON) output formats,
//! with proper TTY detection and verbosity control.

use std::io::{self, IsTerminal};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialize the tracing subscriber based on CLI flags and environment.
///
/// # Arguments
///
/// * `robot_mode` - If true, output structured JSON logs for machine consumption
/// * `verbose` - Verbosity level: 0 = info, 1 = debug, 2+ = trace
/// * `quiet` - If true, suppress non-essential output (only errors)
///
/// # Environment Variables
///
/// * `RUST_LOG` - Override default filter (e.g., "sd=debug,elgato_streamdeck=warn")
///
/// # Output Behavior
///
/// | Mode | TTY | Output |
/// |------|-----|--------|
/// | Robot | any | JSON lines to stderr |
/// | Human | yes | Pretty colored output to stderr |
/// | Human | no | Compact plain output to stderr |
pub fn init_logging(robot_mode: bool, verbose: u8, quiet: bool) {
    // Build the filter directive based on verbosity
    let default_directive = if quiet {
        "sd=error"
    } else {
        match verbose {
            0 => "sd=info",
            1 => "sd=debug",
            _ => "sd=trace",
        }
    };

    // Allow RUST_LOG to override, but use our default otherwise
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_directive));

    if robot_mode {
        // JSON output for AI agents and scripts
        let fmt_layer = fmt::layer()
            .json()
            .with_target(true)
            .with_file(false)
            .with_line_number(false)
            .with_thread_ids(false)
            .with_span_events(FmtSpan::NONE)
            .with_writer(io::stderr);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    } else if io::stderr().is_terminal() {
        // Pretty output for interactive terminals
        let fmt_layer = fmt::layer()
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .with_thread_ids(false)
            .with_span_events(FmtSpan::NONE)
            .with_writer(io::stderr);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    } else {
        // Compact output for non-TTY (piped, redirected)
        let fmt_layer = fmt::layer()
            .with_ansi(false)
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .with_thread_ids(false)
            .with_span_events(FmtSpan::NONE)
            .compact()
            .with_writer(io::stderr);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: We can't easily test tracing initialization in unit tests
    // since the global subscriber can only be set once. Integration tests
    // should verify logging behavior.

    #[test]
    fn test_filter_directives() {
        // Just verify the filter parsing works
        assert!(EnvFilter::try_new("sd=info").is_ok());
        assert!(EnvFilter::try_new("sd=debug").is_ok());
        assert!(EnvFilter::try_new("sd=trace").is_ok());
        assert!(EnvFilter::try_new("sd=error").is_ok());
        assert!(EnvFilter::try_new("sd=debug,elgato_streamdeck=warn").is_ok());
    }
}
