//! Health check operation module
//!
//! This module implements the `anvil health` command, which validates
//! the current system state against a workload definition.

use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::commands::HealthArgs;
use crate::cli::output::{
    get_formatter, print_info, print_warning, CheckResult, CheckStatus, HealthReport,
};
use crate::cli::progress::SimpleProgress;
use crate::cli::Cli;
use crate::config::{expand_variables, ConfigManager};
use crate::providers::backup::compute_file_hash;
use crate::providers::winget::version;
use crate::providers::{create_registry, ProviderConfig};
use crate::state::{CachedPackageInfo, FileStateManager, PackageCache};

use super::resolve_workload_path;

/// Execute the health check command
pub fn execute(args: &HealthArgs, cli: &Cli) -> Result<()> {
    let _use_color = !cli.should_disable_color();
    let verbosity = cli.verbosity_level();
    let quiet = cli.quiet;

    // Load and resolve the workload
    let mut config_manager = ConfigManager::new();
    let workload = config_manager
        .load_resolved(&args.workload)
        .with_context(|| format!("Failed to load workload: {}", args.workload))?;

    // Resolve workload path for script execution
    // resolve_workload_path returns path to workload.yaml, we need the directory
    let workload_yaml_path = resolve_workload_path(&args.workload, None)?;
    let workload_path = workload_yaml_path
        .parent()
        .unwrap_or(&workload_yaml_path)
        .to_path_buf();

    if verbosity > 1 {
        tracing::info!("Checking health for workload: {}", workload.name);
    }

    // Show spinner while checking
    let spinner = if !quiet {
        Some(SimpleProgress::spinner("Checking system health..."))
    } else {
        None
    };

    // Collect all check results
    let mut checks: Vec<CheckResult> = Vec::new();
    let mut _packages_to_fix: Vec<String> = Vec::new();
    let mut _packages_to_update: Vec<String> = Vec::new();

    // Package checks
    if !args.files_only && !args.assertions_only {
        let (package_checks, fix_list, update_list) = check_packages(&workload, verbosity, quiet)?;
        checks.extend(package_checks);
        _packages_to_fix = fix_list;
        _packages_to_update = update_list;

        if args.fail_fast && checks.iter().any(|c| c.status == CheckStatus::Fail) {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }
            return report_and_exit(args, cli, &workload.name, checks);
        }
    }

    // File checks
    if !args.packages_only && !args.assertions_only {
        let file_checks = check_files(&workload, &workload_path, verbosity, args.show_diff)?;
        checks.extend(file_checks);

        if args.fail_fast && checks.iter().any(|c| c.status == CheckStatus::Fail) {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }
            return report_and_exit(args, cli, &workload.name, checks);
        }
    }

    // Assertion checks
    if !args.packages_only && !args.files_only {
        let health_config = workload.health.as_ref().cloned().unwrap_or_default();
        if health_config.assertion_check {
            if let Some(assertions) = &workload.assertions {
                if let Some(s) = &spinner {
                    s.set_message("Evaluating assertions...");
                }

                let assertion_pairs: Vec<(String, crate::conditions::Condition)> = assertions
                    .iter()
                    .map(|a| (a.name.clone(), a.check.clone()))
                    .collect();

                let assertion_results = crate::assertions::evaluate_assertions(&assertion_pairs);
                let assertion_checks = crate::assertions::to_check_results(&assertion_results);
                checks.extend(assertion_checks);

                if args.fail_fast && checks.iter().any(|c| c.status == CheckStatus::Fail) {
                    if let Some(s) = spinner {
                        s.finish_and_clear();
                    }
                    return report_and_exit(args, cli, &workload.name, checks);
                }
            }
        }
    }

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    // Generate and output the report
    report_and_exit(args, cli, &workload.name, checks)
}

