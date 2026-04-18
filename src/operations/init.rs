//! Init operation - Create new workload templates
//!
//! This module implements the `anvil init` command which creates
//! a new workload directory with template files.

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, trace};

use crate::cli::commands::{InitArgs, WorkloadTemplate};
use crate::cli::Cli;
use crate::config::schema::is_valid_workload_name;
use crate::config::ConfigManager;

/// Execute the init command
pub fn execute(args: &InitArgs, cli: &Cli) -> Result<()> {
    debug!("Executing init command for workload: {}", args.name);
    trace!("Init arguments: {:?}", args);

    let use_color = !cli.should_disable_color();

    // Validate workload name
    if !is_valid_workload_name(&args.name) {
        if use_color {
            eprintln!(
                "{} Invalid workload name: '{}'",
                "✗".red().bold(),
                args.name.yellow()
            );
            eprintln!(
                "  {} Workload names must start with a lowercase letter",
                "→".dimmed()
            );
            eprintln!(
                "  {} Only lowercase letters, numbers, and hyphens are allowed",
                "→".dimmed()
            );
            eprintln!("  {} Example: {}", "→".dimmed(), "my-workload-123".green());
        } else {
            eprintln!("Error: Invalid workload name: '{}'", args.name);
            eprintln!("  Workload names must start with a lowercase letter");
            eprintln!("  Only lowercase letters, numbers, and hyphens are allowed");
            eprintln!("  Example: my-workload-123");
        }
        std::process::exit(1);
    }

    // Validate extends if provided
    if let Some(ref extends) = args.extends {
        let manager = ConfigManager::new();
        if manager.find_workload(extends).is_none() {
            if use_color {
                eprintln!(
                    "{} Parent workload '{}' not found",
                    "⚠".yellow(),
                    extends.cyan()
                );
                eprintln!(
                    "  {} The workload will be created, but validation may fail",
                    "→".dimmed()
                );
            } else {
                eprintln!("Warning: Parent workload '{}' not found", extends);
                eprintln!("  The workload will be created, but validation may fail");
            }
        } else {
            info!("Parent workload '{}' found", extends);
        }
    }

    let output_dir = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("workloads").join(&args.name));

    if output_dir.exists() {
        if use_color {
            eprintln!(
                "{} Directory already exists: {}",
                "✗".red().bold(),
                output_dir.display().to_string().yellow()
            );
            eprintln!(
                "  {} Use {} to specify a different location",
                "→".dimmed(),
                "--output".cyan()
            );
        } else {
            eprintln!("Error: Directory already exists: {}", output_dir.display());
            eprintln!("  Use --output to specify a different location");
        }
        std::process::exit(1);
    }

    if cli.verbose > 0 {
        if use_color {
            println!(
                "{} Creating workload '{}' at {}",
                "ℹ".blue(),
                args.name.green(),
                output_dir.display().to_string().cyan()
            );
            println!(
                "  {} Template: {}",
                "→".dimmed(),
                format!("{}", args.template).yellow()
            );
            if let Some(ref extends) = args.extends {
                println!("  {} Extends: {}", "→".dimmed(), extends.yellow());
            }
        } else {
            println!(
                "Creating workload '{}' at {}",
                args.name,
                output_dir.display()
            );
            println!("  Template: {}", args.template);
            if let Some(ref extends) = args.extends {
                println!("  Extends: {}", extends);
            }
        }
    }

    // Create directory structure
    debug!("Creating directory structure");
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;

    fs::create_dir_all(output_dir.join("files")).context("Failed to create files directory")?;

    fs::create_dir_all(output_dir.join("scripts")).context("Failed to create scripts directory")?;

    // Generate workload.yaml content based on template
    let workload_content =
        generate_workload_yaml(&args.name, &args.template, args.extends.as_deref());

    // Write workload.yaml
    debug!("Writing workload.yaml");
    fs::write(output_dir.join("workload.yaml"), workload_content)
        .context("Failed to write workload.yaml")?;

    // Generate template scripts based on template type
    if matches!(
        args.template,
        WorkloadTemplate::Standard | WorkloadTemplate::Full
    ) {
        debug!("Creating post-install script");
        let post_install = generate_post_install_script(&args.name);
        fs::write(
            output_dir.join("scripts").join("post-install.ps1"),
            post_install,
        )
        .context("Failed to write post-install.ps1")?;
    }

    // Create a pre-install script for full template
    if matches!(args.template, WorkloadTemplate::Full) {
        debug!("Creating pre-install script");
        let pre_install = generate_pre_install_script(&args.name);
        fs::write(
            output_dir.join("scripts").join("pre-install.ps1"),
            pre_install,
        )
        .context("Failed to write pre-install.ps1")?;

        // Create a sample config file
        debug!("Creating sample config file");
        let sample_config = generate_sample_config(&args.name);
        fs::write(output_dir.join("files").join("config.toml"), sample_config)
            .context("Failed to write sample config.toml")?;
    }

    // Create a .gitkeep in files directory for minimal template
    if matches!(args.template, WorkloadTemplate::Minimal) {
        fs::write(
            output_dir.join("files").join(".gitkeep"),
            "# Add configuration files here\n",
        )
        .context("Failed to write .gitkeep")?;

        fs::write(
            output_dir.join("scripts").join(".gitkeep"),
            "# Add scripts here\n",
        )
        .context("Failed to write .gitkeep")?;
    }

    // Print success message
    println!();
    if use_color {
        println!(
            "{} Created workload '{}' at {}",
            "✓".green().bold(),
            args.name.green().bold(),
            output_dir.display().to_string().cyan()
        );
    } else {
        println!(
            "Created workload '{}' at {}",
            args.name,
            output_dir.display()
        );
    }

    // Print next steps
    println!();
    if use_color {
        println!("{}", "Next steps:".bold());
        println!(
            "  {} Edit {} to define packages and files",
            "1.".dimmed(),
            format!("{}/workload.yaml", output_dir.display()).cyan()
        );
        println!(
            "  {} Add configuration files to {}",
            "2.".dimmed(),
            format!("{}/files/", output_dir.display()).cyan()
        );
        println!(
            "  {} Customize scripts in {}",
            "3.".dimmed(),
            format!("{}/scripts/", output_dir.display()).cyan()
        );
        println!(
            "  {} Validate with: {}",
            "4.".dimmed(),
            format!("anvil validate {}", output_dir.display()).yellow()
        );
        println!(
            "  {} Test with: {}",
            "5.".dimmed(),
            format!("anvil install {} --dry-run", args.name).yellow()
        );
    } else {
        println!("Next steps:");
        println!(
            "  1. Edit {}/workload.yaml to define packages and files",
            output_dir.display()
        );
        println!(
            "  2. Add configuration files to {}/files/",
            output_dir.display()
        );
        println!(
            "  3. Customize scripts in {}/scripts/",
            output_dir.display()
        );
        println!(
            "  4. Validate with: anvil validate {}",
            output_dir.display()
        );
        println!("  5. Test with: anvil install {} --dry-run", args.name);
    }

    Ok(())
}

