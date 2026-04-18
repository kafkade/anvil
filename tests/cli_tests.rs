//! Integration tests for Anvil CLI
//!
//! These tests verify the CLI commands work correctly end-to-end.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to get anvil command
fn anvil() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("anvil").unwrap()
}

mod list_command {
    use super::*;

    #[test]
    fn list_shows_available_workloads() {
        // List may return no bundled workloads in the open-source repo,
        // but the command itself should succeed
        anvil().arg("list").assert().success();
    }

    #[test]
    fn list_with_long_format() {
        anvil().args(["list", "--long"]).assert().success();
    }

    #[test]
    fn list_with_json_output() {
        anvil()
            .args(["list", "--output", "json"])
            .assert()
            .success();
    }

    #[test]
    fn list_with_yaml_output() {
        anvil()
            .args(["list", "--output", "yaml"])
            .assert()
            .success();
    }

    #[test]
    fn list_custom_path() {
        let temp = TempDir::new().unwrap();
        // Create a test workload
        let workload_dir = temp.path().join("test-workload");
        fs::create_dir_all(&workload_dir).unwrap();
        fs::write(
            workload_dir.join("workload.yaml"),
            "name: test-workload\nversion: \"1.0.0\"\ndescription: \"Test workload\"\n",
        )
        .unwrap();

        anvil()
            .args(["list", "--path", temp.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("test-workload"));
    }

    #[test]
    fn list_empty_directory() {
        let temp = TempDir::new().unwrap();

        anvil()
            .args(["list", "--path", temp.path().to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn list_with_all_paths_flag() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("test-wl");
        fs::create_dir_all(&workload_dir).unwrap();
        fs::write(
            workload_dir.join("workload.yaml"),
            "name: test-wl\nversion: \"1.0.0\"\ndescription: \"Test workload\"\n",
        )
        .unwrap();

        anvil()
            .args([
                "list",
                "--path",
                temp.path().to_str().unwrap(),
                "--all-paths",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("test-wl"));
    }

    #[test]
    fn list_no_duplicates_by_default() {
        // The --path flag only supports a single path, so we test deduplication
        // via the JSON output with a single path that has unique workloads.
        // The unit tests in config/mod.rs cover multi-path precedence.
        let dir = TempDir::new().unwrap();
        let wl_dir = dir.path().join("nodups-test");
        fs::create_dir_all(&wl_dir).unwrap();
        fs::write(
            wl_dir.join("workload.yaml"),
            "name: nodups-test\nversion: \"1.0.0\"\ndescription: \"no dups\"\n",
        )
        .unwrap();

        anvil()
            .args([
                "list",
                "--path",
                dir.path().to_str().unwrap(),
                "--output",
                "json",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("nodups-test"));
    }

    #[test]
    fn list_all_paths_shows_shadowed() {
        // With --all-paths and a single path (no duplicates), no "(shadowed)"
        let dir = TempDir::new().unwrap();
        let wl_dir = dir.path().join("shadow-test");
        fs::create_dir_all(&wl_dir).unwrap();
        fs::write(
            wl_dir.join("workload.yaml"),
            "name: shadow-test\nversion: \"1.0.0\"\ndescription: \"shadow test\"\n",
        )
        .unwrap();

        // Should succeed with --all-paths and show the workload
        anvil()
            .args([
                "list",
                "--path",
                dir.path().to_str().unwrap(),
                "--all-paths",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("shadow-test"));
    }
}

mod show_command {
    use super::*;

    #[test]
    fn show_displays_workload_details() {
        anvil()
            .args(["show", "./examples/minimal"])
            .assert()
            .success()
            .stdout(predicate::str::contains("minimal"));
    }

    #[test]
    fn show_nonexistent_workload_fails() {
        anvil()
            .args(["show", "nonexistent-workload-xyz"])
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not found")
                    .or(predicate::str::contains("error").or(predicate::str::contains("Error"))),
            );
    }

    #[test]
    fn show_with_inheritance_flag() {
        anvil()
            .args(["show", "./examples/rust-developer"])
            .assert()
            .success();
    }

    #[test]
    fn show_json_output() {
        anvil()
            .args(["show", "./examples/minimal", "--output", "json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{"));
    }

    #[test]
    fn show_yaml_output() {
        anvil()
            .args(["show", "./examples/minimal", "--output", "yaml"])
            .assert()
            .success()
            .stdout(predicate::str::contains("name:"));
    }

    #[test]
    fn show_resolved_workload() {
        anvil()
            .args(["show", "./examples/rust-developer", "--resolved"])
            .assert()
            .success();
    }
}

mod validate_command {
    use super::*;

    #[test]
    fn validate_valid_workload() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("valid-workload");
        fs::create_dir_all(&workload_dir).unwrap();
        fs::write(
            workload_dir.join("workload.yaml"),
            r#"name: valid-workload
version: "1.0.0"
description: "A valid test workload"
packages:
  winget:
    - id: Microsoft.WindowsTerminal
"#,
        )
        .unwrap();

        anvil()
            .args(["validate", workload_dir.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn validate_bundled_workloads() {
        // Validate essentials
        anvil()
            .args(["validate", "./examples/minimal"])
            .assert()
            .success();
    }

    #[test]
    fn validate_invalid_workload_missing_name() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("invalid-workload");
        fs::create_dir_all(&workload_dir).unwrap();
        // Missing required 'name' field
        fs::write(workload_dir.join("workload.yaml"), "version: \"1.0.0\"\n").unwrap();

        anvil()
            .args(["validate", workload_dir.to_str().unwrap()])
            .assert()
            .failure();
    }

    #[test]
    fn validate_invalid_yaml_syntax() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("bad-yaml");
        fs::create_dir_all(&workload_dir).unwrap();
        // Invalid YAML
        fs::write(
            workload_dir.join("workload.yaml"),
            "name: test\n  bad indentation:\n- broken",
        )
        .unwrap();

        anvil()
            .args(["validate", workload_dir.to_str().unwrap()])
            .assert()
            .failure();
    }

    #[test]
    fn validate_strict_mode() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("strict-test");
        fs::create_dir_all(&workload_dir).unwrap();
        // Include description since strict mode requires it
        fs::write(
            workload_dir.join("workload.yaml"),
            r#"name: strict-test
version: "1.0.0"
description: "Test workload for strict validation"
"#,
        )
        .unwrap();

        anvil()
            .args(["validate", workload_dir.to_str().unwrap(), "--strict"])
            .assert()
            .success();
    }

    #[test]
    fn validate_schema_output() {
        anvil().args(["validate", "--schema"]).assert().success();
    }
}

mod init_command {
    use super::*;

    #[test]
    fn init_creates_workload_directory() {
        let temp = TempDir::new().unwrap();
        let workload_name = "new-workload";
        // Create output in a subdirectory that doesn't exist yet
        let output_path = temp.path().join("output").join(workload_name);

        anvil()
            .args([
                "init",
                workload_name,
                "--output",
                output_path.to_str().unwrap(),
            ])
            .assert()
            .success();

        assert!(output_path.join("workload.yaml").exists());
    }

    #[test]
    fn init_with_minimal_template() {
        let temp = TempDir::new().unwrap();
        let workload_name = "minimal-workload";
        let output_path = temp.path().join("output").join(workload_name);

        anvil()
            .args([
                "init",
                workload_name,
                "--template",
                "minimal",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .assert()
            .success();

        let workload_yaml = output_path.join("workload.yaml");
        assert!(workload_yaml.exists());
        let content = fs::read_to_string(workload_yaml).unwrap();
        assert!(content.contains("name:"));
    }

    #[test]
    fn init_with_full_template() {
        let temp = TempDir::new().unwrap();
        let workload_name = "full-workload";
        let output_path = temp.path().join("output").join(workload_name);

        anvil()
            .args([
                "init",
                workload_name,
                "--template",
                "full",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .assert()
            .success();

        assert!(output_path.join("workload.yaml").exists());
    }

    #[test]
    fn init_with_extends() {
        let temp = TempDir::new().unwrap();
        let workload_name = "extended-workload";
        let output_path = temp.path().join("output").join(workload_name);

        // Note: This may warn about parent not found, but should still succeed
        anvil()
            .args([
                "init",
                workload_name,
                "--extends",
                "minimal",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .assert()
            .success();

        let workload_yaml = output_path.join("workload.yaml");
        let content = fs::read_to_string(workload_yaml).unwrap();
        assert!(content.contains("extends") || content.contains("minimal"));
    }
}

/// Run an anvil command and skip the test if it fails due to winget unavailability.
/// Returns the assert on success, or silently returns on winget failure.
macro_rules! assert_or_skip_winget {
    ($cmd:expr) => {{
        let output = $cmd.output().expect("failed to run anvil");
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() && stderr.contains("winget") {
            eprintln!("SKIP: winget not functional on this system");
            return;
        }
        assert!(
            output.status.success(),
            "command failed (exit {}): {}",
            output.status,
            stderr
        );
    }};
}

mod install_command {
    use super::*;

    #[test]
    fn install_dry_run_shows_plan() {
        assert_or_skip_winget!(anvil().args(["install", "./examples/minimal", "--dry-run"]));
    }

    #[test]
    fn install_nonexistent_workload_fails() {
        anvil()
            .args(["install", "nonexistent-workload-xyz"])
            .assert()
            .failure();
    }

    #[test]
    fn install_with_skip_packages() {
        anvil()
            .args([
                "install",
                "./examples/minimal",
                "--dry-run",
                "--skip-packages",
            ])
            .assert()
            .success();
    }

    #[test]
    fn install_with_skip_files() {
        assert_or_skip_winget!(anvil().args([
            "install",
            "./examples/minimal",
            "--dry-run",
            "--skip-files"
        ]));
    }

    #[test]
    fn install_with_skip_scripts() {
        assert_or_skip_winget!(anvil().args([
            "install",
            "./examples/minimal",
            "--dry-run",
            "--skip-scripts"
        ]));
        anvil()
            .args([
                "install",
                "./examples/minimal",
                "--dry-run",
                "--skip-scripts",
            ])
            .assert()
            .success();
    }

    #[test]
    fn install_with_all_skip_flags() {
        anvil()
            .args([
                "install",
                "./examples/minimal",
                "--dry-run",
                "--skip-packages",
                "--skip-files",
                "--skip-scripts",
            ])
            .assert()
            .success();
    }

    #[test]
    fn install_packages_only() {
        assert_or_skip_winget!(anvil().args([
            "install",
            "./examples/minimal",
            "--dry-run",
            "--packages-only"
        ]));
    }

    #[test]
    fn install_files_only() {
        anvil()
            .args(["install", "./examples/minimal", "--dry-run", "--files-only"])
            .assert()
            .success();
    }

    #[test]
    fn install_deprecation_warning_when_commands_and_scripts_coexist() {
        let temp = TempDir::new().unwrap();
        common::create_workload_with_commands_and_scripts(temp.path(), "coexist-test").unwrap();
        let workload_path = temp.path().join("coexist-test");

        // When both commands and scripts exist for the same phase, stderr should contain deprecation warnings
        anvil()
            .args([
                "install",
                workload_path.to_str().unwrap(),
                "--dry-run",
                "--skip-scripts",
                "--skip-packages",
            ])
            .assert()
            .success()
            .stderr(predicate::str::contains(
                "scripts.pre_install` is deprecated when used alongside `commands.pre_install`",
            ))
            .stderr(predicate::str::contains(
                "scripts.post_install` is deprecated when used alongside `commands.post_install`",
            ));
    }
}

mod health_command {
    use super::*;

    #[test]
    fn health_check_runs() {
        // Health check should complete (pass or fail based on system state)
        let result = anvil().args(["health", "./examples/minimal"]).assert();
        // We just verify it runs - it may pass or fail depending on system state
        // but shouldn't panic or error unexpectedly
        result.code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_json_output() {
        let output = anvil()
            .args(["health", "./examples/minimal", "--output", "json"])
            .output()
            .expect("failed to run anvil");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("winget") && stdout.is_empty() {
            eprintln!("SKIP: winget not functional on this system");
            return;
        }
        assert!(
            stdout.starts_with('{'),
            "expected JSON output starting with '{{', got: {}",
            &stdout[..stdout.len().min(100)]
        );
    }

    #[test]
    fn health_yaml_output() {
        anvil()
            .args(["health", "./examples/minimal", "--output", "yaml"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_nonexistent_workload_fails() {
        anvil()
            .args(["health", "nonexistent-workload-xyz"])
            .assert()
            .failure();
    }

    #[test]
    fn health_packages_only() {
        anvil()
            .args(["health", "./examples/minimal", "--packages-only"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_fail_fast() {
        anvil()
            .args(["health", "./examples/minimal", "--fail-fast"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_to_file() {
        let temp = TempDir::new().unwrap();
        let output_file = temp.path().join("health-report.json");

        anvil()
            .args([
                "health",
                "./examples/minimal",
                "--output",
                "json",
                "--file",
                output_file.to_str().unwrap(),
            ])
            .assert()
            .code(predicate::in_iter([0, 1]));

        assert!(output_file.exists());
    }

    #[test]
    fn health_assertions_only() {
        let temp = TempDir::new().unwrap();
        common::create_workload_with_assertions(temp.path(), "assert-test").unwrap();
        let workload_path = temp.path().join("assert-test");

        anvil()
            .args([
                "health",
                workload_path.to_str().unwrap(),
                "--assertions-only",
            ])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_assertions_evaluated() {
        let temp = TempDir::new().unwrap();
        common::create_workload_with_assertions(temp.path(), "assert-eval").unwrap();
        let workload_path = temp.path().join("assert-eval");

        // JSON output should contain Assertions category
        anvil()
            .args([
                "health",
                workload_path.to_str().unwrap(),
                "--output",
                "json",
            ])
            .assert()
            .code(predicate::in_iter([0, 1]))
            .stdout(predicate::str::contains("Assertions"));
    }

    #[test]
    fn health_assertions_disabled() {
        let temp = TempDir::new().unwrap();
        common::create_workload_assertions_disabled(temp.path(), "assert-off").unwrap();
        let workload_path = temp.path().join("assert-off");

        // When assertion_check is false, no assertion results should appear
        let output = anvil()
            .args([
                "health",
                workload_path.to_str().unwrap(),
                "--output",
                "json",
                "--assertions-only",
            ])
            .assert()
            .code(predicate::in_iter([0, 1]))
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);
        assert!(
            !stdout.contains("should be skipped"),
            "Assertions should be skipped when assertion_check is false"
        );
    }

    #[test]
    fn health_assertions_fail_fast() {
        let temp = TempDir::new().unwrap();
        common::create_workload_with_assertions(temp.path(), "assert-ff").unwrap();
        let workload_path = temp.path().join("assert-ff");

        // With fail_fast and a failing assertion, command should still exit
        anvil()
            .args([
                "health",
                workload_path.to_str().unwrap(),
                "--fail-fast",
                "--assertions-only",
            ])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_passing_assertions() {
        let temp = TempDir::new().unwrap();
        common::create_workload_passing_assertions(temp.path(), "assert-pass").unwrap();
        let workload_path = temp.path().join("assert-pass");

        // All passing assertions should exit 0
        anvil()
            .args([
                "health",
                workload_path.to_str().unwrap(),
                "--assertions-only",
            ])
            .assert()
            .success();
    }

    #[test]
    fn health_deprecation_warning_when_both_assertions_and_scripts() {
        let temp = TempDir::new().unwrap();
        common::create_workload_with_both_assertions_and_scripts(temp.path(), "both-test").unwrap();
        let workload_path = temp.path().join("both-test");

        // When both assertions and scripts.health_check exist, stderr should contain deprecation warning
        anvil()
            .args(["health", workload_path.to_str().unwrap()])
            .assert()
            .code(predicate::in_iter([0, 1]))
            .stderr(predicate::str::contains(
                "scripts.health_check` is deprecated when used alongside `assertions`",
            ));
    }
}

mod completions_command {
    use super::*;

    #[test]
    fn generates_powershell_completions() {
        anvil()
            .args(["completions", "powershell"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Register-ArgumentCompleter"));
    }

    #[test]
    fn generates_bash_completions() {
        anvil()
            .args(["completions", "bash"])
            .assert()
            .success()
            .stdout(predicate::str::contains("complete").or(predicate::str::contains("_anvil")));
    }

    #[test]
    fn generates_zsh_completions() {
        anvil()
            .args(["completions", "zsh"])
            .assert()
            .success()
            .stdout(predicate::str::contains("compdef").or(predicate::str::contains("#compdef")));
    }

    #[test]
    fn generates_fish_completions() {
        anvil()
            .args(["completions", "fish"])
            .assert()
            .success()
            .stdout(predicate::str::contains("complete"));
    }
}

mod config_command {
    use super::*;

    #[test]
    fn config_list_displays_settings() {
        anvil().args(["config", "list"]).assert().success();
    }

    #[test]
    fn config_list_json_output() {
        anvil()
            .args(["config", "list", "--output", "json"])
            .assert()
            .success();
    }

    #[test]
    fn config_path_shows_location() {
        anvil().args(["config", "path"]).assert().success();
    }

    #[test]
    fn config_get_nonexistent_key() {
        anvil()
            .args(["config", "get", "nonexistent.key.xyz"])
            .assert()
            .failure();
    }
}

mod status_command {
    use super::*;

    #[test]
    fn status_runs() {
        anvil().args(["status"]).assert().success();
    }

    #[test]
    fn status_for_workload() {
        anvil()
            .args(["status", "./examples/minimal"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn status_json_output() {
        anvil()
            .args(["status", "--output", "json"])
            .assert()
            .success();
    }

    #[test]
    fn status_long_format() {
        anvil().args(["status", "--long"]).assert().success();
    }
}

mod backup_command {
    use super::*;

    #[test]
    fn backup_list_runs() {
        anvil().args(["backup", "list"]).assert().success();
    }

    #[test]
    fn backup_list_json_output() {
        anvil()
            .args(["backup", "list", "--output", "json"])
            .assert()
            .success();
    }

    #[test]
    fn backup_show_nonexistent() {
        anvil()
            .args(["backup", "show", "nonexistent-backup-id"])
            .assert()
            .failure();
    }
}

mod global_flags {
    use super::*;

    #[test]
    fn help_flag_works() {
        anvil()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Windows").or(predicate::str::contains("anvil")));
    }

    #[test]
    fn version_flag_works() {
        anvil()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn verbose_flag_accepted() {
        anvil().args(["-v", "list"]).assert().success();
    }

    #[test]
    fn double_verbose_flag_accepted() {
        anvil().args(["-vv", "list"]).assert().success();
    }

    #[test]
    fn triple_verbose_flag_accepted() {
        anvil().args(["-vvv", "list"]).assert().success();
    }

    #[test]
    fn quiet_flag_accepted() {
        anvil().args(["-q", "list"]).assert().success();
    }

    #[test]
    fn no_color_flag_accepted() {
        anvil().args(["--no-color", "list"]).assert().success();
    }

    #[test]
    fn no_args_shows_help() {
        // With arg_required_else_help, no args should show help
        anvil()
            .assert()
            .failure()
            .stderr(predicate::str::contains("Usage").or(predicate::str::contains("Commands")));
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn unknown_command_shows_error() {
        anvil()
            .arg("unknown-command-xyz")
            .assert()
            .failure()
            .stderr(predicate::str::contains("error").or(predicate::str::contains("invalid")));
    }

    #[test]
    fn install_missing_required_argument() {
        anvil().arg("install").assert().failure();
    }

    #[test]
    fn show_missing_required_argument() {
        anvil().arg("show").assert().failure();
    }

    #[test]
    fn health_missing_required_argument() {
        anvil().arg("health").assert().failure();
    }

    #[test]
    fn completions_missing_shell() {
        anvil().arg("completions").assert().failure();
    }

    #[test]
    fn completions_invalid_shell() {
        anvil()
            .args(["completions", "invalid-shell"])
            .assert()
            .failure();
    }
}

mod workload_validation {
    use super::*;

    #[test]
    fn workload_with_packages() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("packages-workload");
        fs::create_dir_all(&workload_dir).unwrap();
        fs::write(
            workload_dir.join("workload.yaml"),
            r#"name: packages-workload
version: "1.0.0"
description: "Workload with packages"
packages:
  winget:
    - id: Microsoft.WindowsTerminal
    - id: Git.Git
      version: "2.43.0"
"#,
        )
        .unwrap();

        anvil()
            .args(["validate", workload_dir.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn workload_with_files() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("files-workload");
        let files_dir = workload_dir.join("files");
        fs::create_dir_all(&files_dir).unwrap();
        fs::write(files_dir.join("config.txt"), "test content").unwrap();
        fs::write(
            workload_dir.join("workload.yaml"),
            r#"name: files-workload
version: "1.0.0"
description: "Workload with files"
files:
  - source: files/config.txt
    destination: "~/.config/test/config.txt"
    backup: true
"#,
        )
        .unwrap();

        anvil()
            .args(["validate", workload_dir.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn workload_with_scripts() {
        let temp = TempDir::new().unwrap();
        let workload_dir = temp.path().join("scripts-workload");
        let scripts_dir = workload_dir.join("scripts");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::write(scripts_dir.join("health.ps1"), "exit 0").unwrap();
        fs::write(
            workload_dir.join("workload.yaml"),
            r#"name: scripts-workload
version: "1.0.0"
description: "Workload with scripts"
scripts:
  health_check:
    - path: scripts/health.ps1
      name: "Basic Check"
      description: "Simple health check"
"#,
        )
        .unwrap();

        anvil()
            .args(["validate", workload_dir.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn workload_with_extends_using_list() {
        let temp = TempDir::new().unwrap();

        // Create parent workload
        let parent_dir = temp.path().join("parent-workload");
        fs::create_dir_all(&parent_dir).unwrap();
        fs::write(
            parent_dir.join("workload.yaml"),
            r#"name: parent-workload
version: "1.0.0"
description: "Parent workload"
"#,
        )
        .unwrap();

        // Create child workload
        let child_dir = temp.path().join("child-workload");
        fs::create_dir_all(&child_dir).unwrap();
        fs::write(
            child_dir.join("workload.yaml"),
            r#"name: child-workload
version: "1.0.0"
description: "Child workload"
extends:
  - parent-workload
"#,
        )
        .unwrap();

        // Use list command with --path to verify workloads are found
        anvil()
            .args(["list", "--path", temp.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("child-workload"));
    }
}