/// Check that all required packages are installed
fn check_packages(
    workload: &crate::config::workload::Workload,
    verbosity: u8,
    _quiet: bool,
) -> Result<(Vec<CheckResult>, Vec<String>, Vec<String>)> {
    use std::collections::HashMap;

    let mut results = Vec::new();
    let mut packages_to_fix = Vec::new();
    let mut packages_to_update = Vec::new();

    let registry = create_registry(&ProviderConfig::default());
    let provider = match registry.get("winget") {
        Some(p) => p,
        None => return Ok((results, packages_to_fix, packages_to_update)),
    };
    let provider_name = provider.name();

    // Load cache
    let mut cache = PackageCache::load().unwrap_or_default();
    let mut cache_updated = false;

    let packages = match &workload.packages {
        Some(p) => p.winget.as_deref().unwrap_or(&[]),
        None => return Ok((results, packages_to_fix, packages_to_update)),
    };

    if packages.is_empty() {
        return Ok((results, packages_to_fix, packages_to_update));
    }

    // Separate packages into cached and uncached
    let mut cached_packages: HashMap<String, (bool, Option<String>, Option<String>)> =
        HashMap::new();
    let mut uncached_package_ids: Vec<&str> = Vec::new();

    for package in packages {
        let cached = cache.get_scoped(provider_name, &package.id);
        if let Some(cached_info) = cached {
            cached_packages.insert(
                package.id.to_lowercase(),
                (
                    cached_info.is_installed,
                    cached_info.installed_version.clone(),
                    cached_info.available_version.clone(),
                ),
            );
        } else {
            uncached_package_ids.push(&package.id);
        }
    }

    // If there are uncached packages, do a single batch query
    let mut batch_installed: HashMap<String, Option<String>> = HashMap::new();
    if !uncached_package_ids.is_empty() {
        if verbosity > 1 {
            tracing::debug!(
                "Batch querying {} uncached packages",
                uncached_package_ids.len()
            );
        }

        // Get all installed packages in one call via trait
        match provider.list_installed() {
            Ok(installed_list) => {
                // Build a lookup map of installed packages (lowercase ID -> version)
                let installed_map: HashMap<String, String> = installed_list
                    .into_iter()
                    .filter_map(|p| p.version.map(|v| (p.id.to_lowercase(), v)))
                    .collect();

                // Check each uncached package against the installed list
                let mut not_found_in_batch: Vec<&str> = Vec::new();
                for pkg_id in &uncached_package_ids {
                    let pkg_id_lower = pkg_id.to_lowercase();
                    if let Some(version) = installed_map.get(&pkg_id_lower) {
                        batch_installed.insert(pkg_id_lower.clone(), Some(version.clone()));
                        // Update cache with scoped key
                        let info = CachedPackageInfo::installed(
                            *pkg_id,
                            version.clone(),
                            Some("winget".to_string()),
                        );
                        cache.set_scoped(provider_name, info);
                    } else {
                        // Package not found in batch query, will check individually
                        not_found_in_batch.push(*pkg_id);
                    }
                }

                // For packages not found in the batch query, do individual checks
                // This handles edge cases like IrfanView.PlugIns which don't appear in
                // the general `winget list` output but can be found with `winget list --id`
                for pkg_id in not_found_in_batch {
                    let pkg_id_lower = pkg_id.to_lowercase();
                    if verbosity > 1 {
                        tracing::debug!(
                            "Package '{}' not in batch list, checking individually",
                            pkg_id
                        );
                    }
                    match provider.is_installed(pkg_id) {
                        Ok(Some(version)) => {
                            batch_installed.insert(pkg_id_lower.clone(), Some(version.clone()));
                            let info = CachedPackageInfo::installed(
                                pkg_id,
                                version,
                                Some("winget".to_string()),
                            );
                            cache.set_scoped(provider_name, info);
                        }
                        Ok(None) | Err(_) => {
                            batch_installed.insert(pkg_id_lower.clone(), None);
                            cache.set_scoped(
                                provider_name,
                                CachedPackageInfo::not_installed(pkg_id),
                            );
                        }
                    }
                }
                cache_updated = true;
            }
            Err(e) => {
                if verbosity > 0 {
                    tracing::warn!("Failed to batch query packages: {}", e);
                }
                // Fall back to marking all as unknown - they'll show as not installed
                for pkg_id in &uncached_package_ids {
                    batch_installed.insert(pkg_id.to_lowercase(), None);
                }
            }
        }
    }

    // Now process all packages
    for package in packages {
        let pkg_id_lower = package.id.to_lowercase();

        if verbosity > 1 {
            tracing::debug!("Checking package: {}", package.id);
        }

        // Get package status from cache or batch query
        let (is_installed, installed_version, available_version) =
            if let Some(cached) = cached_packages.get(&pkg_id_lower) {
                cached.clone()
            } else if let Some(version_opt) = batch_installed.get(&pkg_id_lower) {
                (version_opt.is_some(), version_opt.clone(), None)
            } else {
                (false, None, None)
            };

        if is_installed {
            let version_display = installed_version
                .clone()
                .unwrap_or_else(|| "installed".to_string());

            // Check version if specified
            if let Some(expected_version) = &package.version {
                let version_ok = if let Some(ref installed) = installed_version {
                    check_version_constraint(installed, expected_version)
                } else {
                    false
                };

                if !version_ok {
                    results.push(CheckResult::fail(
                        &package.id,
                        "Packages",
                        format!(
                            "Version mismatch: expected {}, found {}",
                            expected_version, version_display
                        ),
                    ));
                    packages_to_fix.push(package.id.clone());
                    continue;
                }
            }

            // Check for available updates (only if we have cached data)
            if let Some(ref available) = available_version {
                if available != &version_display {
                    results.push(CheckResult::warn(
                        &package.id,
                        "Packages",
                        format!("{} (update available: {})", version_display, available),
                    ));
                    packages_to_update.push(package.id.clone());
                    continue;
                }
            }

            results.push(CheckResult::ok_with_message(
                &package.id,
                "Packages",
                version_display,
            ));
        } else {
            results.push(CheckResult::fail(&package.id, "Packages", "Not installed"));
            packages_to_fix.push(package.id.clone());
        }
    }

    // Save cache if updated
    if cache_updated {
        let _ = cache.save();
    }

    Ok((results, packages_to_fix, packages_to_update))
}

