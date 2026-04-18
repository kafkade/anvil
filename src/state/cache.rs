//! Package cache module
//!
//! This module provides caching for winget query results to avoid
//! repeated expensive operations when checking package status.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::get_cache_dir;

/// Default cache TTL (time-to-live) in minutes
const DEFAULT_CACHE_TTL_MINUTES: i64 = 5;

/// Information about a cached package
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPackageInfo {
    /// Package ID
    pub id: String,
    /// Display name (if available)
    pub name: Option<String>,
    /// Installed version (if installed)
    pub installed_version: Option<String>,
    /// Available version (latest)
    pub available_version: Option<String>,
    /// Whether the package is installed
    pub is_installed: bool,
    /// Source (e.g., "winget", "msstore")
    pub source: Option<String>,
    /// When this entry was cached
    pub cached_at: DateTime<Utc>,
}

impl CachedPackageInfo {
    /// Create a new cached package info for an installed package
    pub fn installed(
        id: impl Into<String>,
        version: impl Into<String>,
        source: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: None,
            installed_version: Some(version.into()),
            available_version: None,
            is_installed: true,
            source,
            cached_at: Utc::now(),
        }
    }

    /// Create a new cached package info for a not-installed package
    pub fn not_installed(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            installed_version: None,
            available_version: None,
            is_installed: false,
            source: None,
            cached_at: Utc::now(),
        }
    }

    /// Check if the cache entry is still valid
    pub fn is_valid(&self, ttl_minutes: i64) -> bool {
        let ttl = chrono::Duration::minutes(ttl_minutes);
        let age = Utc::now().signed_duration_since(self.cached_at);
        age < ttl
    }

    /// Check if an update is available
    pub fn has_update(&self) -> bool {
        if let (Some(installed), Some(available)) =
            (&self.installed_version, &self.available_version)
        {
            installed != available
        } else {
            false
        }
    }
}

/// Package cache for storing winget query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCache {
    /// Cached package information indexed by package ID (lowercase)
    packages: HashMap<String, CachedPackageInfo>,
    /// When the cache was last updated
    pub last_updated: DateTime<Utc>,
    /// When the cache was created
    pub created_at: DateTime<Utc>,
    /// Cache TTL in minutes
    #[serde(default = "default_ttl")]
    pub ttl_minutes: i64,
}

fn default_ttl() -> i64 {
    DEFAULT_CACHE_TTL_MINUTES
}

