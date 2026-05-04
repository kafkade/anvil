//! Configuration module for Anvil
//!
//! This module handles parsing and validation of workload definitions,
//! including support for workload inheritance and variable expansion.
pub mod global;
pub mod inheritance;
pub mod registry;
pub mod schema;
pub mod sources;
pub mod workload;

pub use global::GlobalConfig;
pub use inheritance::InheritanceGraph;
pub use sources::SourcesConfig;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

pub use workload::*;

/// Default workload search paths
pub fn default_workload_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Built-in workloads (relative to executable)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            paths.push(exe_dir.join("workloads"));
        }
    }

    // 2. User workloads directory
    if let Some(data_dir) = dirs::data_local_dir() {
        paths.push(data_dir.join("anvil").join("workloads"));
    }

    // 3. Current directory workloads
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("workloads"));
    }

    paths
}

/// Configuration manager for loading and resolving workloads
#[derive(Debug)]
pub struct ConfigManager {
    /// Search paths for workloads
    search_paths: Vec<PathBuf>,
    /// Cache of loaded workloads
    loaded_workloads: std::collections::HashMap<String, Workload>,
}

impl ConfigManager {
    /// Create a new configuration manager with default search paths
    ///
    /// User-configured paths from GlobalConfig are prepended before defaults,
    /// giving them higher priority during workload discovery.
    /// Managed sources from sources.json are added after user paths but before defaults.
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // 1. User-configured paths from GlobalConfig (highest priority)
        if let Ok(config) = GlobalConfig::load() {
            for path_str in &config.workloads.paths {
                let expanded = expand_variables(path_str, None);
                let path = PathBuf::from(expanded);
                if !search_paths.contains(&path) {
                    search_paths.push(path);
                }
            }
        }

        // 2. Managed sources from sources.json (after user paths, before defaults)
        if let Ok(sources_config) = SourcesConfig::load() {
            for source_path in sources_config.workload_paths() {
                if !search_paths.contains(&source_path) {
                    search_paths.push(source_path);
                }
            }
        }

        // 3. Default search paths (lower priority)
        for path in default_workload_paths() {
            if !search_paths.contains(&path) {
                search_paths.push(path);
            }
        }

