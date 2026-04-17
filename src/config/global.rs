//! Global configuration management for Anvil
//!
//! This module handles the global configuration file at `~/.anvil/config.yaml`
//! which stores user preferences and default settings.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Global configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct GlobalConfig {
    /// Default settings for commands
    pub defaults: DefaultsConfig,

    /// Backup settings
    pub backup: BackupConfig,

    /// Installation settings
    pub install: InstallConfig,

    /// Workload search paths configuration
    pub workloads: WorkloadsConfig,

    /// Logging configuration
    pub logging: LoggingConfig,
}

impl GlobalConfig {
    /// Load global configuration from file
    ///
    /// If the file doesn't exist, returns default configuration.
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;
            let config: GlobalConfig = serde_yaml::from_str(&content).with_context(|| {
                format!("Failed to parse config file: {}", config_path.display())
            })?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create parent directories if needed
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = serde_yaml::to_string(self).context("Failed to serialize configuration")?;

        std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        Ok(())
    }

    /// Get the path to the global configuration file
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot find home directory")?;
        Ok(home.join(".anvil").join("config.yaml"))
    }

    /// Get a configuration value by key path (e.g., "defaults.shell")
    pub fn get(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();

        match parts.as_slice() {
            ["defaults", "shell"] => Some(self.defaults.shell.clone()),
            ["defaults", "script_timeout"] => Some(self.defaults.script_timeout.to_string()),
            ["defaults", "output_format"] => Some(self.defaults.output_format.clone()),
            ["defaults", "color"] => Some(self.defaults.color.clone()),
            ["backup", "auto_backup"] => Some(self.backup.auto_backup.to_string()),
            ["backup", "retention_days"] => Some(self.backup.retention_days.to_string()),
            ["backup", "max_backups"] => Some(self.backup.max_backups.to_string()),
            ["backup", "compress"] => Some(self.backup.compress.to_string()),
            ["install", "parallel_packages"] => Some(self.install.parallel_packages.to_string()),
            ["install", "skip_installed"] => Some(self.install.skip_installed.to_string()),
            ["install", "confirm"] => Some(self.install.confirm.to_string()),
            ["logging", "level"] => Some(self.logging.level.clone()),
            ["logging", "file"] => self.logging.file.clone(),
            _ => None,
        }
    }

    /// Set a configuration value by key path (e.g., "defaults.shell")
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();

        match parts.as_slice() {
            ["defaults", "shell"] => {
                self.defaults.shell = value.to_string();
            }
            ["defaults", "script_timeout"] => {
                self.defaults.script_timeout = value
                    .parse()
                    .with_context(|| format!("Invalid timeout value: {}", value))?;
            }
            ["defaults", "output_format"] => {
                let valid = ["table", "json", "yaml", "html"];
                if !valid.contains(&value) {
                    anyhow::bail!(
                        "Invalid output format: {}. Valid values: {:?}",
                        value,
                        valid
                    );
                }
                self.defaults.output_format = value.to_string();
            }
            ["defaults", "color"] => {
                let valid = ["auto", "always", "never"];
                if !valid.contains(&value) {
                    anyhow::bail!(
                        "Invalid color setting: {}. Valid values: {:?}",
                        value,
                        valid
                    );
                }
                self.defaults.color = value.to_string();
            }
            ["backup", "auto_backup"] => {
                self.backup.auto_backup = parse_bool(value)
                    .with_context(|| format!("Invalid boolean value: {}", value))?;
            }
            ["backup", "retention_days"] => {
                self.backup.retention_days = value
                    .parse()
                    .with_context(|| format!("Invalid retention_days value: {}", value))?;
            }
            ["backup", "max_backups"] => {
                self.backup.max_backups = value
                    .parse()
                    .with_context(|| format!("Invalid max_backups value: {}", value))?;
            }
            ["backup", "compress"] => {
                self.backup.compress = parse_bool(value)
                    .with_context(|| format!("Invalid boolean value: {}", value))?;
            }
            ["install", "parallel_packages"] => {
                self.install.parallel_packages = parse_bool(value)
                    .with_context(|| format!("Invalid boolean value: {}", value))?;
            }
            ["install", "skip_installed"] => {
                self.install.skip_installed = parse_bool(value)
                    .with_context(|| format!("Invalid boolean value: {}", value))?;
            }
            ["install", "confirm"] => {
                self.install.confirm = parse_bool(value)
                    .with_context(|| format!("Invalid boolean value: {}", value))?;
            }
            ["logging", "level"] => {
                let valid = ["error", "warn", "info", "debug", "trace"];
                if !valid.contains(&value) {
                    anyhow::bail!("Invalid log level: {}. Valid values: {:?}", value, valid);
                }
                self.logging.level = value.to_string();
            }
            ["logging", "file"] => {
                if value.is_empty() || value == "null" || value == "none" {
                    self.logging.file = None;
                } else {
                    self.logging.file = Some(value.to_string());
                }
            }
            _ => {
                anyhow::bail!("Unknown configuration key: {}", key);
            }
        }

        Ok(())
    }

    /// List all configuration keys and values
    pub fn list(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();

        items.push(("defaults.shell".to_string(), self.defaults.shell.clone()));
        items.push((
            "defaults.script_timeout".to_string(),
            self.defaults.script_timeout.to_string(),
        ));
        items.push((
            "defaults.output_format".to_string(),
            self.defaults.output_format.clone(),
        ));
        items.push(("defaults.color".to_string(), self.defaults.color.clone()));

        items.push((
            "backup.auto_backup".to_string(),
            self.backup.auto_backup.to_string(),
        ));
        items.push((
            "backup.retention_days".to_string(),
            self.backup.retention_days.to_string(),
        ));
        items.push((
            "backup.max_backups".to_string(),
            self.backup.max_backups.to_string(),
        ));
        items.push((
            "backup.compress".to_string(),
            self.backup.compress.to_string(),
        ));

        items.push((
            "install.parallel_packages".to_string(),
            self.install.parallel_packages.to_string(),
        ));
        items.push((
            "install.skip_installed".to_string(),
            self.install.skip_installed.to_string(),
        ));
        items.push((
            "install.confirm".to_string(),
            self.install.confirm.to_string(),
        ));

        items.push((
            "workloads.paths".to_string(),
            if self.workloads.paths.is_empty() {
                "(default)".to_string()
            } else {
                self.workloads.paths.join(", ")
            },
        ));

        items.push(("logging.level".to_string(), self.logging.level.clone()));
        items.push((
            "logging.file".to_string(),
            self.logging
                .file
                .clone()
                .unwrap_or_else(|| "(none)".to_string()),
        ));

        items
    }

    /// Reset configuration to defaults
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Add a workload search path
    #[allow(dead_code)]
    pub fn add_workload_path(&mut self, path: impl Into<String>) {
        let path_str = path.into();
        if !self.workloads.paths.contains(&path_str) {
            self.workloads.paths.push(path_str);
        }
    }

    /// Remove a workload search path
    #[allow(dead_code)]
    pub fn remove_workload_path(&mut self, path: &str) -> bool {
        if let Some(pos) = self.workloads.paths.iter().position(|p| p == path) {
            self.workloads.paths.remove(pos);
            true
        } else {
            false
        }
    }
}

