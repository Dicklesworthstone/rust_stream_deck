//! Stream Deck CLI library - Cross-platform control for Elgato Stream Deck devices.
//!
//! This library exposes the core functionality of the `sd` CLI for use in tests
//! and potentially other applications.
//!
//! # Modules
//!
//! - `device`: Device abstraction layer for Stream Deck hardware
//! - `error`: Error types with user-recoverable hints
//! - `output`: Output mode abstraction (robot/human)
//! - `batch`: Batch operations support
//! - `config`: Configuration file handling
//! - `snapshot`: Device state snapshots
#![forbid(unsafe_code)]

pub mod batch;
pub mod cli;
pub mod config;
pub mod device;
pub mod error;
pub mod image_ops;
pub mod logging;
pub mod output;
pub mod snapshot;
pub mod state;
pub mod theme;
