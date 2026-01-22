//! Configuration module for Stream Deck profiles.
//!
//! Handles loading, saving, and managing Stream Deck profiles.
//! Supports both the official Elgato `.streamDeckProfile` format (ZIP archives)
//! and our internal `SQLite` storage format.

mod db;
mod loader;
mod path;
mod schema;

// Re-export schema types for use by other modules
#[allow(unused_imports)] // Types are for future use
pub use schema::{
    Action, ActionSettings, ActionState, Device, Image, Page, Plugin, Profile, ProfilePackage,
};

// Re-export database types
#[allow(unused_imports)] // Types are for future use
pub use db::{PackageRow, ProfileDb};

// Re-export loader types
#[allow(unused_imports)] // Types are for future use
pub use loader::ProfileLoader;

// Re-export path helpers for declarative config support
#[allow(unused_imports)] // Types are for future use
pub use path::{home_dir, resolve_path, validate_image_path, PathResolver};
