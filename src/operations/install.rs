//! Install operation module
//!
//! This module implements the `anvil install` command which applies
//! a workload configuration to the system by:
//! 1. Validating winget availability
//! 2. Running pre-installation scripts
//! 3. Installing packages via winget (with progress tracking)
//! 4. Copying files to target destinations
//! 5. Running post-installation scripts

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::cli::commands::InstallArgs;
use crate::cli::output::{print_error, print_info, print_success, print_warning};
use crate::cli::progress::{format_duration, InstallProgress, ProgressManager, SimpleProgress};
use crate::cli::Cli;
use crate::config::workload::{WingetPackage, Workload};
use crate::config::ConfigManager;
use crate::providers::backup::BackupManager;
use crate::providers::filesystem::{CopyOptions, CopyResult};
use crate::providers::script::{
    OutputMode, ScriptConfig, ScriptContext, ScriptExecutionResult, ScriptExecutionSummary,
    ScriptPhase, ScriptProvider,
};
use crate::providers::template::TemplateProcessor;
use crate::providers::{FilesystemProvider, ProviderConfig, WingetProvider};
use crate::state::{FileState, FileStateManager, InstallationState, PackageCache};

use super::{resolve_workload_path, OperationContext};

/// Package installation plan entry
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PackagePlanEntry {
    /// The package to install
    pub package: WingetPackage,
    /// Action to take
    pub action: PackageAction,
    /// Currently installed version (if any)
    pub installed_version: Option<String>,
    /// Available version for upgrade (if any)
    pub available_version: Option<String>,
}

/// Action to take for a package
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageAction {
    /// Package needs to be installed
    Install,
    /// Package is already installed at correct version
    Skip,
    /// Package needs to be upgraded
    Upgrade,
    /// Package needs to be reinstalled
    Reinstall,
}

impl std::fmt::Display for PackageAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageAction::Install => write!(f, "install"),
            PackageAction::Skip => write!(f, "skip"),
            PackageAction::Upgrade => write!(f, "upgrade"),
            PackageAction::Reinstall => write!(f, "reinstall"),
        }
    }
}

/// Installation summary
#[derive(Debug, Default)]
pub struct InstallationSummary {
    /// Packages successfully installed
    pub installed: usize,
    /// Packages skipped (already installed)
    pub skipped: usize,
    /// Packages upgraded
    pub upgraded: usize,
    /// Packages failed
    pub failed: usize,
    /// Total duration
    pub duration: std::time::Duration,
    /// Whether a reboot is required
    pub reboot_required: bool,
    /// Failed package names
    pub failed_packages: Vec<String>,
}

impl InstallationSummary {
    /// Check if installation was fully successful
    pub fn is_successful(&self) -> bool {
        self.failed == 0
    }

    /// Get total packages processed
    #[allow(dead_code)]
    pub fn total(&self) -> usize {
        self.installed + self.skipped + self.upgraded + self.failed
    }
}

/// Execute the install operation
pub fn execute(args: &InstallArgs, cli: &Cli) -> Result<()> {
    let start_time = Instant::now();

    // Resolve the workload path
    let workload_path = resolve_workload_path(&args.workload, None)
        .with_context(|| format!("Failed to find workload: {}", args.workload))?;

    // Load the workload
    let mut config_manager = ConfigManager::new();
    let workload = config_manager
        .load_resolved(&workload_path.to_string_lossy())
        .with_context(|| format!("Failed to load workload: {}", workload_path.display()))?;

    // Create operation context
    let workload_dir = workload_path.parent().unwrap_or(&workload_path);
    let context = OperationContext::new(
        &workload,
        workload_dir,
        args.dry_run,
        cli.verbosity_level(),
        !cli.should_disable_color(),
    );

    // Check winget availability first (skip if files_only)
    if !args.skip_packages && !args.files_only {
        check_winget_availability(&context)?;
    }

    // Compute total file count including files in directories
    let files_dir = workload_dir.join("files");
    let total_file_count = if let Some(files) = &workload.files {
        count_total_files(files, &files_dir)
    } else {
        0
    };

    // Print header
    print_install_header(&workload, args.dry_run, total_file_count);

    // Generate installation plan if not skipping packages (skip if files_only)
    let plan = if !args.skip_packages && !args.files_only {
        Some(generate_installation_plan(&workload, &context, args)?)
    } else {
        None
    };

    // Show plan and confirm with user unless --force is set
    if let Some(ref plan) = plan {
        print_installation_plan(plan, &context);

        if plan.iter().all(|p| p.action == PackageAction::Skip) {
            print_info("All packages are already installed!");
            if args.packages_only {
                return Ok(());
            }
        }
    }

    if !args.force && !args.dry_run && !confirm_installation(&workload)? {
        print_info("Installation cancelled by user");
        return Ok(());
    }

    // Initialize installation state
    let mut state = InstallationState::new(&workload.name, &workload.version);

    // Run pre-installation scripts (skip if files_only)
    let mut pre_script_summary = ScriptExecutionSummary::new();
    if !args.skip_scripts && !args.skip_pre_scripts && !args.files_only {
        pre_script_summary = run_pre_install_scripts(&context, args)?;
    }

    // Install packages (skip if files_only)
    let mut summary = InstallationSummary::default();
    if !args.skip_packages && !args.files_only {
        if let Some(plan) = plan {
            summary = install_packages_with_progress(&context, args, plan, &mut state)?;
        }
    }

    // Copy files
    if (!args.skip_files && !args.packages_only) || args.files_only {
        copy_files(&context, args)?;
    }

    // Run post-installation scripts (skip if files_only)
    let mut post_script_summary = ScriptExecutionSummary::new();
    if !args.skip_scripts && !args.skip_post_scripts && !args.packages_only && !args.files_only {
        post_script_summary = run_post_install_scripts(&context, args)?;
    }

    // Check if any scripts require reboot
    if pre_script_summary.requires_reboot || post_script_summary.requires_reboot {
        summary.reboot_required = true;
    }

    // Mark state as complete and save
    state.mark_complete();
    if let Err(e) = state.save() {
        context.warn(&format!("Failed to save installation state: {}", e));
    }

    // Invalidate package cache after installation
    let mut cache = PackageCache::load().unwrap_or_default();
    cache.clear();
    let _ = cache.save();

    // Print final summary
    summary.duration = start_time.elapsed();
    print_final_summary(&summary, args.dry_run, &workload.name);

    if !summary.is_successful() {
        print_warning(&format!(
            "Some packages failed to install. Run 'anvil health {}' to check status.",
            workload.name
        ));
    }

    Ok(())
}

