//! Installation state tracking module
//!
//! This module tracks the state of package installations for recovery,
//! status reporting, and avoiding redundant operations.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::get_state_dir;

/// Status of a package installation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageStatus {
    /// Package installation is pending
    Pending,
    /// Package is currently being installed
    Installing,
    /// Package was successfully installed
    Installed,
    /// Package installation failed
    Failed,
    /// Package was skipped (already installed)
    Skipped,
    /// Package was upgraded
    Upgraded,
}

impl std::fmt::Display for PackageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageStatus::Pending => write!(f, "pending"),
            PackageStatus::Installing => write!(f, "installing"),
            PackageStatus::Installed => write!(f, "installed"),
            PackageStatus::Failed => write!(f, "failed"),
            PackageStatus::Skipped => write!(f, "skipped"),
            PackageStatus::Upgraded => write!(f, "upgraded"),
        }
    }
}

/// Record of a single package installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRecord {
    /// Package ID
    pub id: String,
    /// Requested version (if any)
    pub requested_version: Option<String>,
    /// Actually installed version (if known)
    pub installed_version: Option<String>,
    /// Installation status
    pub status: PackageStatus,
    /// Timestamp when status was last updated
    pub timestamp: DateTime<Utc>,
    /// Duration of installation in seconds
    pub duration_secs: Option<f64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Whether a reboot is required
    pub reboot_required: bool,
}

impl PackageRecord {
    /// Create a new pending package record
    pub fn pending(id: impl Into<String>, version: Option<String>) -> Self {
        Self {
            id: id.into(),
            requested_version: version,
            installed_version: None,
            status: PackageStatus::Pending,
            timestamp: Utc::now(),
            duration_secs: None,
            error: None,
            reboot_required: false,
        }
    }

    /// Mark the package as installing
    pub fn mark_installing(&mut self) {
        self.status = PackageStatus::Installing;
        self.timestamp = Utc::now();
    }

    /// Mark the package as successfully installed
    pub fn mark_installed(
        &mut self,
        version: Option<String>,
        duration: f64,
        reboot_required: bool,
    ) {
        self.status = PackageStatus::Installed;
        self.installed_version = version;
        self.duration_secs = Some(duration);
        self.reboot_required = reboot_required;
        self.timestamp = Utc::now();
    }

    /// Mark the package as upgraded
    pub fn mark_upgraded(&mut self, version: Option<String>, duration: f64, reboot_required: bool) {
        self.status = PackageStatus::Upgraded;
        self.installed_version = version;
        self.duration_secs = Some(duration);
        self.reboot_required = reboot_required;
        self.timestamp = Utc::now();
    }

    /// Mark the package as failed
    pub fn mark_failed(&mut self, error: impl Into<String>, duration: f64) {
        self.status = PackageStatus::Failed;
        self.error = Some(error.into());
        self.duration_secs = Some(duration);
        self.timestamp = Utc::now();
    }

    /// Mark the package as skipped
    pub fn mark_skipped(&mut self, reason: impl Into<String>) {
        self.status = PackageStatus::Skipped;
        self.error = Some(reason.into());
        self.timestamp = Utc::now();
    }
}

/// Summary of installation state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallationSummary {
    /// Total packages in the workload
    pub total: usize,
    /// Packages pending installation
    pub pending: usize,
    /// Packages currently installing
    pub installing: usize,
    /// Successfully installed packages
    pub installed: usize,
    /// Failed packages
    pub failed: usize,
    /// Skipped packages
    pub skipped: usize,
    /// Upgraded packages
    pub upgraded: usize,
    /// Total duration in seconds
    pub total_duration_secs: f64,
    /// Whether any package requires a reboot
    pub reboot_required: bool,
}

impl InstallationSummary {
    /// Calculate summary from package records
    pub fn from_records(records: &HashMap<String, PackageRecord>) -> Self {
        let mut summary = Self {
            total: records.len(),
            ..Default::default()
        };

        for record in records.values() {
            match record.status {
                PackageStatus::Pending => summary.pending += 1,
                PackageStatus::Installing => summary.installing += 1,
                PackageStatus::Installed => summary.installed += 1,
                PackageStatus::Failed => summary.failed += 1,
                PackageStatus::Skipped => summary.skipped += 1,
                PackageStatus::Upgraded => summary.upgraded += 1,
            }

            if let Some(duration) = record.duration_secs {
                summary.total_duration_secs += duration;
            }

            if record.reboot_required {
                summary.reboot_required = true;
            }
        }

        summary
    }

    /// Check if installation is complete (no pending or installing)
    pub fn is_complete(&self) -> bool {
        self.pending == 0 && self.installing == 0
    }

    /// Check if all packages succeeded
    pub fn is_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Installation state for a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationState {
    /// Workload name
    pub workload_name: String,
    /// Workload version
    pub workload_version: String,
    /// When the installation was started
    pub started_at: DateTime<Utc>,
    /// When the installation was last updated
    pub updated_at: DateTime<Utc>,
    /// Whether the installation is complete
    pub completed: bool,
    /// Package records indexed by package ID
    pub packages: HashMap<String, PackageRecord>,
    /// Session ID for this installation run
    pub session_id: String,
}

impl InstallationState {
    /// Create a new installation state for a workload
    pub fn new(workload_name: impl Into<String>, workload_version: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            workload_name: workload_name.into(),
            workload_version: workload_version.into(),
            started_at: now,
            updated_at: now,
            completed: false,
            packages: HashMap::new(),
            session_id: generate_session_id(),
        }
    }

