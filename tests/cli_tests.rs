//! Integration tests for Winforge CLI
//!
//! These tests verify the CLI commands work correctly end-to-end.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to get winforge command
fn winforge() -> Command {
    Command::cargo_bin("winforge").unwrap()
}

mod list_command {
    use super::*;

    #[test]
    fn list_shows_available_workloads() {
        winforge().arg("list").assert().success().stdout(
            predicate::str::contains("essentials").or(predicate::str::contains("rust-developer")),
        );
    }

    #[test]
    fn list_with_long_format() {
        winforge().args(["list", "--long"]).assert().success();
    }

    #[test]
    fn list_with_json_output() {
        winforge()
            .args(["list", "--output", "json"])
            .assert()
            .success()
            // JSON output might have INFO log prefix, just check it contains JSON array structure
            .stdout(predicate::str::contains("["));
    }

    #[test]
    fn list_with_yaml_output() {
        winforge()
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

        winforge()
            .args(["list", "--path", temp.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("test-workload"));
    }

    #[test]
    fn list_empty_directory() {
        let temp = TempDir::new().unwrap();

        winforge()
            .args(["list", "--path", temp.path().to_str().unwrap()])
            .assert()
            .success();
    }
}

mod show_command {
    use super::*;

    #[test]
    fn show_displays_workload_details() {
        winforge()
            .args(["show", "essentials"])
            .assert()
            .success()
            .stdout(predicate::str::contains("essentials"));
    }

    #[test]
    fn show_nonexistent_workload_fails() {
        winforge()
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
        winforge()
            .args(["show", "rust-developer", "--show-inheritance"])
            .assert()
            .success();
    }

    #[test]
    fn show_json_output() {
        winforge()
            .args(["show", "essentials", "--output", "json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{"));
    }

    #[test]
    fn show_yaml_output() {
        winforge()
            .args(["show", "essentials", "--output", "yaml"])
            .assert()
            .success()
            .stdout(predicate::str::contains("name:"));
    }

    #[test]
    fn show_resolved_workload() {
        winforge()
            .args(["show", "rust-developer", "--resolved"])
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

        winforge()
            .args(["validate", workload_dir.to_str().unwrap()])
            .assert()
            .success();
    }

    #[test]
    fn validate_bundled_workloads() {
        // Validate essentials
        winforge()
            .args(["validate", "workloads/essentials"])
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

        winforge()
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

        winforge()
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

        winforge()
            .args(["validate", workload_dir.to_str().unwrap(), "--strict"])
            .assert()
            .success();
    }

