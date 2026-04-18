//! Common test utilities for Anvil integration tests
//!
//! This module provides helper functions for creating test workloads
//! and managing test fixtures.

use std::fs;
use std::path::Path;

/// Create a minimal valid workload for testing
///
/// # Arguments
/// * `dir` - Parent directory where the workload will be created
/// * `name` - Name of the workload
///
/// # Returns
/// Result indicating success or IO error
pub fn create_test_workload(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    fs::create_dir_all(workload_dir.join("scripts"))?;
    fs::create_dir_all(workload_dir.join("files"))?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Test workload for integration tests"

packages:
  winget:
    - id: Microsoft.WindowsTerminal

files: []
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;

    Ok(())
}

/// Create a workload with inheritance for testing
///
/// # Arguments
/// * `dir` - Parent directory where the workload will be created
/// * `name` - Name of the child workload
/// * `parent` - Name of the parent workload to extend
///
/// # Returns
/// Result indicating success or IO error
pub fn create_inherited_workload(dir: &Path, name: &str, parent: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    fs::create_dir_all(&workload_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Child workload extending {parent}"

extends:
  - {parent}
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;
    Ok(())
}

/// Create an invalid workload for testing error handling
///
/// # Arguments
/// * `dir` - Parent directory where the workload will be created
/// * `name` - Name of the workload
///
/// # Returns
/// Result indicating success or IO error
pub fn create_invalid_workload(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    fs::create_dir_all(&workload_dir)?;

    // Invalid: missing required 'name' field
    fs::write(
        workload_dir.join("workload.yaml"),
        r#"version: "1.0.0"
invalid_field: true
description: "This workload is missing the required name field"
"#,
    )?;

    Ok(())
}

/// Create a workload with circular dependency for testing
///
/// Creates two workloads that extend each other, which should cause
/// a circular dependency error.
///
/// # Arguments
/// * `dir` - Parent directory where the workloads will be created
///
/// # Returns
/// Result indicating success or IO error
#[allow(dead_code)]
pub fn create_circular_workloads(dir: &Path) -> std::io::Result<()> {
    // Create workload A that extends B
    let workload_a_dir = dir.join("circular-a");
    fs::create_dir_all(&workload_a_dir)?;
    fs::write(
        workload_a_dir.join("workload.yaml"),
        r#"name: circular-a
version: "1.0.0"
description: "Workload A in circular dependency"
extends:
  - circular-b
"#,
    )?;

    // Create workload B that extends A
    let workload_b_dir = dir.join("circular-b");
    fs::create_dir_all(&workload_b_dir)?;
    fs::write(
        workload_b_dir.join("workload.yaml"),
        r#"name: circular-b
version: "1.0.0"
description: "Workload B in circular dependency"
extends:
  - circular-a
"#,
    )?;

    Ok(())
}

/// Create a full-featured workload for comprehensive testing
///
/// # Arguments
/// * `dir` - Parent directory where the workload will be created
/// * `name` - Name of the workload
///
/// # Returns
/// Result indicating success or IO error
pub fn create_full_workload(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    let files_dir = workload_dir.join("files");

    fs::create_dir_all(&files_dir)?;

    // Create workload.yaml
    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Full-featured test workload with all sections"

packages:
  winget:
    - id: Microsoft.WindowsTerminal
    - id: Git.Git
      version: "2.43.0"

files:
  - source: files/config.json
    destination: "~/.config/test/config.json"
    backup: true
    template: false

commands:
  pre_install:
    - run: echo "Pre-installation checks"
      description: "Pre-installation checks"
      timeout: 60
  post_install:
    - run: echo "Post-installation configuration"
      description: "Post-installation configuration"
      timeout: 120

environment:
  variables:
    - name: TEST_VAR
      value: "test_value"
      scope: user

  path_additions:
    - "~/.local/bin"
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;

    // Create config file
    fs::write(
        files_dir.join("config.json"),
        r#"{
  "setting1": "value1",
  "setting2": "value2"
}
"#,
    )?;

    Ok(())
}

