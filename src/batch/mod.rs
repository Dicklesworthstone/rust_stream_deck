//! Batch operations for Stream Deck key management.
//!
//! This module provides functionality for batch operations like setting multiple keys
//! from a directory of images.

mod scanner;

pub use scanner::{ScanResult, scan_directory};
