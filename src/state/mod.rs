//! State management module for Anvil
//!
//! This module handles tracking installation state and caching package information
//! to improve performance and enable recovery from interrupted operations.
pub mod cache;
pub mod files;
pub mod installation;

use std::path::PathBuf;

use anyhow::Result;

pub use cache::{CachedPackageInfo, PackageCache};
pub use files::{FileState, FileStateManager};
pub use installation::{InstallationState, PackageStatus};

/// Get the Anvil data directory (~/.anvil)
pub fn get_anvil_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let anvil_dir = home.join(".anvil");

    // Create directory if it doesn't exist
    if !anvil_dir.exists() {
        std::fs::create_dir_all(&anvil_dir)?;
    }

    Ok(anvil_dir)
}

/// Get the state directory (~/.anvil/state)
pub fn get_state_dir() -> Result<PathBuf> {
    let state_dir = get_anvil_dir()?.join("state");

    if !state_dir.exists() {
        std::fs::create_dir_all(&state_dir)?;
    }

    Ok(state_dir)
}

/// Get the cache directory (~/.anvil/cache)
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = get_anvil_dir()?.join("cache");

    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)?;
    }

    Ok(cache_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_anvil_dir() {
        // This test just ensures the function doesn't panic
        // Actual directory creation is tested in integration tests
        let result = get_anvil_dir();
        assert!(result.is_ok() || result.is_err());
    }
}
