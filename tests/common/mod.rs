//! Common test utilities for the Stream Deck CLI.
//!
//! This module provides infrastructure for end-to-end CLI testing with:
//! - `cli`: CLI runner with output verification and fluent assertions
//! - `fixtures`: Test image and config file generation
//! - `logging`: Log output verification helpers
#![allow(dead_code)]

pub mod cli;
pub mod capture;
pub mod env;
pub mod fixtures;
pub mod logging;
pub mod mocks;
pub mod assertions;

use rich_rust::prelude::{ColorSystem, Console};
use tracing_subscriber::EnvFilter;

#[must_use]
pub fn test_console() -> Console {
    Console::builder()
        .force_terminal(true)
        .color_system(ColorSystem::TrueColor)
        .width(80)
        .build()
}

#[must_use]
pub fn test_console_no_color() -> Console {
    Console::builder()
        .force_terminal(true)
        .no_color()
        .width(80)
        .build()
}

#[must_use]
pub fn test_console_ascii() -> Console {
    Console::builder()
        .force_terminal(true)
        .safe_box(true)
        .width(80)
        .build()
}

pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();
}