/// Check if a version satisfies a constraint
fn check_version_constraint(installed: &str, constraint: &str) -> bool {
    let constraint = constraint.trim();

    // Handle version ranges and constraints
    if constraint.contains(',') || constraint.starts_with('>') || constraint.starts_with('<') {
        return version::matches_constraint(installed, constraint);
    }

    // Handle exact version match (default)
    installed == constraint
}

/// Check that all required files exist and match expected content
fn check_files(
    workload: &crate::config::workload::Workload,
    workload_path: &Path,
    verbosity: u8,
    show_diff: bool,
) -> Result<Vec<CheckResult>> {
    let mut results = Vec::new();

    let files = match &workload.files {
        Some(f) => f.as_slice(),
        None => return Ok(results),
    };

    // Load file state for hash comparison
    let state_manager = FileStateManager::new().unwrap_or_default();

    // Get the files directory for checking if source is a directory
    let files_dir = workload_path.join("files");

    for file in files {
        let dest_path = expand_variables(&file.destination, Some(&workload.name));
        let source_path = files_dir.join(&file.source);

        if verbosity > 1 {
            tracing::debug!("Checking file: {}", dest_path);
        }

        // Check if source is a directory
        if source_path.is_dir() {
            // Handle directory health check
            let dir_results =
                check_directory_health(&dest_path, &source_path, file, &state_manager, show_diff);
            results.extend(dir_results);
        } else {
            let result = check_file_health(&dest_path, file, &state_manager, show_diff);
            results.push(result);
        }
    }

    Ok(results)
}