/// Create a workload with a directory in files section for testing
///
/// # Arguments
/// * `dir` - Parent directory where the workload will be created
/// * `name` - Name of the workload
///
/// # Returns
/// Result indicating success or IO error
pub fn create_workload_with_directory(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    let files_dir = workload_dir.join("files");
    let config_dir = files_dir.join("config");
    let nested_dir = config_dir.join("nested");

    fs::create_dir_all(&nested_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Workload with directory files"

files:
  - source: config
    destination: "~/.config/test-app"
    backup: true
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;

    // Create files in the config directory
    fs::write(config_dir.join("settings.json"), r#"{"key": "value"}"#)?;
    fs::write(config_dir.join("options.toml"), "[options]\nvalue = true")?;
    fs::write(nested_dir.join("deep.txt"), "nested file content")?;

    Ok(())
}

/// Create a workload with template files for testing
///
/// # Arguments
/// * `dir` - Parent directory where the workload will be created
/// * `name` - Name of the workload
///
/// # Returns
/// Result indicating success or IO error
#[allow(dead_code)]
pub fn create_template_workload(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    let files_dir = workload_dir.join("files");

    fs::create_dir_all(&files_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Workload with template files"

files:
  - source: files/config.toml.hbs
    destination: "~/.config/test/config.toml"
    template: true
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;

    // Create template file
    fs::write(
        files_dir.join("config.toml.hbs"),
        r#"# Configuration for {{username}}
# Generated by Anvil

[user]
name = "{{username}}"
home = "{{home}}"

[workload]
name = "{{workload_name}}"
dir = "{{workload_dir}}"
"#,
    )?;

    Ok(())
}

/// Create a workload with declarative assertions for testing
///
/// # Arguments
/// * `dir` - Parent directory where the workload will be created
/// * `name` - Name of the workload
///
/// # Returns
/// Result indicating success or IO error
#[allow(dead_code)]
pub fn create_workload_with_assertions(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    fs::create_dir_all(&workload_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Workload with assertions for testing"

assertions:
  - name: "PATH is set"
    check:
      type: env_var
      name: PATH
  - name: "missing var should fail"
    check:
      type: env_var
      name: ANVIL_TEST_NONEXISTENT_VAR_XYZ_12345
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;
    Ok(())
}

/// Create a workload with assertions disabled via health config
#[allow(dead_code)]
pub fn create_workload_assertions_disabled(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    fs::create_dir_all(&workload_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Workload with assertions disabled"

health:
  assertion_check: false

assertions:
  - name: "should be skipped"
    check:
      type: env_var
      name: ANVIL_TEST_NONEXISTENT_VAR_XYZ_12345
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;
    Ok(())
}

/// Create a workload with only passing assertions
#[allow(dead_code)]
pub fn create_workload_passing_assertions(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    fs::create_dir_all(&workload_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Workload with only passing assertions"

assertions:
  - name: "PATH is set"
    check:
      type: env_var
      name: PATH
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;
    Ok(())
}

/// Create a workload with the removed health_check field
/// for testing that validation correctly rejects it
#[allow(dead_code)]
pub fn create_workload_with_both_assertions_and_scripts(
    dir: &Path,
    name: &str,
) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    let scripts_dir = workload_dir.join("scripts");
    fs::create_dir_all(&scripts_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Workload with removed health_check field"

assertions:
  - name: "PATH is set"
    check:
      type: env_var
      name: PATH

scripts:
  health_check:
    - path: scripts/health.ps1
      name: "Legacy Check"
      description: "Legacy health check script"
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;
    fs::write(
        scripts_dir.join("health.ps1"),
        r#"# Legacy health check script
Write-Host "Legacy health check running..."
exit 0
"#,
    )?;

    Ok(())
}

/// Create a workload with the removed scripts fields for testing
/// that validation correctly rejects them
#[allow(dead_code)]
pub fn create_workload_with_commands_and_scripts(dir: &Path, name: &str) -> std::io::Result<()> {
    let workload_dir = dir.join(name);
    fs::create_dir_all(&workload_dir)?;

    let yaml = format!(
        r#"name: {name}
version: "1.0.0"
description: "Workload with removed scripts fields"

commands:
  pre_install:
    - run: echo pre-install command
      description: "Pre-install command"
  post_install:
    - run: echo post-install command
      description: "Post-install command"

scripts:
  pre_install:
    - path: pre-install.ps1
      description: "Legacy pre-install script"
  post_install:
    - path: post-install.ps1
      description: "Legacy post-install script"
"#
    );

    fs::write(workload_dir.join("workload.yaml"), yaml)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_test_workload() {
        let temp = TempDir::new().unwrap();
        create_test_workload(temp.path(), "test-workload").unwrap();

        assert!(temp.path().join("test-workload/workload.yaml").exists());
    }

    #[test]
    fn test_create_inherited_workload() {
        let temp = TempDir::new().unwrap();
        create_inherited_workload(temp.path(), "child", "parent").unwrap();

        let content = fs::read_to_string(temp.path().join("child/workload.yaml")).unwrap();
        assert!(content.contains("extends:"));
        assert!(content.contains("parent"));
    }

    #[test]
    fn test_create_invalid_workload() {
        let temp = TempDir::new().unwrap();
        create_invalid_workload(temp.path(), "invalid").unwrap();

        let content = fs::read_to_string(temp.path().join("invalid/workload.yaml")).unwrap();
        assert!(!content.contains("name:"));
    }

    #[test]
    fn test_create_full_workload() {
        let temp = TempDir::new().unwrap();
        create_full_workload(temp.path(), "full-test").unwrap();

        let workload_dir = temp.path().join("full-test");
        assert!(workload_dir.join("workload.yaml").exists());
        assert!(workload_dir.join("files/config.json").exists());
    }

    #[test]
    fn test_create_workload_with_directory() {
        let temp = TempDir::new().unwrap();
        create_workload_with_directory(temp.path(), "dir-test").unwrap();

        let workload_dir = temp.path().join("dir-test");
        assert!(workload_dir.join("workload.yaml").exists());
        assert!(workload_dir.join("files/config").is_dir());
        assert!(workload_dir.join("files/config/settings.json").exists());
        assert!(workload_dir.join("files/config/options.toml").exists());
        assert!(workload_dir.join("files/config/nested/deep.txt").exists());
    }
}