        Self {
            search_paths,
            loaded_workloads: HashMap::new(),
        }
    }

    /// Create a configuration manager with custom search paths
    #[cfg(test)]
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths: paths,
            loaded_workloads: HashMap::new(),
        }
    }

    /// Get the current search paths (user-configured + defaults)
    #[cfg(test)]
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Add a search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Find a workload by name or path
    pub fn find_workload(&self, name_or_path: &str) -> Option<PathBuf> {
        let path = Path::new(name_or_path);

        // If it's an existing path (file or directory), use it directly
        if path.exists() {
            if path.is_file() {
                return Some(path.to_path_buf());
            } else if path.is_dir() {
                let workload_file = path.join("workload.yaml");
                if workload_file.exists() {
                    return Some(workload_file);
                }
                // Also check for workload.yml
                let workload_file = path.join("workload.yml");
                if workload_file.exists() {
                    return Some(workload_file);
                }
            }
            return None;
        }

        // Search in configured paths
        for search_path in &self.search_paths {
            // Try: <search_path>/<name>/workload.yaml
            let workload_dir = search_path.join(name_or_path);
            let workload_file = workload_dir.join("workload.yaml");
            if workload_file.exists() {
                return Some(workload_file);
            }

            // Try: <search_path>/<name>/workload.yml
            let workload_file = workload_dir.join("workload.yml");
            if workload_file.exists() {
                return Some(workload_file);
            }

            // Try: <search_path>/<name>.yaml
            let workload_file = search_path.join(format!("{}.yaml", name_or_path));
            if workload_file.exists() {
                return Some(workload_file);
            }
        }

        None
    }

    /// Load a workload from a file
    pub fn load_workload(&mut self, path: &Path) -> Result<Workload> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read workload file: {}", path.display()))?;

        let workload: Workload = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse workload file: {}", path.display()))?;

        // Store the workload's base directory for relative path resolution
        let _base_dir = path.parent().map(|p| p.to_path_buf());

        // Cache the loaded workload
        self.loaded_workloads
            .insert(workload.name.clone(), workload.clone());

        Ok(workload)
    }

    /// Load and resolve a workload by name, including inheritance
    pub fn load_resolved(&mut self, name_or_path: &str) -> Result<Workload> {
        let path = self
            .find_workload(name_or_path)
            .with_context(|| format!("Workload not found: {}", name_or_path))?;

        let workload = self.load_workload(&path)?;

        // Resolve inheritance
        inheritance::resolve_inheritance(self, workload)
    }

    /// List all available workloads with precedence-based conflict resolution
    ///
    /// When the same workload name exists in multiple search paths, the first
    /// match (higher priority path) wins. Shadowed duplicates are tracked in
    /// `WorkloadInfo::shadowed_paths` for reporting.
    pub fn list_workloads(&self) -> Result<Vec<WorkloadInfo>> {
        let mut workloads: Vec<WorkloadInfo> = Vec::new();
        let mut seen: HashMap<String, usize> = HashMap::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            let entries = std::fs::read_dir(search_path)
                .with_context(|| format!("Failed to read directory: {}", search_path.display()))?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let workload_file = path.join("workload.yaml");
                    if workload_file.exists() {
                        if let Ok(info) = self.read_workload_info(&workload_file) {
                            if let Some(&idx) = seen.get(&info.name) {
                                // Duplicate found — the existing entry has higher priority
                                tracing::warn!(
                                    "Workload '{}' in '{}' is shadowed by '{}'",
                                    info.name,
                                    info.path.display(),
                                    workloads[idx].path.display()
                                );
                                workloads[idx].shadowed_paths.push(info.path);
                            } else {
                                seen.insert(info.name.clone(), workloads.len());
                                workloads.push(info);
                            }
                        }
                    }
                }
            }
        }

        // Sort by name
        workloads.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(workloads)
    }

    /// List all workload instances including shadowed duplicates
    ///
    /// Unlike `list_workloads()`, this returns every occurrence of every workload
    /// across all search paths, marking lower-priority duplicates as shadowed.
    pub fn list_all_workloads(&self) -> Result<Vec<WorkloadInfo>> {
        let mut all: Vec<WorkloadInfo> = Vec::new();
        let mut primary_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            let entries = std::fs::read_dir(search_path)
                .with_context(|| format!("Failed to read directory: {}", search_path.display()))?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let workload_file = path.join("workload.yaml");
                    if workload_file.exists() {
                        if let Ok(mut info) = self.read_workload_info(&workload_file) {
                            if primary_names.contains(&info.name) {
                                // Mark as shadowed — record the primary's path
                                if let Some(primary) = all
                                    .iter()
                                    .find(|w| w.name == info.name && w.shadowed_paths.is_empty())
                                {
                                    info.shadowed_paths = vec![primary.path.clone()];
                                }
                            } else {
                                primary_names.insert(info.name.clone());
                            }
                            all.push(info);
                        }
                    }
                }
            }
        }

        // Sort by name, then by whether it's shadowed (primary first)
        all.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.shadowed_paths.len().cmp(&b.shadowed_paths.len()))
        });

        Ok(all)
    }

    /// Read basic workload info without full parsing
    fn read_workload_info(&self, path: &Path) -> Result<WorkloadInfo> {
        let content = std::fs::read_to_string(path)?;
        let workload: Workload = serde_yaml::from_str(&content)?;

        Ok(WorkloadInfo {
            name: workload.name,
            version: workload.version,
            description: workload.description,
            extends: workload.extends.unwrap_or_default(),
            package_count: workload
                .packages
                .as_ref()
                .map(|p| p.winget.as_ref().map(|w| w.len()).unwrap_or(0))
                .unwrap_or(0),
            file_count: workload.files.as_ref().map(|f| f.len()).unwrap_or(0),
            path: path.to_path_buf(),
            shadowed_paths: Vec::new(),
        })
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic workload information for listing
#[derive(Debug, Clone, Serialize)]
pub struct WorkloadInfo {
    /// Workload name
    pub name: String,
    /// Workload version
    pub version: String,
    /// Short description
    pub description: String,
    /// Parent workloads (if any)
    pub extends: Vec<String>,
    /// Number of packages
    pub package_count: usize,
    /// Number of files
    pub file_count: usize,
    /// Path to the workload file
    #[serde(skip_serializing)]
    pub path: PathBuf,
    /// Other paths where this workload was also found (shadowed)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shadowed_paths: Vec<PathBuf>,
}

/// Expand variables in a string
///
/// Supported variables:
/// - `~` or `${HOME}` - User's home directory
/// - `${APPDATA}` - Application data directory (Roaming)
/// - `${LOCALAPPDATA}` - Local application data
/// - `${PROGRAMFILES}` - Program Files directory
/// - `${PROGRAMFILES_X86}` - Program Files (x86) directory
/// - `${DOCUMENTS}` - User's Documents folder
/// - `${DESKTOP}` - User's Desktop folder
/// - `${TEMP}` - Temporary directory
/// - `${USERNAME}` - Current username
/// - `${COMPUTERNAME}` - Computer name
/// - `${ANVIL_WORKLOAD}` - Current workload name
/// - `${ANVIL_VERSION}` - Anvil version
/// - `${ENV:VARNAME}` - Any environment variable
pub fn expand_variables(input: &str, workload_name: Option<&str>) -> String {
    expand_variables_with_context(input, workload_name, None)
}

/// Expand variables with additional custom variables
pub fn expand_variables_with_context(
    input: &str,
    workload_name: Option<&str>,
    custom_vars: Option<&HashMap<String, String>>,
) -> String {
    let mut result = input.to_string();

    // Expand ~ at the beginning of the path
    if result.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            result = result.replacen('~', home.to_string_lossy().as_ref(), 1);
        }
    }

    // Build the variable map
    let mut vars: HashMap<&str, String> = HashMap::new();

    // User directories
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy().to_string();
        vars.insert("HOME", home_str.clone());
        vars.insert("home", home_str);
    }

    if let Some(appdata) = dirs::data_dir() {
        let appdata_str = appdata.to_string_lossy().to_string();
        vars.insert("APPDATA", appdata_str.clone());
        vars.insert("appdata", appdata_str);
    }

    if let Some(local_appdata) = dirs::data_local_dir() {
        let local_str = local_appdata.to_string_lossy().to_string();
        vars.insert("LOCALAPPDATA", local_str.clone());
        vars.insert("localappdata", local_str);
    }

    if let Some(config) = dirs::config_dir() {
        let config_str = config.to_string_lossy().to_string();
        vars.insert("CONFIG", config_str.clone());
        vars.insert("config", config_str);
    }

    if let Some(documents) = dirs::document_dir() {
        let doc_str = documents.to_string_lossy().to_string();
        vars.insert("DOCUMENTS", doc_str.clone());
        vars.insert("documents", doc_str);
    }

    if let Some(desktop) = dirs::desktop_dir() {
        let desktop_str = desktop.to_string_lossy().to_string();
        vars.insert("DESKTOP", desktop_str.clone());
        vars.insert("desktop", desktop_str);
    }

    // System environment variables
    if let Ok(temp) = std::env::var("TEMP") {
        vars.insert("TEMP", temp.clone());
        vars.insert("temp", temp);
    } else if let Ok(tmp) = std::env::var("TMP") {
        vars.insert("TEMP", tmp.clone());
        vars.insert("temp", tmp);
    }

    if let Ok(username) = std::env::var("USERNAME") {
        vars.insert("USERNAME", username.clone());
        vars.insert("username", username);
    }

    if let Ok(computername) = std::env::var("COMPUTERNAME") {
        vars.insert("COMPUTERNAME", computername.clone());
        vars.insert("computername", computername);
    }

    // Program files
    if let Ok(pf) = std::env::var("ProgramFiles") {
        vars.insert("PROGRAMFILES", pf.clone());
        vars.insert("programfiles", pf);
    }

    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        vars.insert("PROGRAMFILES_X86", pf86.clone());
        vars.insert("programfiles_x86", pf86);
    }

    // Anvil-specific variables
    if let Some(name) = workload_name {
        vars.insert("ANVIL_WORKLOAD", name.to_string());
        vars.insert("anvil_workload", name.to_string());
    }

    vars.insert("ANVIL_VERSION", env!("CARGO_PKG_VERSION").to_string());
    vars.insert("anvil_version", env!("CARGO_PKG_VERSION").to_string());

    // Expand all known variables
    for (var_name, value) in &vars {
        let pattern = format!("${{{}}}", var_name);
        result = result.replace(&pattern, value);
    }

    // Expand custom variables if provided
    if let Some(custom) = custom_vars {
        for (var_name, value) in custom {
            let pattern = format!("${{{}}}", var_name);
            result = result.replace(&pattern, value);
        }
    }

    // Expand ${ENV:VARNAME} syntax for any environment variable
    let env_re = regex_lite::Regex::new(r"\$\{ENV:([^}]+)\}").unwrap();
    let mut env_result = result.clone();
    for cap in env_re.captures_iter(&result) {
        if let Some(var_name) = cap.get(1) {
            if let Ok(value) = std::env::var(var_name.as_str()) {
                env_result = env_result.replace(&cap[0], &value);
            }
        }
    }
    result = env_result;

    // Expand any remaining ${VAR} that might be environment variables
    let re = regex_lite::Regex::new(r"\$\{([^}:]+)\}").unwrap();
    let mut final_result = result.clone();
    for cap in re.captures_iter(&result) {
        if let Some(var_name) = cap.get(1) {
            let var_str = var_name.as_str();
            // Only expand if we haven't already expanded this variable
            if !vars.contains_key(var_str) {
                if let Ok(value) = std::env::var(var_str) {
                    final_result = final_result.replace(&cap[0], &value);
                }
            }
        }
    }

    final_result
}