    /// Load installation state from disk for a workload
    pub fn load(workload_name: &str) -> Result<Option<Self>> {
        let state_file = Self::state_file_path(workload_name)?;

        if !state_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&state_file)
            .with_context(|| format!("Failed to read state file: {}", state_file.display()))?;

        let state: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse state file: {}", state_file.display()))?;

        Ok(Some(state))
    }

    /// Save installation state to disk
    pub fn save(&self) -> Result<()> {
        let state_file = Self::state_file_path(&self.workload_name)?;

        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize installation state")?;

        std::fs::write(&state_file, content)
            .with_context(|| format!("Failed to write state file: {}", state_file.display()))?;

        Ok(())
    }

    /// Delete the state file for a workload
    pub fn delete(workload_name: &str) -> Result<()> {
        let state_file = Self::state_file_path(workload_name)?;

        if state_file.exists() {
            std::fs::remove_file(&state_file).with_context(|| {
                format!("Failed to delete state file: {}", state_file.display())
            })?;
        }

        Ok(())
    }

    /// Get the path to the state file for a workload
    fn state_file_path(workload_name: &str) -> Result<PathBuf> {
        let state_dir = get_state_dir()?;
        // Sanitize workload name for use in filename
        let safe_name = workload_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        Ok(state_dir.join(format!("{}.json", safe_name)))
    }

    /// Add a package to track
    pub fn add_package(&mut self, id: impl Into<String>, version: Option<String>) {
        let id = id.into();
        self.packages
            .insert(id.clone(), PackageRecord::pending(id, version));
        self.updated_at = Utc::now();
    }

    /// Get a mutable reference to a package record
    pub fn get_package_mut(&mut self, id: &str) -> Option<&mut PackageRecord> {
        self.updated_at = Utc::now();
        self.packages.get_mut(id)
    }

    /// Get a package record
    #[allow(dead_code)]
    pub fn get_package(&self, id: &str) -> Option<&PackageRecord> {
        self.packages.get(id)
    }

    /// Mark the installation as complete
    pub fn mark_complete(&mut self) {
        self.completed = true;
        self.updated_at = Utc::now();
    }

    /// Get installation summary
    pub fn summary(&self) -> InstallationSummary {
        InstallationSummary::from_records(&self.packages)
    }

    /// Get list of failed packages
    pub fn failed_packages(&self) -> Vec<&PackageRecord> {
        self.packages
            .values()
            .filter(|p| p.status == PackageStatus::Failed)
            .collect()
    }

    /// Get list of packages requiring reboot
    pub fn packages_requiring_reboot(&self) -> Vec<&PackageRecord> {
        self.packages
            .values()
            .filter(|p| p.reboot_required)
            .collect()
    }

    /// Check if there are failed packages from a previous run
    #[allow(dead_code)]
    pub fn has_failed_packages(&self) -> bool {
        self.packages
            .values()
            .any(|p| p.status == PackageStatus::Failed)
    }

    /// Reset failed packages to pending (for retry)
    #[allow(dead_code)]
    pub fn reset_failed_packages(&mut self) {
        for record in self.packages.values_mut() {
            if record.status == PackageStatus::Failed {
                record.status = PackageStatus::Pending;
                record.error = None;
                record.duration_secs = None;
                record.timestamp = Utc::now();
            }
        }
        self.updated_at = Utc::now();
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    format!("{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_record_lifecycle() {
        let mut record = PackageRecord::pending("Git.Git", Some("2.43.0".to_string()));
        assert_eq!(record.status, PackageStatus::Pending);

        record.mark_installing();
        assert_eq!(record.status, PackageStatus::Installing);

        record.mark_installed(Some("2.43.0".to_string()), 45.5, false);
        assert_eq!(record.status, PackageStatus::Installed);
        assert_eq!(record.duration_secs, Some(45.5));
    }

    #[test]
    fn test_package_record_failure() {
        let mut record = PackageRecord::pending("Bad.Package", None);
        record.mark_installing();
        record.mark_failed("Network error", 10.0);

        assert_eq!(record.status, PackageStatus::Failed);
        assert_eq!(record.error, Some("Network error".to_string()));
    }

    #[test]
    fn test_installation_summary() {
        let mut records = HashMap::new();
        records.insert("a".to_string(), {
            let mut r = PackageRecord::pending("a", None);
            r.mark_installed(None, 10.0, false);
            r
        });
        records.insert("b".to_string(), {
            let mut r = PackageRecord::pending("b", None);
            r.mark_failed("error", 5.0);
            r
        });
        records.insert("c".to_string(), {
            let mut r = PackageRecord::pending("c", None);
            r.mark_skipped("already installed");
            r
        });

        let summary = InstallationSummary::from_records(&records);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.installed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
        assert!(!summary.is_successful());
    }

    #[test]
    fn test_installation_state() {
        let mut state = InstallationState::new("test-workload", "1.0.0");
        state.add_package("Git.Git", Some("2.43.0".to_string()));
        state.add_package("VSCode", None);

        assert_eq!(state.packages.len(), 2);

        if let Some(record) = state.get_package_mut("Git.Git") {
            record.mark_installing();
            record.mark_installed(Some("2.43.0".to_string()), 30.0, false);
        }

        let summary = state.summary();
        assert_eq!(summary.installed, 1);
        assert_eq!(summary.pending, 1);
    }
}