/// Check if winget is available
fn check_winget_availability(context: &OperationContext) -> Result<()> {
    let spinner = if context.verbosity >= 1 {
        Some(SimpleProgress::spinner("Checking winget availability..."))
    } else {
        None
    };

    let mut provider = WingetProvider::new();
    match provider.check_availability() {
        Ok(info) => {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }

            if context.verbosity >= 2 {
                print_info(&format!("Winget version: {}", info.version));
            }

            if !info.meets_minimum {
                print_warning(&format!(
                    "Winget version {} is below recommended minimum {}. Some features may not work correctly.",
                    info.version, info.minimum_version
                ));
            }

            Ok(())
        }
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }

            print_error("Windows Package Manager (winget) is not available.");
            println!();
            println!("{}", WingetProvider::get_installation_instructions());
            anyhow::bail!("Winget not available: {}", e);
        }
    }
}

/// Generate installation plan by checking current package state
fn generate_installation_plan(
    workload: &Workload,
    context: &OperationContext,
    args: &InstallArgs,
) -> Result<Vec<PackagePlanEntry>> {
    let packages = match &workload.packages {
        Some(p) => p.winget.as_deref().unwrap_or(&[]),
        None => return Ok(Vec::new()),
    };

    if packages.is_empty() {
        return Ok(Vec::new());
    }

    let spinner = if context.verbosity >= 1 && !context.dry_run {
        Some(SimpleProgress::spinner("Checking installed packages..."))
    } else {
        None
    };

    let provider = WingetProvider::new();
    let mut cache = PackageCache::load().unwrap_or_default();
    let mut plan = Vec::with_capacity(packages.len());

    for package in packages {
        let mut entry = PackagePlanEntry {
            package: package.clone(),
            action: PackageAction::Install,
            installed_version: None,
            available_version: None,
        };

        // Check cache first
        if let Some(cached) = cache.get(&package.id) {
            entry.installed_version = cached.installed_version.clone();
            if cached.is_installed {
                entry.action = determine_action(package, &entry.installed_version, args);
            }
        } else {
            // Query winget
            match provider.is_installed(&package.id) {
                Ok(true) => {
                    if let Ok(Some(version)) = provider.get_installed_version(&package.id) {
                        cache.mark_installed(&package.id, &version, Some("winget".to_string()));
                        entry.installed_version = Some(version);
                    }
                    entry.action = determine_action(package, &entry.installed_version, args);
                }
                Ok(false) => {
                    cache.mark_not_installed(&package.id);
                    entry.action = PackageAction::Install;
                }
                Err(e) => {
                    context.debug(&format!(
                        "Could not check if {} is installed: {}",
                        package.id, e
                    ));
                    entry.action = PackageAction::Install;
                }
            }
        }

        plan.push(entry);
    }

    // Save updated cache
    let _ = cache.save();

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    Ok(plan)
}

/// Determine what action to take for a package
fn determine_action(
    package: &WingetPackage,
    installed_version: &Option<String>,
    _args: &InstallArgs,
) -> PackageAction {
    match installed_version {
        Some(installed) => {
            if let Some(ref required) = package.version {
                // Check if installed version matches required
                if installed == required {
                    PackageAction::Skip
                } else {
                    // Version mismatch - need to upgrade/reinstall
                    PackageAction::Upgrade
                }
            } else {
                // No specific version required, already installed
                PackageAction::Skip
            }
        }
        None => PackageAction::Install,
    }
}

/// Print the installation header
fn print_install_header(workload: &Workload, dry_run: bool, file_count: usize) {
    println!();
    if dry_run {
        println!("=== DRY RUN: Install Workload: {} ===", workload.name);
    } else {
        println!("=== Install Workload: {} ===", workload.name);
    }
    println!("Version:     {}", workload.version);
    println!("Description: {}", workload.description);
    println!("Packages:    {}", workload.package_count());
    println!("Files:       {}", file_count);
    println!("Scripts:     {}", workload.script_count());
    println!();
}