/// Get all available variables and their current values
#[cfg(test)]
pub fn get_available_variables(workload_name: Option<&str>) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // User directories
    if let Some(home) = dirs::home_dir() {
        vars.insert("HOME".to_string(), home.to_string_lossy().to_string());
    }

    if let Some(appdata) = dirs::data_dir() {
        vars.insert("APPDATA".to_string(), appdata.to_string_lossy().to_string());
    }

    if let Some(local_appdata) = dirs::data_local_dir() {
        vars.insert(
            "LOCALAPPDATA".to_string(),
            local_appdata.to_string_lossy().to_string(),
        );
    }

    if let Some(config) = dirs::config_dir() {
        vars.insert("CONFIG".to_string(), config.to_string_lossy().to_string());
    }

    if let Some(documents) = dirs::document_dir() {
        vars.insert(
            "DOCUMENTS".to_string(),
            documents.to_string_lossy().to_string(),
        );
    }

    if let Some(desktop) = dirs::desktop_dir() {
        vars.insert("DESKTOP".to_string(), desktop.to_string_lossy().to_string());
    }

    // System environment variables
    if let Ok(temp) = std::env::var("TEMP").or_else(|_| std::env::var("TMP")) {
        vars.insert("TEMP".to_string(), temp);
    }

    if let Ok(username) = std::env::var("USERNAME") {
        vars.insert("USERNAME".to_string(), username);
    }

    if let Ok(computername) = std::env::var("COMPUTERNAME") {
        vars.insert("COMPUTERNAME".to_string(), computername);
    }

    // Program files
    if let Ok(pf) = std::env::var("ProgramFiles") {
        vars.insert("PROGRAMFILES".to_string(), pf);
    }

    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        vars.insert("PROGRAMFILES_X86".to_string(), pf86);
    }

    // Anvil-specific variables
    if let Some(name) = workload_name {
        vars.insert("ANVIL_WORKLOAD".to_string(), name.to_string());
    }

    vars.insert(
        "ANVIL_VERSION".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap();
        let expanded = expand_variables("~/.cargo/config.toml", None);
        assert!(expanded.starts_with(home.to_string_lossy().as_ref()));
        assert!(expanded.ends_with(".cargo/config.toml"));
    }

    #[test]
    fn test_expand_workload_name() {
        let expanded = expand_variables("${ANVIL_WORKLOAD}", Some("rust-developer"));
        assert_eq!(expanded, "rust-developer");
    }

    #[test]
    fn test_expand_version() {
        let expanded = expand_variables("${ANVIL_VERSION}", None);
        assert_eq!(expanded, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_expand_home() {
        let home = dirs::home_dir().unwrap();
        let expanded = expand_variables("${HOME}/.config", None);
        assert!(expanded.starts_with(home.to_string_lossy().as_ref()));
    }

    #[test]
    fn test_expand_env_syntax() {
        std::env::set_var("ANVIL_TEST_VAR", "test_value");
        let expanded = expand_variables("${ENV:ANVIL_TEST_VAR}", None);
        assert_eq!(expanded, "test_value");
        std::env::remove_var("ANVIL_TEST_VAR");
    }

    #[test]
    fn test_expand_nested_variables() {
        let home = dirs::home_dir().unwrap();
        let expanded = expand_variables("${HOME}/.config/${ANVIL_WORKLOAD}", Some("my-workload"));
        assert!(expanded.starts_with(home.to_string_lossy().as_ref()));
        assert!(expanded.ends_with("/.config/my-workload"));
    }

    #[test]
    fn test_expand_with_custom_vars() {
        let mut custom = HashMap::new();
        custom.insert("CUSTOM_VAR".to_string(), "custom_value".to_string());

        let expanded = expand_variables_with_context("${CUSTOM_VAR}/path", None, Some(&custom));
        assert_eq!(expanded, "custom_value/path");
    }

    #[test]
    fn test_get_available_variables() {
        let vars = get_available_variables(Some("test-workload"));
        assert!(vars.contains_key("HOME"));
        assert!(vars.contains_key("ANVIL_VERSION"));
        assert_eq!(
            vars.get("ANVIL_WORKLOAD"),
            Some(&"test-workload".to_string())
        );
    }

    #[test]
    fn test_config_manager_default_paths() {
        let manager = ConfigManager::new();
        assert!(!manager.search_paths.is_empty());
        // Default paths should always be present
        let defaults = default_workload_paths();
        for dp in &defaults {
            assert!(
                manager.search_paths.contains(dp),
                "Default path {:?} should be in search_paths",
                dp
            );
        }
    }

    #[test]
    fn test_config_manager_search_paths_accessor() {
        let manager = ConfigManager::new();
        let paths = manager.search_paths();
        assert!(!paths.is_empty());
        assert_eq!(paths.len(), manager.search_paths.len());
    }

    #[test]
    fn test_config_manager_with_paths() {
        let custom = vec![PathBuf::from("/custom/a"), PathBuf::from("/custom/b")];
        let manager = ConfigManager::with_paths(custom.clone());
        assert_eq!(manager.search_paths(), custom.as_slice());
    }

    #[test]
    fn test_config_manager_add_search_path_dedup() {
        let mut manager = ConfigManager::with_paths(vec![PathBuf::from("/first")]);
        manager.add_search_path(PathBuf::from("/second"));
        manager.add_search_path(PathBuf::from("/first")); // duplicate
        assert_eq!(manager.search_paths().len(), 2);
    }

    #[test]
    fn test_config_manager_global_config_integration() {
        // When GlobalConfig can be loaded (or defaults), ConfigManager
        // should still contain all default workload paths.
        let manager = ConfigManager::new();
        let defaults = default_workload_paths();
        for dp in &defaults {
            assert!(
                manager.search_paths().contains(dp),
                "Default path {:?} must be present",
                dp
            );
        }
    }

    #[test]
    fn test_precedence_first_path_wins() {
        let dir1 = tempfile::TempDir::new().unwrap();
        let dir2 = tempfile::TempDir::new().unwrap();

        // Create same-named workload in both directories
        let wl1 = dir1.path().join("my-wl");
        std::fs::create_dir_all(&wl1).unwrap();
        std::fs::write(
            wl1.join("workload.yaml"),
            "name: my-wl\nversion: \"1.0.0\"\ndescription: \"from dir1\"\n",
        )
        .unwrap();

        let wl2 = dir2.path().join("my-wl");
        std::fs::create_dir_all(&wl2).unwrap();
        std::fs::write(
            wl2.join("workload.yaml"),
            "name: my-wl\nversion: \"2.0.0\"\ndescription: \"from dir2\"\n",
        )
        .unwrap();

        // dir1 has higher priority (listed first)
        let manager =
            ConfigManager::with_paths(vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()]);
        let workloads = manager.list_workloads().unwrap();

        assert_eq!(workloads.len(), 1);
        assert_eq!(workloads[0].name, "my-wl");
        assert_eq!(workloads[0].description, "from dir1");
        assert_eq!(workloads[0].version, "1.0.0");
    }

    #[test]
    fn test_shadowed_paths_populated() {
        let dir1 = tempfile::TempDir::new().unwrap();
        let dir2 = tempfile::TempDir::new().unwrap();

        // Create same-named workload in both directories
        let wl1 = dir1.path().join("dup-wl");
        std::fs::create_dir_all(&wl1).unwrap();
        std::fs::write(
            wl1.join("workload.yaml"),
            "name: dup-wl\nversion: \"1.0.0\"\ndescription: \"primary\"\n",
        )
        .unwrap();

        let wl2 = dir2.path().join("dup-wl");
        std::fs::create_dir_all(&wl2).unwrap();
        std::fs::write(
            wl2.join("workload.yaml"),
            "name: dup-wl\nversion: \"2.0.0\"\ndescription: \"shadow\"\n",
        )
        .unwrap();

        let manager =
            ConfigManager::with_paths(vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()]);
        let workloads = manager.list_workloads().unwrap();

        assert_eq!(workloads.len(), 1);
        assert_eq!(workloads[0].shadowed_paths.len(), 1);
        assert_eq!(workloads[0].shadowed_paths[0], wl2.join("workload.yaml"));
    }

    #[test]
    fn test_list_all_workloads_includes_duplicates() {
        let dir1 = tempfile::TempDir::new().unwrap();
        let dir2 = tempfile::TempDir::new().unwrap();

        // Create same-named workload in both directories
        let wl1 = dir1.path().join("all-wl");
        std::fs::create_dir_all(&wl1).unwrap();
        std::fs::write(
            wl1.join("workload.yaml"),
            "name: all-wl\nversion: \"1.0.0\"\ndescription: \"primary\"\n",
        )
        .unwrap();

        let wl2 = dir2.path().join("all-wl");
        std::fs::create_dir_all(&wl2).unwrap();
        std::fs::write(
            wl2.join("workload.yaml"),
            "name: all-wl\nversion: \"2.0.0\"\ndescription: \"shadow\"\n",
        )
        .unwrap();

        // Also add a unique workload in dir2
        let wl3 = dir2.path().join("unique-wl");
        std::fs::create_dir_all(&wl3).unwrap();
        std::fs::write(
            wl3.join("workload.yaml"),
            "name: unique-wl\nversion: \"1.0.0\"\ndescription: \"unique\"\n",
        )
        .unwrap();

        let manager =
            ConfigManager::with_paths(vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()]);
        let all = manager.list_all_workloads().unwrap();

        // Should have 3 entries: primary all-wl, shadowed all-wl, unique-wl
        assert_eq!(all.len(), 3);

        // Primary entry has empty shadowed_paths
        let primary = all
            .iter()
            .find(|w| w.name == "all-wl" && w.shadowed_paths.is_empty());
        assert!(primary.is_some());

        // Shadowed entry has non-empty shadowed_paths
        let shadowed = all
            .iter()
            .find(|w| w.name == "all-wl" && !w.shadowed_paths.is_empty());
        assert!(shadowed.is_some());

        // unique-wl present
        assert!(all.iter().any(|w| w.name == "unique-wl"));
    }

    #[test]
    fn test_no_shadowing_for_unique_workloads() {
        let dir1 = tempfile::TempDir::new().unwrap();
        let dir2 = tempfile::TempDir::new().unwrap();

        let wl1 = dir1.path().join("alpha");
        std::fs::create_dir_all(&wl1).unwrap();
        std::fs::write(
            wl1.join("workload.yaml"),
            "name: alpha\nversion: \"1.0.0\"\ndescription: \"a\"\n",
        )
        .unwrap();

        let wl2 = dir2.path().join("beta");
        std::fs::create_dir_all(&wl2).unwrap();
        std::fs::write(
            wl2.join("workload.yaml"),
            "name: beta\nversion: \"1.0.0\"\ndescription: \"b\"\n",
        )
        .unwrap();

        let manager =
            ConfigManager::with_paths(vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()]);
        let workloads = manager.list_workloads().unwrap();

        assert_eq!(workloads.len(), 2);
        for w in &workloads {
            assert!(w.shadowed_paths.is_empty());
        }
    }
}