/// Default settings for commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultsConfig {
    /// Default shell for scripts (powershell, pwsh)
    pub shell: String,

    /// Default script timeout in seconds
    pub script_timeout: u64,

    /// Default output format (table, json, yaml)
    pub output_format: String,

    /// Color output (auto, always, never)
    pub color: String,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            shell: "powershell".to_string(),
            script_timeout: 300,
            output_format: "table".to_string(),
            color: "auto".to_string(),
        }
    }
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupConfig {
    /// Create backup before install
    pub auto_backup: bool,

    /// Delete backups older than this many days
    pub retention_days: u32,

    /// Maximum backups to keep per workload
    pub max_backups: u32,

    /// Compress backups by default
    pub compress: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            auto_backup: true,
            retention_days: 30,
            max_backups: 10,
            compress: false,
        }
    }
}

/// Installation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallConfig {
    /// Install packages in parallel
    pub parallel_packages: bool,

    /// Skip already installed packages
    pub skip_installed: bool,

    /// Prompt for confirmation
    pub confirm: bool,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            parallel_packages: false,
            skip_installed: true,
            confirm: true,
        }
    }
}

/// Workload search paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct WorkloadsConfig {
    /// Additional workload search paths
    pub paths: Vec<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: String,

    /// Log file path (null for no file logging)
    pub file: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
        }
    }
}