impl Default for PackageCache {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl PackageCache {
    /// Create a new empty package cache
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            packages: HashMap::new(),
            last_updated: now,
            created_at: now,
            ttl_minutes: DEFAULT_CACHE_TTL_MINUTES,
        }
    }

    /// Create a new package cache with custom TTL
    pub fn with_ttl(ttl_minutes: i64) -> Self {
        let mut cache = Self::new();
        cache.ttl_minutes = ttl_minutes;
        cache
    }

    /// Load the package cache from disk
    pub fn load() -> Result<Self> {
        let cache_file = Self::cache_file_path()?;

        if !cache_file.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&cache_file)
            .with_context(|| format!("Failed to read cache file: {}", cache_file.display()))?;

        let cache: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse cache file: {}", cache_file.display()))?;

        Ok(cache)
    }

    /// Save the package cache to disk
    pub fn save(&self) -> Result<()> {
        let cache_file = Self::cache_file_path()?;

        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize package cache")?;

        std::fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write cache file: {}", cache_file.display()))?;

        Ok(())
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.packages.clear();
        self.last_updated = Utc::now();
    }

    /// Delete the cache file from disk
    pub fn delete() -> Result<()> {
        let cache_file = Self::cache_file_path()?;

        if cache_file.exists() {
            std::fs::remove_file(&cache_file).with_context(|| {
                format!("Failed to delete cache file: {}", cache_file.display())
            })?;
        }

        Ok(())
    }

    /// Get the path to the cache file
    fn cache_file_path() -> Result<PathBuf> {
        let cache_dir = get_cache_dir()?;
        Ok(cache_dir.join("packages.json"))
    }

    /// Get cached info for a package using a provider-scoped key.
    /// Falls back to unscoped (legacy) key if the scoped key is not found,
    /// ensuring backward compatibility with existing cache files.
    pub fn get(&self, package_id: &str) -> Option<&CachedPackageInfo> {
        let key = package_id.to_lowercase();
        let info = self.packages.get(&key)?;

        // Check if the cache entry is still valid
        if info.is_valid(self.ttl_minutes) {
            Some(info)
        } else {
            None
        }
    }

    /// Get cached info for a package using a provider-scoped key.
    /// Tries the scoped key `provider:id` first, then falls back to the
    /// legacy unscoped key `id` for backward compatibility.
    pub fn get_scoped(&self, provider: &str, package_id: &str) -> Option<&CachedPackageInfo> {
        let scoped_key = crate::providers::cache_key(provider, package_id);
        if let Some(info) = self.packages.get(&scoped_key) {
            if info.is_valid(self.ttl_minutes) {
                return Some(info);
            }
        }
        // Fallback to legacy unscoped key
        self.get(package_id)
    }

    /// Check if a package is cached and the entry is valid
    pub fn contains(&self, package_id: &str) -> bool {
        self.get(package_id).is_some()
    }

    /// Set cached info for a package (legacy unscoped key)
    pub fn set(&mut self, info: CachedPackageInfo) {
        let key = info.id.to_lowercase();
        self.packages.insert(key, info);
        self.last_updated = Utc::now();
    }

    /// Set cached info for a package using a provider-scoped key
    pub fn set_scoped(&mut self, provider: &str, info: CachedPackageInfo) {
        let key = crate::providers::cache_key(provider, &info.id);
        self.packages.insert(key, info);
        self.last_updated = Utc::now();
    }

    /// Remove a package from the cache
    pub fn remove(&mut self, package_id: &str) {
        let key = package_id.to_lowercase();
        self.packages.remove(&key);
        self.last_updated = Utc::now();
    }

    /// Invalidate all entries (remove expired entries)
    pub fn invalidate_expired(&mut self) {
        let ttl = self.ttl_minutes;
        self.packages.retain(|_, info| info.is_valid(ttl));
        self.last_updated = Utc::now();
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// Get all cached packages
    pub fn all_packages(&self) -> impl Iterator<Item = &CachedPackageInfo> {
        self.packages.values()
    }

    /// Get all valid cached packages (not expired)
    pub fn valid_packages(&self) -> impl Iterator<Item = &CachedPackageInfo> {
        let ttl = self.ttl_minutes;
        self.packages.values().filter(move |p| p.is_valid(ttl))
    }

    /// Bulk update cache from a list of installed packages
    pub fn update_from_installed(&mut self, packages: Vec<CachedPackageInfo>) {
        for pkg in packages {
            self.set(pkg);
        }
    }

    /// Mark a package as installed (update or create entry) — legacy unscoped
    pub fn mark_installed(&mut self, package_id: &str, version: &str, source: Option<String>) {
        let key = package_id.to_lowercase();

        if let Some(existing) = self.packages.get_mut(&key) {
            existing.is_installed = true;
            existing.installed_version = Some(version.to_string());
            existing.source = source;
            existing.cached_at = Utc::now();
        } else {
            self.set(CachedPackageInfo::installed(package_id, version, source));
        }
    }

    /// Mark a package as installed using a provider-scoped cache key
    pub fn mark_installed_scoped(
        &mut self,
        provider: &str,
        package_id: &str,
        version: &str,
        source: Option<String>,
    ) {
        let key = crate::providers::cache_key(provider, package_id);

        if let Some(existing) = self.packages.get_mut(&key) {
            existing.is_installed = true;
            existing.installed_version = Some(version.to_string());
            existing.source = source;
            existing.cached_at = Utc::now();
        } else {
            self.set_scoped(
                provider,
                CachedPackageInfo::installed(package_id, version, source),
            );
        }
    }

    /// Mark a package as not installed — legacy unscoped
    pub fn mark_not_installed(&mut self, package_id: &str) {
        let key = package_id.to_lowercase();

        if let Some(existing) = self.packages.get_mut(&key) {
            existing.is_installed = false;
            existing.installed_version = None;
            existing.cached_at = Utc::now();
        } else {
            self.set(CachedPackageInfo::not_installed(package_id));
        }
    }

    /// Mark a package as not installed using a provider-scoped cache key
    pub fn mark_not_installed_scoped(&mut self, provider: &str, package_id: &str) {
        let key = crate::providers::cache_key(provider, package_id);

        if let Some(existing) = self.packages.get_mut(&key) {
            existing.is_installed = false;
            existing.installed_version = None;
            existing.cached_at = Utc::now();
        } else {
            self.set_scoped(provider, CachedPackageInfo::not_installed(package_id));
        }
    }

    /// Get list of packages with available updates
    pub fn packages_with_updates(&self) -> Vec<&CachedPackageInfo> {
        self.valid_packages().filter(|p| p.has_update()).collect()
    }

    /// Get statistics about the cache
    pub fn stats(&self) -> CacheStats {
        let ttl = self.ttl_minutes;
        let total = self.packages.len();
        let valid = self.packages.values().filter(|p| p.is_valid(ttl)).count();
        let expired = total - valid;
        let installed = self
            .packages
            .values()
            .filter(|p| p.is_installed && p.is_valid(ttl))
            .count();
        let with_updates = self.packages_with_updates().len();

        CacheStats {
            total_entries: total,
            valid_entries: valid,
            expired_entries: expired,
            installed_count: installed,
            with_updates_count: with_updates,
            ttl_minutes: ttl,
            last_updated: self.last_updated,
        }
    }
}