/// Generate workload.yaml content based on template type
fn generate_workload_yaml(
    name: &str,
    template: &WorkloadTemplate,
    extends: Option<&str>,
) -> String {
    match template {
        WorkloadTemplate::Minimal => generate_minimal_yaml(name, extends),
        WorkloadTemplate::Standard => generate_standard_yaml(name, extends),
        WorkloadTemplate::Full => generate_full_yaml(name, extends),
    }
}

fn generate_minimal_yaml(name: &str, extends: Option<&str>) -> String {
    let extends_section = extends
        .map(|e| format!("\n# Inherit from parent workload\nextends:\n  - {}\n", e))
        .unwrap_or_default();

    format!(
        r#"# {name} Workload
# Minimal workload template - add packages, files, and scripts as needed
#
# Documentation: https://github.com/anvil/anvil#workload-schema

name: {name}
version: "1.0.0"
description: "TODO: Add description for {name}"
{extends_section}
# Uncomment and customize the sections below:
#
# packages:
#   winget:
#     - id: Example.Package
#
# files:
#   - source: config.toml
#     destination: "~/.config/app/config.toml"
#
# commands:
#   post_install:
#     - run: echo "Setup complete"
#       description: "Post-install setup"
"#,
        name = name,
        extends_section = extends_section.trim_start()
    )
}

fn generate_standard_yaml(name: &str, extends: Option<&str>) -> String {
    let extends_section = extends
        .map(|e| format!("\n# Inherit from parent workload\nextends:\n  - {}\n", e))
        .unwrap_or_default();

    format!(
        r#"# {name} Workload
# Standard workload template with common sections
#
# Documentation: https://github.com/anvil/anvil#workload-schema
# Validate with: anvil validate workloads/{name}

name: {name}
version: "1.0.0"
description: "TODO: Add description for {name}"
{extends_section}
packages:
  winget:
    # Add packages to install via winget
    # Find package IDs with: winget search <name>
    #
    # Example packages:
    # - id: Git.Git
    # - id: Microsoft.VisualStudioCode
    #   version: "1.85.0"  # Pin to specific version (optional)

files:
  # Add configuration files to deploy
  # Source paths are relative to the files/ directory
  # Destination paths support ~ (home) and ${{VAR}} expansion
  #
  # Example:
  # - source: config.toml
  #   destination: "~/.config/app/config.toml"
  #   backup: true  # Backup existing file before overwriting

commands:
  # Post-installation commands - runs after packages are installed
  post_install:
    - run: echo "Configure {name} environment"
      description: "Configure {name} environment"
      timeout: 300

# Optional: Health check settings
health:
  package_check: true   # Verify packages are installed
  file_check: true      # Verify files are deployed
  assertion_check: true # Evaluate declarative assertions

# Optional: Declarative health assertions
# assertions:
#   - name: "Example command exists"
#     check:
#       type: command_exists
#       command: my-tool
"#,
        name = name,
        extends_section = extends_section.trim_start()
    )
}