/// Parse a boolean value from string
fn parse_bool(s: &str) -> Result<bool> {
    match s.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        _ => anyhow::bail!("Cannot parse '{}' as boolean", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert_eq!(config.defaults.shell, "powershell");
        assert_eq!(config.defaults.script_timeout, 300);
        assert!(config.backup.auto_backup);
        assert_eq!(config.backup.retention_days, 30);
    }

    #[test]
    fn test_get_config_value() {
        let config = GlobalConfig::default();
        assert_eq!(config.get("defaults.shell"), Some("powershell".to_string()));
        assert_eq!(
            config.get("defaults.script_timeout"),
            Some("300".to_string())
        );
        assert_eq!(config.get("backup.auto_backup"), Some("true".to_string()));
        assert_eq!(config.get("unknown.key"), None);
    }

    #[test]
    fn test_set_config_value() {
        let mut config = GlobalConfig::default();

        config.set("defaults.shell", "pwsh").unwrap();
        assert_eq!(config.defaults.shell, "pwsh");

        config.set("defaults.script_timeout", "600").unwrap();
        assert_eq!(config.defaults.script_timeout, 600);

        config.set("backup.auto_backup", "false").unwrap();
        assert!(!config.backup.auto_backup);
    }

    #[test]
    fn test_set_invalid_value() {
        let mut config = GlobalConfig::default();

        let result = config.set("defaults.script_timeout", "not_a_number");
        assert!(result.is_err());

        let result = config.set("defaults.output_format", "invalid");
        assert!(result.is_err());

        let result = config.set("unknown.key", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_config() {
        let config = GlobalConfig::default();
        let items = config.list();

        assert!(!items.is_empty());
        assert!(items.iter().any(|(k, _)| k == "defaults.shell"));
        assert!(items.iter().any(|(k, _)| k == "backup.auto_backup"));
    }

    #[test]
    fn test_reset_config() {
        let mut config = GlobalConfig::default();
        config.defaults.shell = "custom".to_string();
        config.defaults.script_timeout = 999;

        config.reset();

        assert_eq!(config.defaults.shell, "powershell");
        assert_eq!(config.defaults.script_timeout, 300);
    }

    #[test]
    fn test_workload_paths() {
        let mut config = GlobalConfig::default();

        config.add_workload_path("/custom/path");
        assert!(config.workloads.paths.contains(&"/custom/path".to_string()));

        // Adding same path again should not duplicate
        config.add_workload_path("/custom/path");
        assert_eq!(
            config
                .workloads
                .paths
                .iter()
                .filter(|p| *p == "/custom/path")
                .count(),
            1
        );

        // Remove path
        assert!(config.remove_workload_path("/custom/path"));
        assert!(!config.workloads.paths.contains(&"/custom/path".to_string()));

        // Removing non-existent path returns false
        assert!(!config.remove_workload_path("/nonexistent"));
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("on").unwrap());

        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("off").unwrap());

        assert!(parse_bool("invalid").is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = GlobalConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: GlobalConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.defaults.shell, parsed.defaults.shell);
        assert_eq!(config.backup.retention_days, parsed.backup.retention_days);
    }
}
