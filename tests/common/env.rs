//! Environment variable helpers for tests.
#![allow(dead_code)]

use env_lock::{EnvGuard as LockedEnvGuard, lock_env};
use tracing::{instrument, trace};

/// RAII guard to restore environment variables on drop.
pub struct EnvGuard<'a> {
    _guard: LockedEnvGuard<'a>,
}

impl<'a> EnvGuard<'a> {
    #[must_use]
    #[instrument]
    pub fn set(key: &'a str, value: &str) -> Self {
        trace!(key, value, "Setting env var");
        let guard = lock_env([(key, Some(value))]);
        Self { _guard: guard }
    }

    #[must_use]
    #[instrument]
    pub fn remove(key: &'a str) -> Self {
        trace!(key, "Removing env var");
        let guard = lock_env([(key, None::<&str>)]);
        Self { _guard: guard }
    }
}

#[must_use]
pub fn with_no_color() -> EnvGuard<'static> {
    EnvGuard::set("NO_COLOR", "1")
}

#[must_use]
pub fn with_force_color() -> EnvGuard<'static> {
    EnvGuard::set("FORCE_COLOR", "1")
}

#[must_use]
pub fn with_sd_format(format: &str) -> EnvGuard<'static> {
    EnvGuard::set("SD_FORMAT", format)
}