fn generate_full_yaml(name: &str, extends: Option<&str>) -> String {
    let extends_section = extends
        .map(|e| format!("\n# Inherit from parent workload\nextends:\n  - {}\n", e))
        .unwrap_or_default();

    format!(
        r#"# {name} Workload
# Full workload template with all available options documented
#
# Documentation: https://github.com/anvil/anvil#workload-schema
# Validate with: anvil validate workloads/{name}
# JSON Schema:   anvil validate --schema

# Required: Unique workload identifier
# Rules: lowercase letters, numbers, hyphens only; must start with letter
name: {name}

# Required: Semantic version (MAJOR.MINOR.PATCH)
version: "1.0.0"

# Required: Human-readable description
description: "TODO: Add description for {name}"
{extends_section}
# Packages to install via winget
packages:
  winget:
    # Basic package installation
    - id: Example.Package

    # Package with version pinning (recommended for reproducibility)
    # - id: Microsoft.VisualStudioCode
    #   version: "1.85.0"

    # Package from Microsoft Store
    # - id: 9WZDNCRF0083
    #   source: msstore

    # Package with custom installer arguments
    # - id: Git.Git
    #   override:
    #     - --override
    #     - '/VERYSILENT /NORESTART'

# Files to deploy to the system
files:
  # Basic file deployment
  - source: config.toml                    # Relative to files/ directory
    destination: "~/.config/app/config.toml"  # Supports ~ and ${{VAR}}
    backup: true                           # Backup existing (default: true)

  # Template file - variables will be expanded
  # - source: template.conf
  #   destination: "~/.app/config"
  #   template: true

# Commands to execute at various stages
commands:
  # Pre-installation: Check prerequisites
  pre_install:
    - run: echo "Checking prerequisites for {name}..."
      description: "Check prerequisites for {name}"
      timeout: 60

  # Post-installation: Configure the environment
  post_install:
    - run: echo "Configuring {name} environment..."
      description: "Configure {name} environment"
      timeout: 300

# Environment configuration
environment:
  # Environment variables to set
  variables:
    # User-scope variable (default)
    - name: MY_APP_CONFIG
      value: "~/.config/app"
      scope: user               # user or machine

    # Machine-scope requires admin privileges
    # - name: GLOBAL_SETTING
    #   value: "some-value"
    #   scope: machine

  # Directories to add to PATH
  path_additions:
    - "~/.local/bin"
    # - "${{PROGRAMFILES}}/MyApp/bin"

# Declarative health assertions
# Assertions replace or complement health check scripts with simple checks
assertions:
  - name: Example tool check
    check:
      type: command_exists
      command: git

  # - name: Config directory exists
  #   check:
  #     type: dir_exists
  #     path: "~/.config/app"

  # - name: Environment variable set
  #   check:
  #     type: env_var
  #     name: MY_APP_CONFIG

# Health check configuration
health:
  package_check: true    # Verify all packages are installed
  file_check: true       # Verify all files are deployed correctly
  assertion_check: true  # Evaluate declarative assertions
"#,
        name = name,
        extends_section = extends_section.trim_start()
    )
}

fn generate_pre_install_script(name: &str) -> String {
    format!(
        r#"# pre-install.ps1 - {name} Workload
# This script runs BEFORE packages are installed
# Use it to check prerequisites and prepare the system
#
# Exit codes:
#   0 - Success, continue with installation
#   1 - Failure, abort installation

$ErrorActionPreference = "Stop"

Write-Host "Checking prerequisites for {name}..." -ForegroundColor Cyan

# Example: Check for required disk space
$requiredSpaceGB = 5
$systemDrive = $env:SystemDrive
$freeSpace = (Get-PSDrive $systemDrive[0]).Free / 1GB

if ($freeSpace -lt $requiredSpaceGB) {{
    Write-Host "  [FAIL] Insufficient disk space. Need $requiredSpaceGB GB, have $([math]::Round($freeSpace, 2)) GB" -ForegroundColor Red
    exit 1
}}
Write-Host "  [PASS] Disk space: $([math]::Round($freeSpace, 2)) GB available" -ForegroundColor Green

# Example: Check for required Windows version
$osVersion = [Environment]::OSVersion.Version
if ($osVersion.Major -lt 10) {{
    Write-Host "  [FAIL] Windows 10 or later required" -ForegroundColor Red
    exit 1
}}
Write-Host "  [PASS] Windows version: $($osVersion.Major).$($osVersion.Minor)" -ForegroundColor Green

# TODO: Add your prerequisite checks here
# Examples:
# - Check for specific software
# - Verify network connectivity
# - Check user permissions

Write-Host ""
Write-Host "All prerequisites met!" -ForegroundColor Green
exit 0
"#,
        name = name
    )
}

