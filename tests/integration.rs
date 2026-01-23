//! Integration tests for the Stream Deck CLI.
//!
//! These tests verify component interactions without real hardware,
//! using the mock device and test fixtures.
//!
//! # Modules
//!
//! - `device_operations`: Tests for device operations using MockDevice
//! - `image_processing`: Tests for image loading and resizing
//! - `config_parsing`: Tests for configuration file parsing
//! - `state_management`: Tests for session state tracking

#[path = "integration/config_parsing.rs"]
mod config_parsing;

#[path = "integration/device_operations.rs"]
mod device_operations;

#[path = "integration/image_processing.rs"]
mod image_processing;

#[path = "integration/state_management.rs"]
mod state_management;
