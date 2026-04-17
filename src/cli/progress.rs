//! Progress bar management module
//!
//! This module provides utilities for displaying progress during
//! long-running operations like package installation.

use std::sync::Arc;
use std::time::Duration;

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

/// Default spinner tick interval in milliseconds
const SPINNER_TICK_MS: u64 = 100;

/// Progress bar styles
#[allow(dead_code)]
pub struct ProgressStyles;

#[allow(dead_code)]
impl ProgressStyles {
    /// Style for package installation progress
    pub fn install() -> ProgressStyle {
        ProgressStyle::with_template(
            "  {spinner:.green} [{bar:30.cyan/blue}] {pos}/{len} {wide_msg}",
        )
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        .progress_chars("█▓▒░")
    }

    /// Style for download progress (with bytes)
    pub fn download() -> ProgressStyle {
        ProgressStyle::with_template(
            "  {spinner:.green} [{bar:30.cyan/blue}] {bytes}/{total_bytes} {wide_msg}",
        )
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        .progress_chars("█▓▒░")
    }

    /// Style for indeterminate spinner
    pub fn spinner() -> ProgressStyle {
        ProgressStyle::with_template("  {spinner:.green} {wide_msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
    }

    /// Style for overall progress (summary bar)
    pub fn summary() -> ProgressStyle {
        ProgressStyle::with_template(
            "\n{spinner:.green} Overall Progress: [{bar:40.cyan/blue}] {pos}/{len} packages\n{wide_msg}",
        )
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        .progress_chars("█▓▒░")
    }

    /// Style for individual package installation
    pub fn package() -> ProgressStyle {
        ProgressStyle::with_template("  {prefix:.bold} {spinner:.green} {wide_msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
    }

    /// Style for completed operations (no spinner)
    pub fn completed() -> ProgressStyle {
        ProgressStyle::with_template("  {prefix:.bold} {wide_msg}").unwrap()
    }
}

/// Manager for multi-progress bar display
#[allow(dead_code)]
pub struct ProgressManager {
    /// The multi-progress container
    multi_progress: MultiProgress,
    /// Whether progress display is enabled
    enabled: bool,
    /// Whether to use quiet mode (minimal output)
    quiet: bool,
}

#[allow(dead_code)]
impl ProgressManager {
    /// Create a new progress manager
    pub fn new() -> Self {
        Self {
            multi_progress: MultiProgress::new(),
            enabled: true,
            quiet: false,
        }
    }

    /// Create a progress manager with quiet mode
    pub fn quiet() -> Self {
        Self {
            multi_progress: MultiProgress::with_draw_target(ProgressDrawTarget::hidden()),
            enabled: false,
            quiet: true,
        }
    }

    /// Check if progress display is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if quiet mode is active
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Disable progress display (but keep the manager)
    pub fn disable(&mut self) {
        self.enabled = false;
        self.multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::hidden());
    }

    /// Enable progress display
    pub fn enable(&mut self) {
        self.enabled = true;
        self.multi_progress = MultiProgress::new();
    }

    /// Create a new progress bar for package installation
    pub fn create_install_bar(&self, total: u64, message: &str) -> ProgressBar {
        let pb = if self.enabled {
            self.multi_progress.add(ProgressBar::new(total))
        } else {
            ProgressBar::hidden()
        };

        pb.set_style(ProgressStyles::install());
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        pb
    }

    /// Create a new spinner for indeterminate operations
    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let pb = if self.enabled {
            self.multi_progress.add(ProgressBar::new_spinner())
        } else {
            ProgressBar::hidden()
        };

        pb.set_style(ProgressStyles::spinner());
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        pb
    }

    /// Create a summary/overall progress bar
    pub fn create_summary_bar(&self, total: u64) -> ProgressBar {
        let pb = if self.enabled {
            self.multi_progress.add(ProgressBar::new(total))
        } else {
            ProgressBar::hidden()
        };

        pb.set_style(ProgressStyles::summary());
        pb.set_message("");
        pb.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        pb
    }

    /// Create a progress bar for a specific package
    pub fn create_package_bar(&self, package_id: &str) -> ProgressBar {
        let pb = if self.enabled {
            self.multi_progress.add(ProgressBar::new_spinner())
        } else {
            ProgressBar::hidden()
        };

        pb.set_style(ProgressStyles::package());
        pb.set_prefix(format!("[{}]", package_id));
        pb.set_message("Pending...");
        pb.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        pb
    }

    /// Get a reference to the multi-progress container
    pub fn multi_progress(&self) -> &MultiProgress {
        &self.multi_progress
    }

