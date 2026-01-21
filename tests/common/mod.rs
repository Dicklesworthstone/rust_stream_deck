//! Common test utilities for the Stream Deck CLI.
//!
//! This module provides infrastructure for end-to-end CLI testing with:
//! - `cli`: CLI runner with output verification and fluent assertions
//! - `fixtures`: Test image and config file generation
//! - `logging`: Log output verification helpers

pub mod cli;
pub mod fixtures;
pub mod logging;
