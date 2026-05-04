//! Workload registry for community discovery
//!
//! This module defines the registry index format and provides
//! cache management for the remote registry.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default registry URL (GitHub raw content)
pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/kafkade/anvil-registry/main/registry.json";

/// Cache TTL in seconds (1 hour)
const CACHE_TTL_SECS: i64 = 3600;

/// A single entry in the workload registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Unique workload name (used as source name when adding)
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Git repository URL
    pub url: String,

    /// Author or maintainer
    pub author: String,

    /// Searchable tags
    #[serde(default)]
    pub tags: Vec<String>,

    /// Minimum Anvil version required (semver, e.g. "1.1.0")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_anvil_version: Option<String>,

    /// Git ref to track (branch or tag)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,

    /// Subdirectory within the repo containing workloads
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workload_subdir: Option<String>,
}

/// The registry index — a versioned list of registry entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    /// Schema version (for forward compatibility)
    pub version: String,

    /// Available workloads
    pub entries: Vec<RegistryEntry>,
}

/// Cached registry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRegistry {
    /// When the cache was last fetched
    pub fetched_at: DateTime<Utc>,

    /// The registry index data
    pub index: RegistryIndex,
}

impl CachedRegistry {
    /// Check if the cache is still fresh
    pub fn is_fresh(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.fetched_at);
        age.num_seconds() < CACHE_TTL_SECS
    }
}

/// Get the path to the registry cache file
pub fn registry_cache_path() -> Result<PathBuf> {
    let cache_dir = crate::state::get_cache_dir()?;
    Ok(cache_dir.join("registry.json"))
}

/// Load the cached registry index, if it exists and is valid
pub fn load_cache() -> Result<Option<CachedRegistry>> {
    let path = registry_cache_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read registry cache: {}", path.display()))?;

    let cached: CachedRegistry = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse registry cache: {}", path.display()))?;

    Ok(Some(cached))
}

/// Save a registry index to the cache
pub fn save_cache(index: &RegistryIndex) -> Result<()> {
    let path = registry_cache_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create cache directory: {}", parent.display()))?;
    }

    let cached = CachedRegistry {
        fetched_at: Utc::now(),
        index: index.clone(),
    };

    let content =
        serde_json::to_string_pretty(&cached).context("Failed to serialize registry cache")?;

    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write registry cache: {}", path.display()))?;

    Ok(())
}

/// Check if the current Anvil version satisfies a minimum version requirement
pub fn version_satisfies(min_version: &str) -> bool {
    let current = env!("CARGO_PKG_VERSION");
    version_cmp(current, min_version) >= std::cmp::Ordering::Equal
}

/// Compare two semver-like version strings
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .filter_map(|part| part.parse::<u64>().ok())
            .collect()
    };

    let va = parse(a);
    let vb = parse(b);

    for i in 0..va.len().max(vb.len()) {
        let pa = va.get(i).copied().unwrap_or(0);
        let pb = vb.get(i).copied().unwrap_or(0);
        match pa.cmp(&pb) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    std::cmp::Ordering::Equal
}

/// Search registry entries by query string
///
/// Matches against name, description, author, and tags (case-insensitive substring).
pub fn search_entries(entries: &[RegistryEntry], query: &str) -> Vec<RegistryEntry> {
    let query_lower = query.to_lowercase();

    entries
        .iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&query_lower)
                || e.description.to_lowercase().contains(&query_lower)
                || e.author.to_lowercase().contains(&query_lower)
                || e.tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<RegistryEntry> {
        vec![
            RegistryEntry {
                name: "rust-developer".to_string(),
                description: "Rust toolchain with cargo tools".to_string(),
                url: "https://github.com/example/rust-workloads.git".to_string(),
                author: "anvil-community".to_string(),
                tags: vec!["rust".to_string(), "development".to_string()],
                min_anvil_version: Some("1.0.0".to_string()),
                git_ref: None,
                workload_subdir: None,
            },
            RegistryEntry {
                name: "python-data-science".to_string(),
                description: "Python data science environment with Jupyter".to_string(),
                url: "https://github.com/example/python-ds.git".to_string(),
                author: "data-team".to_string(),
                tags: vec![
                    "python".to_string(),
                    "data-science".to_string(),
                    "jupyter".to_string(),
                ],
                min_anvil_version: None,
                git_ref: Some("main".to_string()),
                workload_subdir: Some("workloads".to_string()),
            },
            RegistryEntry {
                name: "devops-tools".to_string(),
                description: "DevOps and cloud CLI tools".to_string(),
                url: "https://github.com/example/devops.git".to_string(),
                author: "ops-team".to_string(),
                tags: vec![
                    "devops".to_string(),
                    "cloud".to_string(),
                    "docker".to_string(),
                ],
                min_anvil_version: Some("0.5.0".to_string()),
                git_ref: None,
                workload_subdir: None,
            },
        ]
    }

    #[test]
    fn test_search_by_name() {
        let entries = sample_entries();
        let results = search_entries(&entries, "rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-developer");
    }

    #[test]
    fn test_search_by_tag() {
        let entries = sample_entries();
        let results = search_entries(&entries, "docker");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "devops-tools");
    }

    #[test]
    fn test_search_by_description() {
        let entries = sample_entries();
        let results = search_entries(&entries, "jupyter");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "python-data-science");
    }

    #[test]
    fn test_search_by_author() {
        let entries = sample_entries();
        let results = search_entries(&entries, "data-team");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "python-data-science");
    }

    #[test]
    fn test_search_case_insensitive() {
        let entries = sample_entries();
        let results = search_entries(&entries, "RUST");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_no_results() {
        let entries = sample_entries();
        let results = search_entries(&entries, "nonexistent-xyz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_broad_match() {
        let entries = sample_entries();
        // "development" appears in rust-developer's tags
        let results = search_entries(&entries, "development");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_version_satisfies() {
        assert!(version_satisfies("0.1.0"));
        assert!(version_satisfies("1.0.0"));
        assert!(version_satisfies("1.1.0"));
    }

    #[test]
    fn test_version_cmp() {
        use std::cmp::Ordering;
        assert_eq!(version_cmp("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(version_cmp("1.1.0", "1.0.0"), Ordering::Greater);
        assert_eq!(version_cmp("0.9.0", "1.0.0"), Ordering::Less);
        assert_eq!(version_cmp("1.0.1", "1.0.0"), Ordering::Greater);
        assert_eq!(version_cmp("2.0.0", "1.99.99"), Ordering::Greater);
    }

    #[test]
    fn test_registry_index_serialization() {
        let index = RegistryIndex {
            version: "1".to_string(),
            entries: sample_entries(),
        };

        let json = serde_json::to_string_pretty(&index).unwrap();
        let parsed: RegistryIndex = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, "1");
        assert_eq!(parsed.entries.len(), 3);
        assert_eq!(parsed.entries[0].name, "rust-developer");
    }

    #[test]
    fn test_cached_registry_freshness() {
        let cached = CachedRegistry {
            fetched_at: Utc::now(),
            index: RegistryIndex {
                version: "1".to_string(),
                entries: vec![],
            },
        };
        assert!(cached.is_fresh());

        let old_cached = CachedRegistry {
            fetched_at: Utc::now() - chrono::Duration::hours(2),
            index: RegistryIndex {
                version: "1".to_string(),
                entries: vec![],
            },
        };
        assert!(!old_cached.is_fresh());
    }
}