/// Check health status of a directory recursively
fn check_directory_health(
    dest_path: &str,
    source_path: &Path,
    file: &crate::config::workload::FileEntry,
    state_manager: &FileStateManager,
    show_diff: bool,
) -> Vec<CheckResult> {
    use walkdir::WalkDir;

    let mut results = Vec::new();
    let dest_base = Path::new(dest_path);

    // Check if destination directory exists
    if !dest_base.exists() {
        results.push(CheckResult::fail(
            format!("{}/ (directory)", file.destination),
            "Files",
            "Directory not found",
        ));
        return results;
    }

    if !dest_base.is_dir() {
        results.push(CheckResult::fail(
            format!("{}/ (directory)", file.destination),
            "Files",
            "Expected directory but found file",
        ));
        return results;
    }

    // Track files we expect to find
    let mut expected_files = 0;
    let mut found_ok = 0;
    let mut found_modified = 0;
    let mut found_missing = 0;

    // Walk the source directory to find all expected files
    for entry in WalkDir::new(source_path).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();

        // Skip the root directory itself
        if entry_path == source_path {
            continue;
        }

        // Skip directories
        if entry_path.is_dir() {
            continue;
        }

        expected_files += 1;

        // Get relative path from source
        let relative = match entry_path.strip_prefix(source_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Build destination file path
        let dest_file = dest_base.join(relative);
        let dest_file_str = dest_file.to_string_lossy().to_string();

        // Check if file exists
        if !dest_file.exists() {
            found_missing += 1;
            if show_diff {
                results.push(CheckResult::fail(
                    &dest_file_str,
                    "Files",
                    "File not found in directory",
                ));
            }
            continue;
        }

        // Check hash if we have state
        if let Some(state) = state_manager.get(&dest_file) {
            match compute_file_hash(&dest_file) {
                Ok(current_hash) => {
                    if current_hash != state.installed_hash {
                        found_modified += 1;
                        if show_diff {
                            results.push(CheckResult::warn(
                                &dest_file_str,
                                "Files",
                                "Modified since install",
                            ));
                        }
                    } else {
                        found_ok += 1;
                    }
                }
                Err(_) => {
                    found_modified += 1;
                    if show_diff {
                        results.push(CheckResult::warn(
                            &dest_file_str,
                            "Files",
                            "Cannot compute hash",
                        ));
                    }
                }
            }
        } else {
            // No state, just check existence
            found_ok += 1;
        }
    }

    // Add summary result for the directory
    if found_missing > 0 || found_modified > 0 {
        let message = format!(
            "{} files: {} OK, {} modified, {} missing",
            expected_files, found_ok, found_modified, found_missing
        );
        if found_missing > 0 {
            results.push(CheckResult::fail(
                format!("{}/ (directory)", file.destination),
                "Files",
                message,
            ));
        } else {
            results.push(CheckResult::warn(
                format!("{}/ (directory)", file.destination),
                "Files",
                message,
            ));
        }
    } else {
        results.push(CheckResult::ok_with_message(
            format!("{}/ (directory)", file.destination),
            "Files",
            format!("{} files OK", expected_files),
        ));
    }

    results
}

/// Check health status of a single file
fn check_file_health(
    dest_path: &str,
    file: &crate::config::workload::FileEntry,
    state_manager: &FileStateManager,
    show_diff: bool,
) -> CheckResult {
    use std::path::Path;

    let path = Path::new(dest_path);

    // Check if file exists
    if !path.exists() {
        return CheckResult::fail(&file.destination, "Files", "File not found");
    }

    // Get file metadata
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return CheckResult::fail(
                &file.destination,
                "Files",
                format!("Cannot read file: {}", e),
            );
        }
    };

    let size = metadata.len();

    // Check hash if we have state
    if let Some(state) = state_manager.get(path) {
        // Compute current hash
        let current_hash = match compute_file_hash(path) {
            Ok(h) => h,
            Err(_) => {
                return CheckResult::warn(
                    &file.destination,
                    "Files",
                    format!("Cannot compute hash ({})", format_file_size(size)),
                );
            }
        };

        if current_hash != state.installed_hash {
            let message = if show_diff {
                format!(
                    "Modified since install\n  Expected: {}\n  Current:  {}",
                    &state.installed_hash[..12.min(state.installed_hash.len())],
                    &current_hash[..12.min(current_hash.len())]
                )
            } else {
                "Modified since install".to_string()
            };
            return CheckResult::warn(&file.destination, "Files", message);
        }

        CheckResult::ok_with_message(
            &file.destination,
            "Files",
            format!("Hash OK ({})", format_file_size(size)),
        )
    } else {
        // No state, just report existence
        CheckResult::ok_with_message(
            &file.destination,
            "Files",
            format!("Exists ({})", format_file_size(size)),
        )
    }
}

