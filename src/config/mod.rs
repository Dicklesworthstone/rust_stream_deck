//! Configuration module for Stream Deck profiles.
//!
//! Handles loading, saving, and managing Stream Deck profiles.
//! Supports both the official Elgato `.streamDeckProfile` format (ZIP archives)
//! and our internal `SQLite` storage format.

mod db;
pub mod declarative;
mod key_config;
mod loader;
mod path;
mod schema;
mod selector;

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
pub use path::{PathResolver, home_dir, resolve_path, validate_image_path};

// Re-export key config types for declarative YAML/TOML configuration
pub use key_config::{ColorSpec, KeyConfig, MissingBehavior, ResolvedKey};

// Re-export key selector types for targeting keys in config
#[allow(unused_imports)] // Used by validate/apply commands (future beads)
pub use selector::KeySelector;