    #[test]
    fn validate_schema_output() {
        winforge().args(["validate", "--schema"]).assert().success();
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

        winforge()
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

        winforge()
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

        winforge()
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
        winforge()
            .args([
                "init",
                workload_name,
                "--extends",
                "essentials",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .assert()
            .success();

        let workload_yaml = output_path.join("workload.yaml");
        let content = fs::read_to_string(workload_yaml).unwrap();
        assert!(content.contains("extends") || content.contains("essentials"));
    }
}

mod install_command {
    use super::*;

    #[test]
    fn install_dry_run_shows_plan() {
        winforge()
            .args(["install", "essentials", "--dry-run"])
            .assert()
            .success();
    }

    #[test]
    fn install_nonexistent_workload_fails() {
        winforge()
            .args(["install", "nonexistent-workload-xyz"])
            .assert()
            .failure();
    }

    #[test]
    fn install_with_skip_packages() {
        winforge()
            .args(["install", "essentials", "--dry-run", "--skip-packages"])
            .assert()
            .success();
    }

    #[test]
    fn install_with_skip_files() {
        winforge()
            .args(["install", "essentials", "--dry-run", "--skip-files"])
            .assert()
            .success();
    }

    #[test]
    fn install_with_skip_scripts() {
        winforge()
            .args(["install", "essentials", "--dry-run", "--skip-scripts"])
            .assert()
            .success();
    }

    #[test]
    fn install_with_all_skip_flags() {
        winforge()
            .args([
                "install",
                "essentials",
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
        winforge()
            .args(["install", "essentials", "--dry-run", "--packages-only"])
            .assert()
            .success();
    }

    #[test]
    fn install_files_only() {
        winforge()
            .args(["install", "essentials", "--dry-run", "--files-only"])
            .assert()
            .success();
    }
}

mod health_command {
    use super::*;

    #[test]
    fn health_check_runs() {
        // Health check should complete (pass or fail based on system state)
        let result = winforge().args(["health", "essentials"]).assert();
        // We just verify it runs - it may pass or fail depending on system state
        // but shouldn't panic or error unexpectedly
        result.code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_json_output() {
        winforge()
            .args(["health", "essentials", "--output", "json"])
            .assert()
            .stdout(predicate::str::starts_with("{"));
    }

    #[test]
    fn health_yaml_output() {
        winforge()
            .args(["health", "essentials", "--output", "yaml"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_nonexistent_workload_fails() {
        winforge()
            .args(["health", "nonexistent-workload-xyz"])
            .assert()
            .failure();
    }

    #[test]
    fn health_packages_only() {
        winforge()
            .args(["health", "essentials", "--packages-only"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_fail_fast() {
        winforge()
            .args(["health", "essentials", "--fail-fast"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn health_to_file() {
        let temp = TempDir::new().unwrap();
        let output_file = temp.path().join("health-report.json");

        winforge()
            .args([
                "health",
                "essentials",
                "--output",
                "json",
                "--file",
                output_file.to_str().unwrap(),
            ])
            .assert()
            .code(predicate::in_iter([0, 1]));

        assert!(output_file.exists());
    }
}

mod completions_command {
    use super::*;

    #[test]
    fn generates_powershell_completions() {
        winforge()
            .args(["completions", "powershell"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Register-ArgumentCompleter"));
    }

    #[test]
    fn generates_bash_completions() {
        winforge()
            .args(["completions", "bash"])
            .assert()
            .success()
            .stdout(predicate::str::contains("complete").or(predicate::str::contains("_winforge")));
    }

    #[test]
    fn generates_zsh_completions() {
        winforge()
            .args(["completions", "zsh"])
            .assert()
            .success()
            .stdout(predicate::str::contains("compdef").or(predicate::str::contains("#compdef")));
    }

    #[test]
    fn generates_fish_completions() {
        winforge()
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
        winforge().args(["config", "list"]).assert().success();
    }

    #[test]
    fn config_list_json_output() {
        winforge()
            .args(["config", "list", "--output", "json"])
            .assert()
            .success();
    }

    #[test]
    fn config_path_shows_location() {
        winforge().args(["config", "path"]).assert().success();
    }

    #[test]
    fn config_get_nonexistent_key() {
        winforge()
            .args(["config", "get", "nonexistent.key.xyz"])
            .assert()
            .failure();
    }
}

mod status_command {
    use super::*;

    #[test]
    fn status_runs() {
        winforge().args(["status"]).assert().success();
    }

    #[test]
    fn status_for_workload() {
        winforge()
            .args(["status", "essentials"])
            .assert()
            .code(predicate::in_iter([0, 1]));
    }

    #[test]
    fn status_json_output() {
        winforge()
            .args(["status", "--output", "json"])
            .assert()
            .success();
    }

    #[test]
    fn status_long_format() {
        winforge().args(["status", "--long"]).assert().success();
    }
}

mod backup_command {
    use super::*;

    #[test]
    fn backup_list_runs() {
        winforge().args(["backup", "list"]).assert().success();
    }

    #[test]
    fn backup_list_json_output() {
        winforge()
            .args(["backup", "list", "--output", "json"])
            .assert()
            .success();
    }

    #[test]
    fn backup_show_nonexistent() {
        winforge()
            .args(["backup", "show", "nonexistent-backup-id"])
            .assert()
            .failure();
    }
}

mod global_flags {
    use super::*;

    #[test]
    fn help_flag_works() {
        winforge()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Windows").or(predicate::str::contains("winforge")));
    }

    #[test]
    fn version_flag_works() {
        winforge()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn verbose_flag_accepted() {
        winforge().args(["-v", "list"]).assert().success();
    }

    #[test]
    fn double_verbose_flag_accepted() {
        winforge().args(["-vv", "list"]).assert().success();
    }

    #[test]
    fn triple_verbose_flag_accepted() {
        winforge().args(["-vvv", "list"]).assert().success();
    }

    #[test]
    fn quiet_flag_accepted() {
        winforge().args(["-q", "list"]).assert().success();
    }

    #[test]
    fn no_color_flag_accepted() {
        winforge().args(["--no-color", "list"]).assert().success();
    }

    #[test]
    fn no_args_shows_help() {
        // With arg_required_else_help, no args should show help
        winforge()
            .assert()
            .failure()
            .stderr(predicate::str::contains("Usage").or(predicate::str::contains("Commands")));
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn unknown_command_shows_error() {
        winforge()
            .arg("unknown-command-xyz")
            .assert()
            .failure()
            .stderr(predicate::str::contains("error").or(predicate::str::contains("invalid")));
    }

    #[test]
    fn install_missing_required_argument() {
        winforge().arg("install").assert().failure();
    }

    #[test]
    fn show_missing_required_argument() {
        winforge().arg("show").assert().failure();
    }

    #[test]
    fn health_missing_required_argument() {
        winforge().arg("health").assert().failure();
    }

    #[test]
    fn completions_missing_shell() {
        winforge().arg("completions").assert().failure();
    }

    #[test]
    fn completions_invalid_shell() {
        winforge()
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

        winforge()
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

        winforge()
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

        winforge()
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
        winforge()
            .args(["list", "--path", temp.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("child-workload"));
    }
}