fn generate_post_install_script(name: &str) -> String {
    format!(
        r#"# post-install.ps1 - {name} Workload
# This script runs AFTER packages are installed
# Use it to configure applications and set up the environment
#
# Exit codes:
#   0 - Success
#   1 - Failure (installation will be marked as failed)

$ErrorActionPreference = "Stop"

Write-Host "Configuring {name} environment..." -ForegroundColor Cyan

# TODO: Add your post-installation configuration here
# Examples:

# Create configuration directories
# $configDir = Join-Path $env:USERPROFILE ".config/myapp"
# if (-not (Test-Path $configDir)) {{
#     New-Item -ItemType Directory -Path $configDir -Force | Out-Null
#     Write-Host "  Created configuration directory: $configDir" -ForegroundColor Gray
# }}

# Configure Git (if installed)
# if (Get-Command git -ErrorAction SilentlyContinue) {{
#     Write-Host "  Configuring Git..." -ForegroundColor Gray
#     # git config --global core.autocrlf true
#     # git config --global init.defaultBranch main
# }}

# Install additional tools
# Write-Host "  Installing additional tools..." -ForegroundColor Gray
# cargo install ripgrep bat fd-find

# Refresh environment variables
# $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "User") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "Machine")

Write-Host ""
Write-Host "Configuration complete!" -ForegroundColor Green
exit 0
"#,
        name = name
    )
}

fn generate_sample_config(name: &str) -> String {
    format!(
        r#"# Sample configuration file for {name}
# This file will be deployed to the destination specified in workload.yaml
#
# Customize this file or replace it with your actual configuration

[general]
# Application name
name = "{name}"

# Enable debug mode
debug = false

# Log level: trace, debug, info, warn, error
log_level = "info"

[paths]
# Data directory (supports environment variables)
# data_dir = "$HOME/.local/share/{name}"

# Cache directory
# cache_dir = "$HOME/.cache/{name}"

[features]
# Enable experimental features
# experimental = false

# Example feature flags
# feature_a = true
# feature_b = false
"#,
        name = name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_minimal_yaml() {
        let yaml = generate_minimal_yaml("test-workload", None);
        assert!(yaml.contains("name: test-workload"));
        assert!(yaml.contains("version: \"1.0.0\""));
        assert!(!yaml.contains("extends:"));
    }

    #[test]
    fn test_generate_minimal_yaml_with_extends() {
        let yaml = generate_minimal_yaml("test-workload", Some("base-workload"));
        assert!(yaml.contains("name: test-workload"));
        assert!(yaml.contains("extends:"));
        assert!(yaml.contains("- base-workload"));
    }

    #[test]
    fn test_generate_standard_yaml() {
        let yaml = generate_standard_yaml("my-app", None);
        assert!(yaml.contains("name: my-app"));
        assert!(yaml.contains("packages:"));
        assert!(yaml.contains("files:"));
        assert!(yaml.contains("commands:"));
        assert!(yaml.contains("health:"));
    }

    #[test]
    fn test_generate_full_yaml() {
        let yaml = generate_full_yaml("full-app", Some("parent"));
        assert!(yaml.contains("name: full-app"));
        assert!(yaml.contains("extends:"));
        assert!(yaml.contains("- parent"));
        assert!(yaml.contains("packages:"));
        assert!(yaml.contains("files:"));
        assert!(yaml.contains("commands:"));
        assert!(yaml.contains("environment:"));
        assert!(yaml.contains("health:"));
    }

    #[test]
    fn test_generate_post_install_script() {
        let script = generate_post_install_script("test");
        assert!(script.contains("post-install.ps1"));
        assert!(script.contains("test"));
        assert!(script.contains("exit 0"));
    }

    #[test]
    fn test_generate_pre_install_script() {
        let script = generate_pre_install_script("test");
        assert!(script.contains("pre-install.ps1"));
        assert!(script.contains("prerequisites"));
        assert!(script.contains("exit 0"));
        assert!(script.contains("exit 1"));
    }

    #[test]
    fn test_generate_sample_config() {
        let config = generate_sample_config("my-app");
        assert!(config.contains("my-app"));
        assert!(config.contains("[general]"));
        assert!(config.contains("[paths]"));
    }
}
