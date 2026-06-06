//! Event types for TUI communication
//!
//! These types define the channel-based protocol between the operations
//! logic (sender) and the TUI dashboards (receiver).

use std::time::Duration;

/// Phases of the installation process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPhase {
    PreCommands,
    Packages,
    Fonts,
    Features,
    Terminal,
    Files,
    PostCommands,
}

impl InstallPhase {
    /// Human-readable label
    pub fn label(&self) -> &str {
        match self {
            InstallPhase::PreCommands => "Running pre-install commands",
            InstallPhase::Packages => "Installing packages",
            InstallPhase::Fonts => "Installing fonts",
            InstallPhase::Features => "Configuring features",
            InstallPhase::Terminal => "Configuring terminal",
            InstallPhase::Files => "Deploying files",
            InstallPhase::PostCommands => "Running post-install commands",
        }
    }
}

impl std::fmt::Display for InstallPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Status of an individual item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemResult {
    Success,
    Skipped,
    Failed,
}

/// Events sent from install logic to the TUI
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InstallEvent {
    /// A phase is starting
    PhaseStart { phase: InstallPhase, total: usize },

    /// An individual item within a phase is starting
    ItemStart { phase: InstallPhase, name: String },

    /// An individual item completed
    ItemComplete {
        phase: InstallPhase,
        name: String,
        result: ItemResult,
        message: String,
    },

    /// A phase completed
    PhaseComplete { phase: InstallPhase },

    /// A log message (for verbose mode)
    Log { message: String },

    /// Installation is done
    Done {
        success: bool,
        summary: String,
        duration: Duration,
    },
}

/// Phases of the health check process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthPhase {
    Packages,
    Files,
    Assertions,
}

impl HealthPhase {
    /// Human-readable label
    pub fn label(&self) -> &str {
        match self {
            HealthPhase::Packages => "Packages",
            HealthPhase::Files => "Files",
            HealthPhase::Assertions => "Assertions",
        }
    }
}

/// Events sent from health check logic to the TUI
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum HealthEvent {
    /// A phase is starting
    PhaseStart { phase: HealthPhase, total: usize },

    /// An individual item check completed
    ItemComplete {
        phase: HealthPhase,
        name: String,
        passed: bool,
        message: Option<String>,
    },

    /// A phase completed
    PhaseComplete { phase: HealthPhase },

    /// A log message
    Log { message: String },

    /// Health check is done
    Done { duration: Duration },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_labels() {
        assert_eq!(InstallPhase::Packages.label(), "Installing packages");
        assert_eq!(InstallPhase::Fonts.label(), "Installing fonts");
        assert_eq!(InstallPhase::Features.label(), "Configuring features");
        assert_eq!(InstallPhase::Terminal.label(), "Configuring terminal");
        assert_eq!(InstallPhase::Files.label(), "Deploying files");
        assert_eq!(
            InstallPhase::PreCommands.label(),
            "Running pre-install commands"
        );
        assert_eq!(
            InstallPhase::PostCommands.label(),
            "Running post-install commands"
        );
    }

    #[test]
    fn test_install_event_construction() {
        let event = InstallEvent::PhaseStart {
            phase: InstallPhase::Packages,
            total: 5,
        };
        match event {
            InstallEvent::PhaseStart { phase, total } => {
                assert_eq!(phase, InstallPhase::Packages);
                assert_eq!(total, 5);
            }
            _ => panic!("Wrong variant"),
        }
    }
}