/// Print the installation plan
fn print_installation_plan(plan: &[PackagePlanEntry], context: &OperationContext) {
    use colored::Colorize;

    println!("Installation Plan:");
    println!();

    for entry in plan {
        let version_info = entry
            .installed_version
            .as_ref()
            .map(|v| format!(" ({})", v))
            .unwrap_or_default();

        let target_version = entry
            .package
            .version
            .as_ref()
            .map(|v| format!(" -> {}", v))
            .unwrap_or_default();

        match entry.action {
            PackageAction::Skip => {
                if context.use_color {
                    println!(
                        "  {} {:<40} {}",
                        "✓".green(),
                        entry.package.id,
                        format!("(already installed{})", version_info).dimmed()
                    );
                } else {
                    println!(
                        "  ✓ {:<40} (already installed{})",
                        entry.package.id, version_info
                    );
                }
            }
            PackageAction::Install => {
                if context.use_color {
                    println!(
                        "  {} {:<40} {}",
                        "↓".cyan(),
                        entry.package.id,
                        format!("(will install{})", target_version).cyan()
                    );
                } else {
                    println!(
                        "  ↓ {:<40} (will install{})",
                        entry.package.id, target_version
                    );
                }
            }
            PackageAction::Upgrade => {
                if context.use_color {
                    println!(
                        "  {} {:<40} {}",
                        "↑".yellow(),
                        entry.package.id,
                        format!("(will upgrade{}{})", version_info, target_version).yellow()
                    );
                } else {
                    println!(
                        "  ↑ {:<40} (will upgrade{}{})",
                        entry.package.id, version_info, target_version
                    );
                }
            }
            PackageAction::Reinstall => {
                if context.use_color {
                    println!(
                        "  {} {:<40} {}",
                        "↻".magenta(),
                        entry.package.id,
                        "(will reinstall)".magenta()
                    );
                } else {
                    println!("  ↻ {:<40} (will reinstall)", entry.package.id);
                }
            }
        }
    }

    println!();
}