/// Statistics about the cache
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    /// Total number of entries in cache
    pub total_entries: usize,
    /// Number of valid (not expired) entries
    pub valid_entries: usize,
    /// Number of expired entries
    pub expired_entries: usize,
    /// Number of installed packages
    pub installed_count: usize,
    /// Number of packages with available updates
    pub with_updates_count: usize,
    /// Cache TTL in minutes
    pub ttl_minutes: i64,
    /// When the cache was last updated
    pub last_updated: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_package_info() {
        let info = CachedPackageInfo::installed("Git.Git", "2.43.0", Some("winget".to_string()));
        assert!(info.is_installed);
        assert_eq!(info.installed_version, Some("2.43.0".to_string()));
        assert!(info.is_valid(DEFAULT_CACHE_TTL_MINUTES));
    }

    #[test]
    fn test_cache_operations() {
        let mut cache = PackageCache::new();

        // Add a package
        cache.set(CachedPackageInfo::installed(
            "Git.Git",
            "2.43.0",
            Some("winget".to_string()),
        ));

        assert!(cache.contains("Git.Git"));
        assert!(cache.contains("git.git")); // Case insensitive

        let info = cache.get("Git.Git").unwrap();
        assert!(info.is_installed);
        assert_eq!(info.installed_version, Some("2.43.0".to_string()));

        // Remove the package
        cache.remove("Git.Git");
        assert!(!cache.contains("Git.Git"));
    }

    #[test]
    fn test_cache_expiration() {
        let mut info = CachedPackageInfo::installed("Test.Package", "1.0.0", None);
        // Set cached_at to 10 minutes ago
        info.cached_at = Utc::now() - chrono::Duration::minutes(10);

        // Should be invalid with 5 minute TTL
        assert!(!info.is_valid(5));

        // Should be valid with 15 minute TTL
        assert!(info.is_valid(15));
    }

    #[test]
    fn test_mark_installed() {
        let mut cache = PackageCache::new();

        // Mark as not installed first
        cache.mark_not_installed("Git.Git");
        assert!(!cache.get("Git.Git").unwrap().is_installed);

        // Then mark as installed
        cache.mark_installed("Git.Git", "2.43.0", Some("winget".to_string()));
        let info = cache.get("Git.Git").unwrap();
        assert!(info.is_installed);
        assert_eq!(info.installed_version, Some("2.43.0".to_string()));
    }

    #[test]
    fn test_has_update() {
        let mut info = CachedPackageInfo::installed("Test.Package", "1.0.0", None);
        info.available_version = Some("2.0.0".to_string());

        assert!(info.has_update());

        info.available_version = Some("1.0.0".to_string());
        assert!(!info.has_update());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = PackageCache::new();

        cache.set(CachedPackageInfo::installed("Pkg1", "1.0.0", None));
        cache.set(CachedPackageInfo::not_installed("Pkg2"));

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.installed_count, 1);
    }
}
