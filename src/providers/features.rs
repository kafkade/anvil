//! Windows feature toggle provider
//!
//! Manages Windows feature toggles via registry modifications.
//! Features are defined declaratively in workload YAML and applied
//! through PowerShell registry commands.
use std::process::Command;

use thiserror::Error;

use crate::config::workload::{FeatureEntry, RegistryConfig};

#[derive(Error, Debug)]
pub enum FeatureError {
    #[error("Registry operation failed: {0}")]
    RegistryFailed(String),
    #[error("Feature requires Windows build {required}+ (current: {current})")]
    #[allow(dead_code)] // Variant covers a real error condition
    UnsupportedBuild { required: u32, current: u32 },
    #[error("Elevation required for feature: {0}")]
    #[allow(dead_code)] // Variant covers a real error condition
    ElevationRequired(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of applying a feature toggle
pub struct FeatureResult {
    #[allow(dead_code)] // Used by callers for logging
    pub feature_name: String,
    pub already_configured: bool,
    pub skipped_build: bool,
    pub values_set: usize,
}

pub struct FeatureProvider;

impl FeatureProvider {
    /// Apply a single feature toggle
    pub fn apply_feature(
        entry: &FeatureEntry,
        dry_run: bool,
    ) -> Result<FeatureResult, FeatureError> {
        // Check minimum build requirement
        if let Some(min_build) = entry.min_build {
            let current_build = Self::get_os_build();
            if current_build < min_build {
                return Ok(FeatureResult {
                    feature_name: entry.name.clone(),
                    already_configured: false,
                    skipped_build: true,
                    values_set: 0,
                });
            }
        }

        // Check if already configured
        if Self::check_registry_values(&entry.registry)? {
            return Ok(FeatureResult {
                feature_name: entry.name.clone(),
                already_configured: true,
                skipped_build: false,
                values_set: 0,
            });
        }

        if dry_run {
            return Ok(FeatureResult {
                feature_name: entry.name.clone(),
                already_configured: false,
                skipped_build: false,
                values_set: entry.registry.values.len(),
            });
        }

        // Set registry values
        Self::set_registry_values(&entry.registry)?;

        // Verify
        let verified = Self::verify_registry_values(&entry.registry)?;
        if !verified {
            return Err(FeatureError::RegistryFailed(format!(
                "Verification failed after setting values for '{}'",
                entry.name
            )));
        }

        Ok(FeatureResult {
            feature_name: entry.name.clone(),
            already_configured: false,
            skipped_build: false,
            values_set: entry.registry.values.len(),
        })
    }

    /// Get the current Windows OS build number
    pub fn get_os_build() -> u32 {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[System.Environment]::OSVersion.Version.Build",
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u32>()
                .unwrap_or(0),
            _ => 0,
        }
    }

    /// Check if all registry values already match the desired state
    pub fn check_registry_values(registry: &RegistryConfig) -> Result<bool, FeatureError> {
        for value_entry in &registry.values {
            let ps_path = format!("{}:\\{}", registry.hive, registry.path);

            let script = format!(
                "try {{ (Get-ItemProperty -Path '{}' -Name '{}' -ErrorAction Stop).'{}' }} catch {{ 'ANVIL_NOT_FOUND' }}",
                ps_path, value_entry.name, value_entry.name
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| FeatureError::RegistryFailed(e.to_string()))?;

            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if stdout == "ANVIL_NOT_FOUND" {
                return Ok(false);
            }

            let expected = match &value_entry.value {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };

            if stdout != expected {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Set registry values via PowerShell
    pub fn set_registry_values(registry: &RegistryConfig) -> Result<(), FeatureError> {
        let ps_path = format!("{}:\\{}", registry.hive, registry.path);

        // Ensure the key exists
        let ensure_script = format!(
            "if (-not (Test-Path '{}')) {{ New-Item -Path '{}' -Force | Out-Null }}",
            ps_path, ps_path
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ensure_script])
            .output()
            .map_err(|e| FeatureError::RegistryFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FeatureError::RegistryFailed(format!(
                "Failed to ensure registry key: {}",
                stderr
            )));
        }

        // Set each value
        for value_entry in &registry.values {
            let reg_type = match value_entry.value_type.as_str() {
                "dword" => "DWord",
                "string" => "String",
                "expand_string" => "ExpandString",
                other => other,
            };

            let value_str = match &value_entry.value {
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };

            let script = format!(
                "Set-ItemProperty -Path '{}' -Name '{}' -Value {} -Type {} -Force",
                ps_path, value_entry.name, value_str, reg_type
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| FeatureError::RegistryFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(FeatureError::RegistryFailed(format!(
                    "Failed to set '{}': {}",
                    value_entry.name, stderr
                )));
            }
        }

        Ok(())
    }

    /// Verify that registry values were set correctly
    pub fn verify_registry_values(registry: &RegistryConfig) -> Result<bool, FeatureError> {
        Self::check_registry_values(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os_build() {
        let build = FeatureProvider::get_os_build();
        // On Windows, should return a reasonable build number (> 10000)
        // On other platforms or CI, may return 0
        if cfg!(windows) {
            assert!(build > 0, "OS build should be > 0 on Windows");
        }
    }

    #[test]
    fn test_check_registry_nonexistent() {
        let config = RegistryConfig {
            path: "SOFTWARE\\AnvilTestNonexistent_12345_xyz".to_string(),
            hive: "HKCU".to_string(),
            values: vec![crate::config::workload::RegistryValueEntry {
                name: "TestValue".to_string(),
                value_type: "dword".to_string(),
                value: serde_json::json!(1),
            }],
        };

        let result = FeatureProvider::check_registry_values(&config);
        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "Nonexistent registry path should return false"
        );
    }
}