/// Confirm installation with the user
fn confirm_installation(workload: &Workload) -> Result<bool> {
    use std::io::{self, Write};

    print!(
        "Do you want to proceed with installing '{}'? [y/N] ",
        workload.name
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
}

/// Run pre-installation scripts with enhanced output and tracking
fn run_pre_install_scripts(
    context: &OperationContext,
    _args: &InstallArgs,
) -> Result<ScriptExecutionSummary> {
    let scripts = match &context.workload.scripts {
        Some(s) => s.pre_install.as_ref(),
        None => None,
    };

    let mut summary = ScriptExecutionSummary::new();

    if scripts.is_none() || scripts.unwrap().is_empty() {
        context.debug("No pre-installation scripts to run");
        return Ok(summary);
    }

    let scripts = scripts.unwrap();
    println!();
    print_info(&format!(
        "Running {} pre-installation script(s)...",
        scripts.len()
    ));

    let scripts_dir = context.workload_path.join("scripts");
    let mut provider = ScriptProvider::new()
        .with_dry_run(context.dry_run)
        .with_verbose(context.verbosity > 1)
        .with_base_path(&scripts_dir);

    // Create script context for environment injection
    let script_context = ScriptContext::new(&context.workload.name, &context.workload_path)
        .with_phase(ScriptPhase::PreInstall)
        .with_dry_run(context.dry_run)
        .with_verbose(context.verbosity > 1);

    for (idx, script) in scripts.iter().enumerate() {
        let script_name = script
            .description
            .clone()
            .unwrap_or_else(|| script.path.clone());

        println!();
        print_info(&format!(
            "  [{}/{}] {}",
            idx + 1,
            scripts.len(),
            script_name
        ));

        // Build script configuration
        let mut config = ScriptConfig::new(&script.path)
            .with_timeout(Duration::from_secs(script.timeout))
            .with_elevated(script.elevated)
            .with_working_dir(&scripts_dir);

        // Set output mode based on verbosity
        if context.verbosity > 0 {
            config = config
                .with_output_mode(OutputMode::Both)
                .with_output_prefix("        ");
        }

        // Inject environment variables
        provider.inject_environment_variables(&mut config, &script_context);

        let start = Instant::now();
        match provider.execute(&config) {
            Ok(result) => {
                let exec_result = ScriptExecutionResult::from_result(
                    &result,
                    script_name.clone(),
                    scripts_dir.join(&script.path),
                    ScriptPhase::PreInstall,
                );

                if result.success {
                    print_success(&format!(
                        "        ✓ {} completed ({:.1}s)",
                        script.path,
                        result.duration.as_secs_f64()
                    ));

                    if result.requires_reboot {
                        print_warning("        Script indicates reboot required");
                    }

                    summary.add_result(exec_result);
                } else {
                    print_error(&format!(
                        "        ✗ {} failed (exit code: {})",
                        script.path, result.exit_code
                    ));

                    if !result.stderr.is_empty() && context.verbosity > 0 {
                        for line in result.stderr.lines().take(5) {
                            eprintln!("          {}", line);
                        }
                    }

                    summary.add_result(exec_result);
                    anyhow::bail!("Pre-installation script failed: {}", script.path);
                }
            }
            Err(e) => {
                let duration = start.elapsed();

                // Create a failed result for tracking
                let exec_result = ScriptExecutionResult {
                    script_name: script_name.clone(),
                    script_path: scripts_dir.join(&script.path),
                    phase: ScriptPhase::PreInstall,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    duration,
                    success: false,
                    requires_reboot: false,
                };
                summary.add_result(exec_result);

                // Provide helpful error messages
                match &e {
                    crate::providers::script::ScriptError::ElevationRequired { .. } => {
                        print_error(&format!("        ✗ {} requires elevation", script.path));
                        print_warning(
                            "        Run Anvil as Administrator or use --skip-pre-scripts",
                        );
                    }
                    crate::providers::script::ScriptError::Timeout {
                        timeout_seconds, ..
                    } => {
                        print_error(&format!(
                            "        ✗ {} timed out after {}s",
                            script.path, timeout_seconds
                        ));
                        print_warning("        Increase timeout in workload.yaml or check script");
                    }
                    _ => {
                        print_error(&format!("        ✗ {} error: {}", script.path, e));
                    }
                }

                anyhow::bail!("Pre-installation script error: {}", e);
            }
        }
    }

    if summary.succeeded > 0 {
        println!();
        print_success(&format!(
            "Pre-installation scripts complete ({} succeeded)",
            summary.succeeded
        ));
    }

    Ok(summary)
}

/// Install packages with progress tracking
fn install_packages_with_progress(
    context: &OperationContext,
    _args: &InstallArgs,
    plan: Vec<PackagePlanEntry>,
    state: &mut InstallationState,
) -> Result<InstallationSummary> {
    let packages_to_process: Vec<_> = plan
        .iter()
        .filter(|p| p.action != PackageAction::Skip)
        .collect();

    let total_to_install = packages_to_process.len();
    let total_skipped = plan.len() - total_to_install;

    if total_to_install == 0 {
        return Ok(InstallationSummary {
            skipped: total_skipped,
            ..Default::default()
        });
    }

    print_info(&format!(
        "Installing {} package(s) ({} already installed)...",
        total_to_install, total_skipped
    ));
    println!();

    // Initialize state for all packages
    for entry in &plan {
        state.add_package(&entry.package.id, entry.package.version.clone());
        if entry.action == PackageAction::Skip {
            if let Some(record) = state.get_package_mut(&entry.package.id) {
                record.mark_skipped("Already installed");
            }
        }
    }

    // Create progress manager
    let progress_manager = if context.verbosity >= 1 && !context.dry_run {
        Arc::new(ProgressManager::new())
    } else {
        Arc::new(ProgressManager::quiet())
    };

    // Create provider
    let provider = WingetProvider::with_config(ProviderConfig {
        dry_run: context.dry_run,
        verbose: context.verbosity >= 2,
        retry_count: 3,
        ..Default::default()
    });

    let mut summary = InstallationSummary {
        skipped: total_skipped,
        ..Default::default()
    };

    // Install packages one by one with progress
    let mut install_progress = InstallProgress::new(progress_manager.clone(), total_to_install);

    for entry in packages_to_process {
        let idx = install_progress.start_package(&entry.package.id);

        // Update state
        if let Some(record) = state.get_package_mut(&entry.package.id) {
            record.mark_installing();
        }

        let start_time = Instant::now();

        // Perform installation based on action
        let result = match entry.action {
            PackageAction::Install | PackageAction::Reinstall => provider.install(&entry.package),
            PackageAction::Upgrade => provider.upgrade(&entry.package.id),
            PackageAction::Skip => unreachable!(),
        };

        let duration = start_time.elapsed();

        match result {
            Ok(install_result) => {
                if install_result.success {
                    let version_msg = install_result
                        .installed_version
                        .as_ref()
                        .map(|v| format!(" ({})", v))
                        .unwrap_or_default();

                    let action_msg = match entry.action {
                        PackageAction::Install => "Installed",
                        PackageAction::Upgrade => "Upgraded",
                        PackageAction::Reinstall => "Reinstalled",
                        _ => "Completed",
                    };

                    install_progress.complete_package(
                        idx,
                        &format!(
                            "{}{} in {}",
                            action_msg,
                            version_msg,
                            format_duration(duration)
                        ),
                    );

                    // Update state
                    if let Some(record) = state.get_package_mut(&entry.package.id) {
                        match entry.action {
                            PackageAction::Upgrade => {
                                record.mark_upgraded(
                                    install_result.installed_version.clone(),
                                    duration.as_secs_f64(),
                                    install_result.reboot_required,
                                );
                                summary.upgraded += 1;
                            }
                            _ => {
                                record.mark_installed(
                                    install_result.installed_version.clone(),
                                    duration.as_secs_f64(),
                                    install_result.reboot_required,
                                );
                                summary.installed += 1;
                            }
                        }
                    }

                    if install_result.reboot_required {
                        summary.reboot_required = true;
                    }
                } else {
                    install_progress.fail_package(
                        idx,
                        install_result.message.as_deref().unwrap_or("Unknown error"),
                    );

                    if let Some(record) = state.get_package_mut(&entry.package.id) {
                        record.mark_failed(
                            install_result.message.unwrap_or_default(),
                            duration.as_secs_f64(),
                        );
                    }

                    summary.failed += 1;
                    summary.failed_packages.push(entry.package.id.clone());
                }
            }
            Err(e) => {
                // Check if it's an "already installed" error (which is actually success)
                let error_str = e.to_string();
                if error_str.contains("already installed") {
                    install_progress.skip_package(idx, "Already installed");

                    if let Some(record) = state.get_package_mut(&entry.package.id) {
                        record.mark_skipped("Already installed");
                    }

                    summary.skipped += 1;
                } else {
                    install_progress.fail_package(idx, &error_str);

                    if let Some(record) = state.get_package_mut(&entry.package.id) {
                        record.mark_failed(&error_str, duration.as_secs_f64());
                    }

                    summary.failed += 1;
                    summary.failed_packages.push(entry.package.id.clone());

                    // Print suggestion if available
                    if let Some(suggestion) = e.suggestion() {
                        install_progress.println(&format!("    Hint: {}", suggestion));
                    }
                }
            }
        }

        // Save state periodically
        let _ = state.save();
    }

    install_progress.finish();
    println!();

    Ok(summary)
}

/// Result of file copy operations
#[derive(Debug, Default)]
pub struct FileCopyResult {
    /// Files successfully copied
    pub copied: usize,
    /// Files skipped (identical)
    pub skipped: usize,
    /// Files backed up before overwrite
    pub backed_up: usize,
    /// Files processed as templates
    pub templated: usize,
    /// Files that failed
    pub failed: usize,
    /// Failed file names
    pub failed_files: Vec<String>,
}

impl FileCopyResult {
    /// Check if all file operations succeeded
    #[allow(dead_code)]
    pub fn is_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Copy files to target destinations with comprehensive handling
fn copy_files(context: &OperationContext, args: &InstallArgs) -> Result<()> {
    let files = match &context.workload.files {
        Some(f) => f,
        None => return Ok(()),
    };

    if files.is_empty() {
        context.debug("No files to copy");
        return Ok(());
    }

    // Count total files including those in directories
    let files_dir = context.workload_path.join("files");
    let total_file_count = count_total_files(files, &files_dir);

    // Note: total_file_count is computed here for the progress display,
    // but the header already shows the computed count from execute()

    println!();
    print_info(&format!("Copying {} file(s)...", total_file_count));

    // Initialize providers
    let mut fs_provider = FilesystemProvider::with_config(ProviderConfig {
        dry_run: context.dry_run,
        verbose: context.verbosity >= 2,
        ..Default::default()
    });

    // Initialize template processor with workload context
    let template_processor =
        TemplateProcessor::with_workload(&context.workload.name, &context.workload.version);

    // Initialize backup manager
    let backup_manager = match BackupManager::new() {
        Ok(mgr) => Some(if context.dry_run {
            mgr.with_dry_run(true)
        } else {
            mgr
        }),
        Err(e) => {
            context.warn(&format!("Failed to initialize backup manager: {}", e));
            None
        }
    };

    // Initialize file state manager
    let mut state_manager = match FileStateManager::new() {
        Ok(mgr) => Some(mgr),
        Err(e) => {
            context.warn(&format!("Failed to initialize file state manager: {}", e));
            None
        }
    };

    let files_dir = context.workload_path.join("files");
    let mut result = FileCopyResult::default();
    let total_files = files.len();

    // First pass: validate all source files/directories exist
    // Tuple: (source_path, destination_pattern, backup, is_template, source_rel, is_directory)
    let mut valid_files: Vec<(std::path::PathBuf, String, bool, bool, Option<String>, bool)> =
        Vec::new();
    for file in files {
        let source = files_dir.join(&file.source);

        // Check for glob patterns
        if file.source.contains('*') || file.source.contains('?') {
            match fs_provider.expand_glob(&file.source, &files_dir) {
                Ok(matches) => {
                    for match_path in matches {
                        let relative = match_path.strip_prefix(&files_dir).unwrap_or(&match_path);
                        let is_dir = match_path.is_dir();
                        valid_files.push((
                            match_path.clone(),
                            file.destination.clone(),
                            file.backup,
                            file.template
                                || (!is_dir && TemplateProcessor::is_template_file(&match_path)),
                            Some(relative.to_string_lossy().to_string()),
                            is_dir,
                        ));
                    }
                }
                Err(e) => {
                    print_warning(&format!(
                        "Failed to expand glob pattern '{}': {}",
                        file.source, e
                    ));
                }
            }
        } else if !source.exists() {
            print_warning(&format!("Source file not found: {}", source.display()));
            result.failed += 1;
            result.failed_files.push(file.source.clone());
        } else {
            let is_dir = source.is_dir();
            valid_files.push((
                source,
                file.destination.clone(),
                file.backup,
                file.template
                    || (!is_dir
                        && TemplateProcessor::is_template_file(&files_dir.join(&file.source))),
                Some(file.source.clone()),
                is_dir,
            ));
        }
    }

    // Second pass: copy files and directories
    for (idx, (source, destination_pattern, backup, is_template, source_rel, is_directory)) in
        valid_files.iter().enumerate()
    {
        let file_num = idx + 1;

        // Expand destination path variables
        let destination =
            crate::config::expand_variables(destination_pattern, Some(&context.workload.name));
        let dest_path = std::path::PathBuf::from(&destination);

        // Determine final destination (handle template extension stripping for files)
        let final_dest = if *is_template && !*is_directory {
            TemplateProcessor::output_filename(&dest_path)
        } else {
            dest_path.clone()
        };

        // Display progress
        let source_name = source_rel.as_ref().map(|s| s.as_str()).unwrap_or_else(|| {
            source
                .file_name()
                .map(|n| n.to_str().unwrap_or(""))
                .unwrap_or("")
        });

        if *is_directory {
            print_info(&format!(
                "  [{}/{}] {}/ -> {}/ (directory)",
                file_num,
                total_files,
                source_name,
                final_dest.display()
            ));
        } else {
            print_info(&format!(
                "  [{}/{}] {} -> {}",
                file_num,
                total_files,
                source_name,
                final_dest.display()
            ));
        }

        // Handle directory copy
        if *is_directory {
            let dir_result = copy_directory_recursive(
                context,
                &mut fs_provider,
                source,
                &final_dest,
                *backup && !args.no_backup,
                args.force_files,
                backup_manager.as_ref(),
                &mut state_manager,
                source_rel.clone().unwrap_or_default(),
            );

            match dir_result {
                Ok((copied, skipped, backed_up, failed)) => {
                    result.copied += copied;
                    result.skipped += skipped;
                    result.backed_up += backed_up;
                    result.failed += failed;

                    if failed > 0 {
                        print_warning(&format!(
                            "        Directory: {} files copied, {} skipped, {} failed",
                            copied, skipped, failed
                        ));
                    } else {
                        println!(
                            "        ✓ Directory copied ({} files, {} skipped)",
                            copied, skipped
                        );
                    }
                }
                Err(e) => {
                    print_error(&format!("        ✗ Failed to copy directory: {}", e));
                    result.failed += 1;
                    result.failed_files.push(source_name.to_string());
                }
            }
            continue;
        }

        // Check for conflicts with other workloads (for files only)
        if let Some(ref state_mgr) = state_manager {
            if let Some(conflicting_workload) =
                state_mgr.would_conflict(&final_dest, &context.workload.name)
            {
                print_warning(&format!(
                    "        File is managed by workload '{}', overwriting",
                    conflicting_workload
                ));
            }
        }

        // Process file
        let copy_result = if *is_template {
            // Process template
            process_template_file(
                context,
                &mut fs_provider,
                &template_processor,
                source,
                &final_dest,
                *backup && !args.no_backup,
                backup_manager.as_ref(),
            )
        } else {
            // Regular file copy
            copy_regular_file(
                context,
                &mut fs_provider,
                source,
                &final_dest,
                *backup && !args.no_backup,
                args.force_files,
                backup_manager.as_ref(),
            )
        };

        match copy_result {
            Ok(copy_info) => {
                if copy_info.skipped {
                    println!("        ↷ Skipped (identical)");
                    result.skipped += 1;
                } else {
                    if copy_info.backup_info.is_some() {
                        println!("        ✓ Backed up existing file");
                        result.backed_up += 1;
                    }

                    if *is_template {
                        println!("        ✓ Processed template");
                        result.templated += 1;
                    }

                    let size_str = format_file_size(copy_info.size);
                    println!("        ✓ Copied ({})", size_str);
                    result.copied += 1;

                    // Record in state
                    if let Some(ref mut state_mgr) = state_manager {
                        let file_state = FileState::new(
                            final_dest.clone(),
                            copy_info.hash.clone(),
                            copy_info.hash.clone(),
                            context.workload.name.clone(),
                            *is_template,
                        )
                        .with_size(copy_info.size)
                        .with_source_path(source_rel.clone().unwrap_or_default());

                        // Add backup ID if we created a backup
                        let file_state = if let Some(ref backup_info) = copy_info.backup_info {
                            // Generate backup ID from hash
                            let backup_id =
                                backup_info.hash.chars().skip(7).take(6).collect::<String>();
                            file_state.with_backup_id(backup_id)
                        } else {
                            file_state
                        };

                        if let Err(e) = state_mgr.record_install_full(file_state) {
                            context.warn(&format!("Failed to record file state: {}", e));
                        }
                    }
                }
            }
            Err(e) => {
                print_error(&format!("        ✗ Failed: {}", e));
                result.failed += 1;
                result.failed_files.push(source_name.to_string());
            }
        }
    }

    // Save file state
    if let Some(ref mut state_mgr) = state_manager {
        if let Err(e) = state_mgr.save() {
            context.warn(&format!("Failed to save file state: {}", e));
        }
    }

    // Print summary
    println!();
    print_info(&format!(
        "✓ Files copied: {}",
        result.copied + result.templated
    ));
    if result.skipped > 0 {
        print_info(&format!("  Skipped: {} (identical)", result.skipped));
    }
    if result.backed_up > 0 {
        print_info(&format!("  Backed up: {}", result.backed_up));
    }
    if result.failed > 0 {
        print_error(&format!("  Failed: {}", result.failed));
    }

    Ok(())
}

/// Process a template file and copy to destination
fn process_template_file(
    context: &OperationContext,
    fs_provider: &mut FilesystemProvider,
    template_processor: &TemplateProcessor,
    source: &std::path::Path,
    destination: &std::path::Path,
    backup: bool,
    backup_manager: Option<&BackupManager>,
) -> Result<CopyResult, anyhow::Error> {
    // Read and render template
    let rendered = template_processor
        .render_file(source)
        .with_context(|| format!("Failed to render template: {}", source.display()))?;

    // Compute hash of rendered content
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(rendered.as_bytes());
    let rendered_hash = format!("sha256:{:x}", hasher.finalize());

    // Check if destination exists and is identical
    if destination.exists() {
        let dest_hash = fs_provider.compute_hash(destination)?;
        if dest_hash == rendered_hash {
            return Ok(CopyResult {
                source: source.to_path_buf(),
                destination: destination.to_path_buf(),
                hash: rendered_hash,
                size: rendered.len() as u64,
                backup_info: None,
                skipped: true,
            });
        }
    }

    // Backup existing file if requested
    let backup_info = if backup && destination.exists() {
        if let Some(mgr) = backup_manager {
            match mgr.backup_file(destination, &context.workload.name) {
                Ok(entry) => Some(crate::providers::filesystem::BackupInfo {
                    original_path: entry.original_path,
                    backup_path: entry.backup_path,
                    timestamp: entry.timestamp,
                    hash: entry.hash,
                }),
                Err(e) => {
                    context.warn(&format!("Failed to backup file: {}", e));
                    None
                }
            }
        } else {
            // Fallback to filesystem provider backup
            fs_provider.backup_file(destination).ok()
        }
    } else {
        None
    };

    // Write rendered content
    if context.dry_run {
        tracing::info!("Would write rendered template to {}", destination.display());
    } else {
        fs_provider.write(destination, &rendered)?;
    }

    Ok(CopyResult {
        source: source.to_path_buf(),
        destination: destination.to_path_buf(),
        hash: rendered_hash,
        size: rendered.len() as u64,
        backup_info,
        skipped: false,
    })
}

/// Copy a regular (non-template) file
fn copy_regular_file(
    context: &OperationContext,
    fs_provider: &mut FilesystemProvider,
    source: &std::path::Path,
    destination: &std::path::Path,
    backup: bool,
    force: bool,
    backup_manager: Option<&BackupManager>,
) -> Result<CopyResult, anyhow::Error> {
    // Use backup manager for creating backups if available
    let options = CopyOptions {
        backup: false, // We'll handle backup separately
        verify: true,
        atomic: true,
        overwrite: true,
        preserve_attributes: true,
    };

    // Check if files are identical (unless force is set)
    if !force && destination.exists() {
        let source_hash = fs_provider.compute_hash(source)?;
        let dest_hash = fs_provider.compute_hash(destination)?;

        if source_hash == dest_hash {
            let size = fs_provider.file_size(source)?;
            return Ok(CopyResult {
                source: source.to_path_buf(),
                destination: destination.to_path_buf(),
                hash: source_hash,
                size,
                backup_info: None,
                skipped: true,
            });
        }
    }

    // Create backup before overwriting
    let backup_info = if backup && destination.exists() {
        if let Some(mgr) = backup_manager {
            match mgr.backup_file(destination, &context.workload.name) {
                Ok(entry) => Some(crate::providers::filesystem::BackupInfo {
                    original_path: entry.original_path,
                    backup_path: entry.backup_path,
                    timestamp: entry.timestamp,
                    hash: entry.hash,
                }),
                Err(e) => {
                    context.warn(&format!("Failed to backup file: {}", e));
                    None
                }
            }
        } else {
            fs_provider.backup_file(destination).ok()
        }
    } else {
        None
    };

    // Copy the file
    let mut result = fs_provider.copy_file_with_options(source, destination, &options)?;
    result.backup_info = backup_info;

    Ok(result)
}

/// Format file size for display
/// Count total files including files inside directories
fn count_total_files(
    files: &[crate::config::workload::FileEntry],
    files_dir: &std::path::Path,
) -> usize {
    use walkdir::WalkDir;

    let mut count = 0;
    for file in files {
        let source = files_dir.join(&file.source);

        if source.is_dir() {
            // Count files in directory recursively
            for entry in WalkDir::new(&source).into_iter().filter_map(|e| e.ok()) {
                if entry.path() != source && entry.path().is_file() {
                    count += 1;
                }
            }
        } else if source.exists() {
            count += 1;
        } else if file.source.contains('*') || file.source.contains('?') {
            // Glob pattern - try to expand and count
            if let Ok(matches) = glob::glob(&files_dir.join(&file.source).to_string_lossy()) {
                count += matches
                    .filter_map(|m| m.ok())
                    .filter(|p| p.is_file())
                    .count();
            }
        } else {
            // File doesn't exist yet, still count it for display
            count += 1;
        }
    }
    count
}

/// Copy a directory recursively with file state tracking
/// Returns (copied, skipped, backed_up, failed) counts
#[allow(clippy::too_many_arguments)]
fn copy_directory_recursive(
    context: &OperationContext,
    fs_provider: &mut FilesystemProvider,
    source: &std::path::Path,
    destination: &std::path::Path,
    backup: bool,
    force: bool,
    backup_manager: Option<&BackupManager>,
    state_manager: &mut Option<FileStateManager>,
    source_rel_base: String,
) -> Result<(usize, usize, usize, usize), anyhow::Error> {
    use walkdir::WalkDir;

    let mut copied = 0;
    let mut skipped = 0;
    let mut backed_up = 0;
    let mut failed = 0;

    // Walk the source directory
    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();

        // Skip the root directory itself
        if entry_path == source {
            continue;
        }

        // Skip directories (we create them as needed when copying files)
        if entry_path.is_dir() {
            continue;
        }

        // Get relative path from source
        let relative = match entry_path.strip_prefix(source) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Build destination path
        let dest_file = destination.join(relative);

        // Build source relative path for state tracking
        let source_rel = if source_rel_base.is_empty() {
            relative.to_string_lossy().to_string()
        } else {
            format!("{}/{}", source_rel_base, relative.to_string_lossy())
        };

        // Check for conflicts with other workloads
        if let Some(ref state_mgr) = state_manager {
            if let Some(conflicting_workload) =
                state_mgr.would_conflict(&dest_file, &context.workload.name)
            {
                if context.verbosity > 0 {
                    print_warning(&format!(
                        "          {} managed by '{}', overwriting",
                        relative.display(),
                        conflicting_workload
                    ));
                }
            }
        }

        // Copy the file
        match copy_regular_file(
            context,
            fs_provider,
            entry_path,
            &dest_file,
            backup,
            force,
            backup_manager,
        ) {
            Ok(copy_info) => {
                if copy_info.skipped {
                    skipped += 1;
                } else {
                    if copy_info.backup_info.is_some() {
                        backed_up += 1;
                    }
                    copied += 1;

                    // Record in state
                    if let Some(ref mut state_mgr) = state_manager {
                        let file_state = FileState::new(
                            dest_file.clone(),
                            copy_info.hash.clone(),
                            copy_info.hash.clone(),
                            context.workload.name.clone(),
                            false, // Not a template
                        )
                        .with_size(copy_info.size)
                        .with_source_path(source_rel);

                        // Add backup ID if we created a backup
                        let file_state = if let Some(ref backup_info) = copy_info.backup_info {
                            let backup_id =
                                backup_info.hash.chars().skip(7).take(6).collect::<String>();
                            file_state.with_backup_id(backup_id)
                        } else {
                            file_state
                        };

                        if let Err(e) = state_mgr.record_install_full(file_state) {
                            context.warn(&format!("Failed to record file state: {}", e));
                        }
                    }
                }
            }
            Err(e) => {
                if context.verbosity > 0 {
                    print_error(&format!(
                        "          Failed to copy {}: {}",
                        relative.display(),
                        e
                    ));
                }
                failed += 1;
            }
        }
    }

    Ok((copied, skipped, backed_up, failed))
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Run post-installation scripts with enhanced output and tracking
fn run_post_install_scripts(
    context: &OperationContext,
    _args: &InstallArgs,
) -> Result<ScriptExecutionSummary> {
    let scripts = match &context.workload.scripts {
        Some(s) => s.post_install.as_ref(),
        None => None,
    };

    let mut summary = ScriptExecutionSummary::new();

    if scripts.is_none() || scripts.unwrap().is_empty() {
        context.debug("No post-installation scripts to run");
        return Ok(summary);
    }

    let scripts = scripts.unwrap();
    println!();
    print_info(&format!(
        "Running {} post-installation script(s)...",
        scripts.len()
    ));

    let scripts_dir = context.workload_path.join("scripts");
    let mut provider = ScriptProvider::new()
        .with_dry_run(context.dry_run)
        .with_verbose(context.verbosity > 1)
        .with_base_path(&scripts_dir);

    // Create script context for environment injection
    let script_context = ScriptContext::new(&context.workload.name, &context.workload_path)
        .with_phase(ScriptPhase::PostInstall)
        .with_dry_run(context.dry_run)
        .with_verbose(context.verbosity > 1);

    for (idx, script) in scripts.iter().enumerate() {
        let script_name = script
            .description
            .clone()
            .unwrap_or_else(|| script.path.clone());

        println!();
        print_info(&format!(
            "  [{}/{}] {}",
            idx + 1,
            scripts.len(),
            script_name
        ));

        // Build script configuration
        let mut config = ScriptConfig::new(&script.path)
            .with_timeout(Duration::from_secs(script.timeout))
            .with_elevated(script.elevated)
            .with_working_dir(&scripts_dir);

        // Set output mode - always stream for post-install (users want to see progress)
        if context.verbosity > 0 {
            config = config
                .with_output_mode(OutputMode::Both)
                .with_output_prefix("        ");
        } else {
            // Even without verbose, stream output for post-install
            config = config
                .with_output_mode(OutputMode::Stream)
                .with_output_prefix("        ");
        }

        // Inject environment variables
        provider.inject_environment_variables(&mut config, &script_context);

        let start = Instant::now();
        match provider.execute(&config) {
            Ok(result) => {
                let exec_result = ScriptExecutionResult::from_result(
                    &result,
                    script_name.clone(),
                    scripts_dir.join(&script.path),
                    ScriptPhase::PostInstall,
                );

                if result.success {
                    print_success(&format!(
                        "        ✓ {} completed ({:.1}s)",
                        script.path,
                        result.duration.as_secs_f64()
                    ));

                    if result.requires_reboot {
                        print_warning("        Script indicates reboot required");
                        summary.requires_reboot = true;
                    }

                    summary.add_result(exec_result);
                } else {
                    print_warning(&format!(
                        "        ⚠ {} completed with exit code {} ({:.1}s)",
                        script.path,
                        result.exit_code,
                        result.duration.as_secs_f64()
                    ));

                    if !result.stderr.is_empty() && context.verbosity > 0 {
                        for line in result.stderr.lines().take(5) {
                            eprintln!("          {}", line);
                        }
                    }

                    summary.add_result(exec_result);
                    // Don't fail on post-install script non-zero exit, just warn
                }
            }
            Err(e) => {
                let duration = start.elapsed();

                // Create a failed result for tracking
                let exec_result = ScriptExecutionResult {
                    script_name: script_name.clone(),
                    script_path: scripts_dir.join(&script.path),
                    phase: ScriptPhase::PostInstall,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    duration,
                    success: false,
                    requires_reboot: false,
                };
                summary.add_result(exec_result);

                // Provide helpful error messages but don't fail
                match &e {
                    crate::providers::script::ScriptError::ElevationRequired { .. } => {
                        print_warning(&format!(
                            "        ⚠ {} requires elevation (skipped)",
                            script.path
                        ));
                        print_info(
                            "        Run Anvil as Administrator to execute elevated scripts",
                        );
                    }
                    crate::providers::script::ScriptError::Timeout {
                        timeout_seconds, ..
                    } => {
                        print_warning(&format!(
                            "        ⚠ {} timed out after {}s",
                            script.path, timeout_seconds
                        ));
                        print_info("        Increase timeout in workload.yaml if needed");
                    }
                    _ => {
                        print_warning(&format!("        ⚠ {} error: {}", script.path, e));
                    }
                }

                // Don't fail on post-install script errors
                print_info(
                    "        Post-installation script failed, but installation may still work",
                );
            }
        }
    }

    println!();
    if summary.failed > 0 {
        print_warning(&format!(
            "Post-installation scripts: {} succeeded, {} failed",
            summary.succeeded, summary.failed
        ));
    } else if summary.succeeded > 0 {
        print_success(&format!(
            "Post-installation scripts complete ({} succeeded, {:.1}s total)",
            summary.succeeded,
            summary.total_duration.as_secs_f64()
        ));
    }

    Ok(summary)
}

/// Print final installation summary
fn print_final_summary(summary: &InstallationSummary, dry_run: bool, workload_name: &str) {
    use colored::Colorize;

    println!();
    println!("{}", "═".repeat(50).dimmed());

    if dry_run {
        println!("{}", "  Dry Run Summary".bold());
        print_info("  No changes were made to the system");
    } else {
        if summary.is_successful() {
            println!("{}", "  ✓ Installation complete!".green().bold());
        } else {
            println!(
                "{}",
                "  ⚠ Installation completed with errors".yellow().bold()
            );
        }
    }

    println!("{}", "═".repeat(50).dimmed());
    println!();

    // Package summary
    println!("  {}", "Packages:".bold());
    println!("    Installed: {}", summary.installed.to_string().green());
    println!("    Upgraded:  {}", summary.upgraded.to_string().cyan());
    println!("    Skipped:   {}", summary.skipped.to_string().dimmed());
    println!(
        "    Failed:    {}",
        if summary.failed > 0 {
            summary.failed.to_string().red().to_string()
        } else {
            "0".dimmed().to_string()
        }
    );

    println!();
    println!("  {}", "Duration:".bold());
    println!("    Total: {}", format_duration(summary.duration));

    if summary.reboot_required {
        println!();
        println!("{}", "═".repeat(50).yellow());
        print_warning("  A system reboot is required to complete installation");
        println!("{}", "═".repeat(50).yellow());
    }

    if !summary.failed_packages.is_empty() {
        println!();
        print_error("  Failed packages:");
        for pkg in &summary.failed_packages {
            println!("    - {}", pkg.red());
        }
        println!();
        print_info(&format!(
            "  To retry: anvil install {} --retry-failed",
            workload_name
        ));
    }

    println!();
}
