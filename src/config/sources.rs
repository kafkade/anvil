//! Source configuration management for Anvil
//!
//! This module manages workload sources — both local directories and remote git
//! repositories. Source metadata is persisted in `~/.anvil/sources.json`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of workload source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    /// A local filesystem directory
    Local,
    /// A remote git repository
    Remote,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Local => write!(f, "local"),
            SourceType::Remote => write!(f, "remote"),
        }
    }
}

/// A workload source definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Unique name for this source
    pub name: String,

    /// Type of source (local or remote)
    #[serde(rename = "type")]
    pub source_type: SourceType,

    /// Local filesystem path where workloads are found
    ///
    /// For local sources, this is the user-specified path.
    /// For remote sources, this is the clone destination (e.g. `~/.anvil/sources/<name>/`).
    pub local_path: PathBuf,

    /// Git remote URL (only for remote sources)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Git ref to track (branch name or tag) — only for remote sources
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,

    /// Subdirectory within the source to search for workloads
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workload_subdir: Option<String>,

    /// Timestamp of last successful sync (only for remote sources)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<DateTime<Utc>>,
}

impl Source {
    /// Create a new local source
    pub fn new_local(name: String, local_path: PathBuf) -> Self {
        Self {
            name,
            source_type: SourceType::Local,
            local_path,
            url: None,
            git_ref: None,
            workload_subdir: None,
            last_synced: None,
        }
    }

    /// Create a new remote (git) source
    pub fn new_remote(
        name: String,
        url: String,
        local_path: PathBuf,
        git_ref: Option<String>,
        workload_subdir: Option<String>,
    ) -> Self {
        Self {
            name,
            source_type: SourceType::Remote,
            local_path,
            url: Some(url),
            git_ref,
            workload_subdir,
            last_synced: None,
        }
    }

    /// Get the effective path where workloads should be discovered
    pub fn workload_path(&self) -> PathBuf {
        if let Some(ref subdir) = self.workload_subdir {
            self.local_path.join(subdir)
        } else {
            self.local_path.clone()
        }
    }
}

/// Sources configuration — persisted at `~/.anvil/sources.json`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourcesConfig {
    /// List of configured sources
    pub sources: Vec<Source>,
}

impl SourcesConfig {
    /// Get the path to the sources configuration file
    pub fn sources_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot find home directory")?;
        Ok(home.join(".anvil").join("sources.json"))
    }

    /// Load sources configuration from file
    ///
    /// Returns default (empty) configuration if the file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::sources_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read sources file: {}", path.display()))?;
            let config: SourcesConfig = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse sources file: {}", path.display()))?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save sources configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::sources_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize sources configuration")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write sources file: {}", path.display()))?;

        Ok(())
    }

    /// Find a source by name
    pub fn find_by_name(&self, name: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.name == name)
    }

    /// Add a source, returning an error if a source with the same name exists
    pub fn add(&mut self, source: Source) -> Result<()> {
        if self.find_by_name(&source.name).is_some() {
            anyhow::bail!("Source '{}' already exists", source.name);
        }
        self.sources.push(source);
        Ok(())
    }

    /// Remove a source by name, returning the removed source if found
    pub fn remove(&mut self, name: &str) -> Option<Source> {
        if let Some(pos) = self.sources.iter().position(|s| s.name == name) {
            Some(self.sources.remove(pos))
        } else {
            None
        }
    }

    /// Get all remote sources
    pub fn remote_sources(&self) -> Vec<&Source> {
        self.sources
            .iter()
            .filter(|s| s.source_type == SourceType::Remote)
            .collect()
    }

    /// Get workload search paths from all sources
    pub fn workload_paths(&self) -> Vec<PathBuf> {
        self.sources.iter().map(|s| s.workload_path()).collect()
    }
}

/// Validate a source name (must be safe for use as a directory name)
pub fn validate_source_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Source name cannot be empty");
    }

    if name.len() > 64 {
        anyhow::bail!("Source name cannot exceed 64 characters");
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        anyhow::bail!(
            "Source name '{}' contains invalid characters. \
             Only alphanumeric characters, dots, underscores, and hyphens are allowed.",
            name
        );
    }

    if name.starts_with('.') || name.starts_with('-') {
        anyhow::bail!("Source name cannot start with a dot or hyphen");
    }

    if name == "." || name == ".." {
        anyhow::bail!("Source name cannot be '.' or '..'");
    }

    Ok(())
}

/// Detect whether a string looks like a git URL
pub fn is_git_url(input: &str) -> bool {
    input.starts_with("https://")
        || input.starts_with("http://")
        || input.starts_with("git@")
        || input.starts_with("ssh://")
        || input.ends_with(".git")
}