/// Format file size for display
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

/// Generate and output the health report, then exit with appropriate code
fn report_and_exit(
    args: &HealthArgs,
    cli: &Cli,
    workload_name: &str,
    checks: Vec<CheckResult>,
) -> Result<()> {
    let use_color = !cli.should_disable_color();
    let report = HealthReport::new(workload_name, checks);

    // Get the formatter based on output format
    let formatter = get_formatter(args.output, use_color);

    // Determine output destination
    if let Some(ref file_path) = args.file {
        let mut file = std::fs::File::create(file_path)
            .with_context(|| format!("Failed to create output file: {}", file_path.display()))?;
        formatter.format_health_report(&report, &mut file)?;
        print_info(&format!("Report written to: {}", file_path.display()));
    } else {
        let mut stdout = std::io::stdout();
        formatter.format_health_report(&report, &mut stdout)?;
    }

    // Print actionable recommendations (only for table format, not JSON/YAML)
    if args.output == crate::cli::commands::OutputFormat::Table {
        print_recommendations(&report);
    }

    // Determine exit code
    let has_failures = report.summary.failed > 0;
    let has_warnings = report.summary.warnings > 0;

    if has_failures || (args.strict && has_warnings) {
        std::process::exit(1);
    }

    Ok(())
}

/// Print recommendations based on health check results
fn print_recommendations(report: &HealthReport) {
    use colored::Colorize;

    let failed_packages: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail && c.category == "Packages")
        .collect();

    let failed_files: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail && c.category == "Files")
        .collect();

    let failed_scripts: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail && c.category == "Scripts")
        .collect();

    let updatable_packages: Vec<_> = report
        .checks
        .iter()
        .filter(|c| {
            c.status == CheckStatus::Warn
                && c.category == "Packages"
                && c.message
                    .as_ref()
                    .map(|m| m.contains("update available"))
                    .unwrap_or(false)
        })
        .collect();

    if failed_packages.is_empty()
        && failed_files.is_empty()
        && failed_scripts.is_empty()
        && updatable_packages.is_empty()
    {
        return;
    }

    println!();
    println!("{}", "Recommendations:".bold());

    // Package recommendations
    if !failed_packages.is_empty() {
        println!();
        println!("  {} Missing packages:", "●".red());
        for pkg in &failed_packages {
            println!("    - {}", pkg.name);
        }
        println!();
        print_warning(&format!(
            "Run 'anvil install {}' to install missing packages",
            report.workload
        ));
    }

    // Update recommendations
    if !updatable_packages.is_empty() {
        println!();
        println!("  {} Packages with updates:", "●".yellow());
        for pkg in &updatable_packages {
            println!("    - {}", pkg.name);
        }
        println!();
        print_warning(&format!(
            "Run 'anvil install {} --upgrade' to update packages",
            report.workload
        ));
    }

    // File recommendations
    if !failed_files.is_empty() {
        println!();
        println!("  {} Missing or incorrect files:", "●".red());
        for file in &failed_files {
            println!("    - {}", file.name);
        }
        println!();
        print_warning(&format!(
            "Run 'anvil install {} --files-only' to restore files",
            report.workload
        ));
    }

    // Script recommendations
    if !failed_scripts.is_empty() {
        println!();
        println!("  {} Failed health checks:", "●".red());
        for script in &failed_scripts {
            if let Some(ref msg) = script.message {
                println!("    - {}: {}", script.name, msg);
            } else {
                println!("    - {}", script.name);
            }
            // Show failure details if available
            if let Some(ref details) = script.details {
                for detail in details {
                    println!("      {}", detail.dimmed());
                }
            }
        }
        println!();
        print_warning("Review the health check output above for details");
    }
}