    /// Suspend progress bars to allow other output
    pub fn suspend<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.multi_progress.suspend(f)
    }

    /// Clear all progress bars
    pub fn clear(&self) -> std::io::Result<()> {
        self.multi_progress.clear()
    }

    /// Print a line above the progress bars
    pub fn println(&self, msg: &str) -> std::io::Result<()> {
        self.multi_progress.println(msg)
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress tracker for a single installation operation
#[allow(dead_code)]
pub struct InstallProgress {
    /// Overall progress bar
    summary_bar: ProgressBar,
    /// Individual package progress bars
    package_bars: Vec<ProgressBar>,
    /// Manager reference
    manager: Arc<ProgressManager>,
    /// Total package count
    total: usize,
    /// Current completed count
    completed: usize,
    /// Installed count
    installed: usize,
    /// Skipped count
    skipped: usize,
    /// Failed count
    failed: usize,
}

#[allow(dead_code)]
impl InstallProgress {
    /// Create a new installation progress tracker
    pub fn new(manager: Arc<ProgressManager>, total: usize) -> Self {
        let summary_bar = manager.create_summary_bar(total as u64);
        summary_bar.set_message("Installed: 0 | Skipped: 0 | Failed: 0".to_string());

        Self {
            summary_bar,
            package_bars: Vec::new(),
            manager,
            total,
            completed: 0,
            installed: 0,
            skipped: 0,
            failed: 0,
        }
    }

    /// Start tracking a package installation
    pub fn start_package(&mut self, package_id: &str) -> usize {
        let bar = self.manager.create_package_bar(package_id);
        bar.set_message("Installing...");
        let idx = self.package_bars.len();
        self.package_bars.push(bar);
        idx
    }

    /// Mark a package as completed successfully
    pub fn complete_package(&mut self, idx: usize, message: &str) {
        if let Some(bar) = self.package_bars.get(idx) {
            bar.set_style(ProgressStyles::completed());
            bar.set_prefix(format!("{}", style("✓").green()));
            bar.set_message(message.to_string());
            bar.finish();
        }
        self.installed += 1;
        self.completed += 1;
        self.update_summary();
    }

    /// Mark a package as skipped
    pub fn skip_package(&mut self, idx: usize, reason: &str) {
        if let Some(bar) = self.package_bars.get(idx) {
            bar.set_style(ProgressStyles::completed());
            bar.set_prefix(format!("{}", style("○").dim()));
            bar.set_message(format!("Skipped: {}", reason));
            bar.finish();
        }
        self.skipped += 1;
        self.completed += 1;
        self.update_summary();
    }

    /// Mark a package as failed
    pub fn fail_package(&mut self, idx: usize, error: &str) {
        if let Some(bar) = self.package_bars.get(idx) {
            bar.set_style(ProgressStyles::completed());
            bar.set_prefix(format!("{}", style("✗").red()));
            bar.set_message(format!("Failed: {}", error));
            bar.finish();
        }
        self.failed += 1;
        self.completed += 1;
        self.update_summary();
    }

    /// Update the package message (for status updates during install)
    pub fn update_package_message(&self, idx: usize, message: &str) {
        if let Some(bar) = self.package_bars.get(idx) {
            bar.set_message(message.to_string());
        }
    }

    /// Update the summary bar
    fn update_summary(&self) {
        self.summary_bar.set_position(self.completed as u64);
        self.summary_bar.set_message(format!(
            "Installed: {} | Skipped: {} | Failed: {}",
            self.installed, self.skipped, self.failed
        ));
    }

    /// Finish all progress bars
    pub fn finish(&self) {
        for bar in &self.package_bars {
            bar.finish();
        }
        self.summary_bar.finish_and_clear();
    }

    /// Get completion statistics
    pub fn stats(&self) -> ProgressStats {
        ProgressStats {
            total: self.total,
            completed: self.completed,
            installed: self.installed,
            skipped: self.skipped,
            failed: self.failed,
        }
    }

    /// Print a line (suspending progress bars)
    pub fn println(&self, msg: &str) {
        let _ = self.manager.println(msg);
    }
}

/// Statistics from progress tracking
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProgressStats {
    /// Total packages
    pub total: usize,
    /// Completed packages
    pub completed: usize,
    /// Successfully installed packages
    pub installed: usize,
    /// Skipped packages
    pub skipped: usize,
    /// Failed packages
    pub failed: usize,
}

#[allow(dead_code)]
impl ProgressStats {
    /// Check if all packages were processed
    pub fn is_complete(&self) -> bool {
        self.completed == self.total
    }

    /// Check if all packages succeeded (installed or skipped)
    pub fn is_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Simple progress bar wrapper for single operations
#[allow(dead_code)]
pub struct SimpleProgress {
    bar: ProgressBar,
}

#[allow(dead_code)]
impl SimpleProgress {
    /// Create a new simple progress bar
    pub fn new(total: u64) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(ProgressStyles::install());
        bar.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        Self { bar }
    }

    /// Create a spinner (indeterminate progress)
    pub fn spinner(message: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(ProgressStyles::spinner());
        bar.set_message(message.to_string());
        bar.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
        Self { bar }
    }

    /// Create a hidden progress bar (for quiet mode)
    pub fn hidden() -> Self {
        Self {
            bar: ProgressBar::hidden(),
        }
    }

    /// Set the current position
    pub fn set_position(&self, pos: u64) {
        self.bar.set_position(pos);
    }

    /// Increment the position
    pub fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }

    /// Set the message
    pub fn set_message(&self, message: impl Into<std::borrow::Cow<'static, str>>) {
        self.bar.set_message(message);
    }

    /// Finish with a message
    pub fn finish_with_message(&self, message: impl Into<std::borrow::Cow<'static, str>>) {
        self.bar.finish_with_message(message);
    }

    /// Finish and clear
    pub fn finish_and_clear(&self) {
        self.bar.finish_and_clear();
    }

    /// Finish the progress bar
    pub fn finish(&self) {
        self.bar.finish();
    }

    /// Get the underlying progress bar
    pub fn bar(&self) -> &ProgressBar {
        &self.bar
    }
}

/// Format a duration in a human-readable way
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        if remaining_secs > 0 {
            format!("{}m {}s", mins, remaining_secs)
        } else {
            format!("{}m", mins)
        }
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}h {}m", hours, mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(60)), "1m");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_progress_stats() {
        let stats = ProgressStats {
            total: 5,
            completed: 5,
            installed: 3,
            skipped: 1,
            failed: 1,
        };

        assert!(stats.is_complete());
        assert!(!stats.is_successful());
    }

    #[test]
    fn test_progress_manager_quiet_mode() {
        let manager = ProgressManager::quiet();
        assert!(!manager.is_enabled());
        assert!(manager.is_quiet());
    }
}
