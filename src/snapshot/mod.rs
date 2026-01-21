//! Snapshot storage for device state persistence.
//!
//! This module provides persistent storage for device state snapshots,
//! allowing users to save and restore complete device layouts.
//!
//! # Directory Structure
//!
//! ```text
//! ~/.local/share/sd/
//! └── snapshots/
//!     ├── snapshots.db           # SQLite database
//!     └── images/                # Content-addressable image cache
//!         ├── aa/
//!         │   └── aabbcc...123.webp
//!         └── bb/
//!             └── bbccdd...456.webp
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use rust_stream_deck::snapshot::{SnapshotDb, Snapshot, SnapshotKey};
//!
//! // Open database
//! let mut db = SnapshotDb::open_default()?;
//!
//! // Create and save a snapshot
//! let mut snap = Snapshot::new("work-mode".to_string(), "StreamDeckXL".to_string(), 32, 96, 96);
//! snap.add_key(SnapshotKey::image(0, Some(path), hash));
//! db.save_snapshot(&snap)?;
//!
//! // Load a snapshot
//! if let Some(snap) = db.load_snapshot("work-mode")? {
//!     // Apply to device...
//! }
//!
//! // List all snapshots
//! for summary in db.list_snapshots()? {
//!     println!("{}: {}", summary.name, summary.device_model);
//! }
//! ```

mod db;
mod schema;

pub use db::{
    default_db_path, default_image_cache_dir, image_cache_path, SnapshotDb,
};
pub use schema::{
    CachedImage, KeyState, Snapshot, SnapshotKey, SnapshotSummary,
};