/// Extract a repository name from a git URL
///
/// Examples:
/// - `https://github.com/org/repo.git` → `repo`
/// - `git@github.com:org/repo.git` → `repo`
/// - `https://github.com/org/repo` → `repo`
pub fn repo_name_from_url(url: &str) -> Option<String> {
    let cleaned = url.trim_end_matches('/').trim_end_matches(".git");

    // Handle SSH URLs like git@github.com:org/repo
    let path_part = if cleaned.contains(':') && cleaned.starts_with("git@") {
        cleaned.rsplit(':').next()?
    } else {
        cleaned.rsplit('/').next()?
    };

    // Take the last component after any remaining slashes
    let name = path_part.rsplit('/').next().unwrap_or(path_part);

    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Get the directory where remote sources are cloned
pub fn sources_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot find home directory")?;
    Ok(home.join(".anvil").join("sources"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_url() {
        assert!(is_git_url("https://github.com/org/repo.git"));
        assert!(is_git_url("https://github.com/org/repo"));
        assert!(is_git_url("http://example.com/repo.git"));
        assert!(is_git_url("git@github.com:org/repo.git"));
        assert!(is_git_url("ssh://git@github.com/org/repo"));
        assert!(is_git_url("my-repo.git"));

        assert!(!is_git_url("/home/user/workloads"));
        assert!(!is_git_url("C:\\Users\\user\\workloads"));
        assert!(!is_git_url("./local-dir"));
        assert!(!is_git_url("some-name"));
    }

    #[test]
    fn test_repo_name_from_url() {
        assert_eq!(
            repo_name_from_url("https://github.com/org/repo.git"),
            Some("repo".to_string())
        );
        assert_eq!(
            repo_name_from_url("https://github.com/org/repo"),
            Some("repo".to_string())
        );
        assert_eq!(
            repo_name_from_url("git@github.com:org/repo.git"),
            Some("repo".to_string())
        );
        assert_eq!(
            repo_name_from_url("https://github.com/org/my-workloads/"),
            Some("my-workloads".to_string())
        );
        assert_eq!(
            repo_name_from_url("ssh://git@github.com/org/repo"),
            Some("repo".to_string())
        );
    }

    #[test]
    fn test_validate_source_name() {
        assert!(validate_source_name("my-source").is_ok());
        assert!(validate_source_name("source_1").is_ok());
        assert!(validate_source_name("org.repo").is_ok());
        assert!(validate_source_name("MySource123").is_ok());

        assert!(validate_source_name("").is_err());
        assert!(validate_source_name("..").is_err());
        assert!(validate_source_name(".hidden").is_err());
        assert!(validate_source_name("-leading").is_err());
        assert!(validate_source_name("path/traversal").is_err());
        assert!(validate_source_name("has spaces").is_err());
        assert!(validate_source_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_source_workload_path() {
        let source = Source::new_local("test".to_string(), PathBuf::from("/some/path"));
        assert_eq!(source.workload_path(), PathBuf::from("/some/path"));

        let mut source = Source::new_remote(
            "test".to_string(),
            "https://example.com/repo.git".to_string(),
            PathBuf::from("/clone/path"),
            None,
            Some("workloads".to_string()),
        );
        assert_eq!(
            source.workload_path(),
            PathBuf::from("/clone/path/workloads")
        );

        source.workload_subdir = None;
        assert_eq!(source.workload_path(), PathBuf::from("/clone/path"));
    }

    #[test]
    fn test_sources_config_add_remove() {
        let mut config = SourcesConfig::default();

        let source = Source::new_local("test".to_string(), PathBuf::from("/some/path"));
        config.add(source).unwrap();

        assert_eq!(config.sources.len(), 1);
        assert!(config.find_by_name("test").is_some());
        assert!(config.find_by_name("nonexistent").is_none());

        // Duplicate add should fail
        let dup = Source::new_local("test".to_string(), PathBuf::from("/other/path"));
        assert!(config.add(dup).is_err());

        // Remove
        let removed = config.remove("test");
        assert!(removed.is_some());
        assert_eq!(config.sources.len(), 0);

        // Remove nonexistent
        let removed = config.remove("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_sources_config_remote_sources() {
        let mut config = SourcesConfig::default();

        config
            .add(Source::new_local(
                "local1".to_string(),
                PathBuf::from("/local"),
            ))
            .unwrap();
        config
            .add(Source::new_remote(
                "remote1".to_string(),
                "https://example.com/repo.git".to_string(),
                PathBuf::from("/clone"),
                None,
                None,
            ))
            .unwrap();

        let remotes = config.remote_sources();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "remote1");
    }

    #[test]
    fn test_sources_config_workload_paths() {
        let mut config = SourcesConfig::default();

        config
            .add(Source::new_local(
                "local1".to_string(),
                PathBuf::from("/local/path"),
            ))
            .unwrap();
        config
            .add(Source::new_remote(
                "remote1".to_string(),
                "https://example.com/repo.git".to_string(),
                PathBuf::from("/clone/path"),
                None,
                Some("workloads".to_string()),
            ))
            .unwrap();

        let paths = config.workload_paths();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("/local/path"));
        assert_eq!(paths[1], PathBuf::from("/clone/path/workloads"));
    }

    #[test]
    fn test_source_serialization_roundtrip() {
        let mut config = SourcesConfig::default();
        config
            .add(Source::new_local(
                "local1".to_string(),
                PathBuf::from("/local/path"),
            ))
            .unwrap();
        config
            .add(Source::new_remote(
                "remote1".to_string(),
                "https://github.com/org/repo.git".to_string(),
                PathBuf::from("/clone/path"),
                Some("main".to_string()),
                Some("workloads".to_string()),
            ))
            .unwrap();

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: SourcesConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.sources.len(), 2);
        assert_eq!(deserialized.sources[0].name, "local1");
        assert_eq!(deserialized.sources[0].source_type, SourceType::Local);
        assert_eq!(deserialized.sources[1].name, "remote1");
        assert_eq!(deserialized.sources[1].source_type, SourceType::Remote);
        assert_eq!(
            deserialized.sources[1].url,
            Some("https://github.com/org/repo.git".to_string())
        );
        assert_eq!(deserialized.sources[1].git_ref, Some("main".to_string()));
    }
}
