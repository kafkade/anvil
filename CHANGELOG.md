# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Nothing yet

### Changed
- Nothing yet

### Fixed
- Nothing yet

## [0.3.1] - 2026-01-10

### Added
- Comprehensive user documentation (USER_GUIDE.md)
- Workload authoring guide (WORKLOAD_AUTHORING.md)
- Troubleshooting guide (TROUBLESHOOTING.md)
- Integration test suite for CLI commands
- GitHub Actions CI/CD workflows
- Shell completion generation for PowerShell, Bash, Zsh, and Fish
- CONTRIBUTING.md with contribution guidelines

### Changed
- Improved error messages throughout CLI
- Enhanced validation reporting with detailed messages
- Updated README with badges and clearer instructions

### Fixed
- Various documentation typos and inconsistencies

## [0.3.0] - 2026-01-08

### Added
- Shell completions command for multiple shells
- Global configuration management (`config` command)
- Backup management with create, list, show, restore, clean, and verify subcommands
- Status command to show installation state
- HTML output format for health reports
- `--strict` flag for validation command
- Environment variable configuration in workloads
- PATH additions support in workloads

### Changed
- Improved inheritance resolution algorithm
- Enhanced output formatting for all commands
- Better progress indicators during installation

## [0.2.0] - 2026-01-05

### Added
- Script execution support with PowerShell and CMD
- Pre-install and post-install script hooks
- Health check script execution
- Elevated privilege handling for scripts
- Configurable script timeouts
- Template processing with Handlebars
- File integrity verification with SHA-256 checksums
- Automatic backup creation before file overwrites
- Variable expansion in paths (`~`, `${HOME}`, etc.)

### Changed
- Improved file operation error handling
- Enhanced dry-run output with detailed plan

### Fixed
- Path expansion on Windows with backslashes
- File permission handling on Windows

## [0.1.0] - 2026-01-01

### Added
- Initial release of Winforge
- Core CLI commands: install, health, list, show, validate, init
- Package management via winget integration
  - Install packages with version pinning
  - Custom winget arguments support
  - Package health verification
- File operations
  - Copy files to target locations
  - Path variable expansion
- Workload system
  - YAML-based workload definitions
  - Workload inheritance and composition
  - Circular dependency detection
- Multiple output formats
  - Table (default, human-readable)
  - JSON (machine-readable)
  - YAML
- Bundled workloads
  - dev-tools-base: VS Code, Git, Windows Terminal, Oh My Posh
  - rust-developer: Rust toolchain with cargo tools
  - python-developer: Python with uv package manager
  - essentials: Essential Windows utilities

### Documentation
- Initial specification document
- Phase development prompts

---

[Unreleased]: https://github.com/javierfe_microsoft/winforge/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/javierfe_microsoft/winforge/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/javierfe_microsoft/winforge/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/javierfe_microsoft/winforge/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/javierfe_microsoft/winforge/releases/tag/v0.1.0
