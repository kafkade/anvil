# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Workload path shown in the TUI browser preview pane and detail view header
- Pressing Enter in the workload browser opens the detail view for the selected workload

### Fixed

- Windows UNC path prefix (`\\?\`) no longer appears in source paths and workload displays
- TUI no longer registers each keystroke twice on Windows (filtered key release events)

## [1.2.0] - 2026-05-09

### Added

- `anvil source` command family for managing workload sources from the CLI
- `anvil source list` shows all configured sources with origin (default/local/remote) and workload counts
- `anvil source add <path>` adds a local directory as a workload source
- `anvil source add <git-url>` clones a git repository as a remote workload source with `--name`, `--ref`, and `--path` options
- `anvil source remove <name>` removes a source (with `--delete` to clean up cloned files)
- `anvil source status` shows sync status, current ref, dirty state, and last synced time for all sources
- `anvil source sync [name]` pulls latest changes for remote sources (non-destructive -- skips dirty working trees)
- Remote source metadata persisted in `~/.anvil/sources.json`
- Remote repositories cloned to `~/.anvil/sources/<name>/` using shallow clones for fast setup
- Managed sources automatically integrated into workload discovery (after user paths, before defaults)
- JSON and YAML output formats for `source list` and `source status`
- `anvil registry` command family for browsing and installing community workloads
- `anvil registry list` displays all workloads available in the registry
- `anvil registry search <query>` searches workloads by name, description, tags, or author
- `anvil registry add <name>` installs a registry workload as a remote source with one command
- Configurable registry URL via `registry.url` in global config (`anvil config set registry.url <url>`)
- Local registry cache with 1-hour TTL and `--refresh` flag to force update
- Version compatibility checking -- registry entries can specify minimum Anvil version
- Interactive TUI installation dashboard with real-time progress across all phases (`anvil install`)
- `--no-tui` flag to disable the interactive dashboard and use plain progress output
- TUI automatically activates when stdout and stdin are a TTY; falls back to existing output in CI/scripts
- Forge-branded color theme for TUI views (amber/gold accent palette)
- Reusable TUI widget library: status badges with animated spinners, progress gauges, and key hints bar
- Interactive workload browser for `anvil list` with search filtering, preview pane, and direct install (press Enter)
- Interactive health report viewer for `anvil health` with collapsible sections, failure filtering (press f), and inline error details
- Rich workload detail view for `anvil show` with collapsible sections for inheritance, packages, files, commands, and assertions
- Interactive status dashboard for `anvil status` showing installed workloads, source summary, and system info
- `--no-tui` flag on `anvil list`, `anvil health`, `anvil show`, and `anvil status` to disable interactive views
- Declarative `fonts:` block in workload schema for downloading and installing fonts from zip archives
- Font provider automatically downloads, extracts, and registers font files in the Windows font registry
- `font_installed` assertion type for verifying fonts are installed (e.g., `type: font_installed, name: Lilex`)
- Font installation phase in the install dashboard (between packages and files)
- Declarative `terminal:` block for managing Windows Terminal color schemes and profile defaults
- `terminal_scheme_exists` assertion type for verifying color schemes are present in Windows Terminal settings
- Declarative `features:` block for toggling Windows system features via registry settings (sudo, developer mode, etc.)
- OS build gating for features (e.g., `min_build: 26100` skips gracefully on older Windows versions)

## [1.1.0] - 2026-04-18

### Changed

- Update release to publish package to winget.

## [1.0.0] - 2026-04-18

### Added

- Promotional landing page at `anvil.kafkade.com` with feature overview, code demo, and installation instructions
- Separate mdbook documentation served at `anvil.kafkade.com/docs/`
- Self-contained site build script (`scripts/build-site.sh`) that auto-installs mdbook
- Redirect rules for old root-level documentation URLs to `/docs/` subpath

### Changed

- Documentation URLs now use `/docs/` subpath (e.g., `anvil.kafkade.com/docs/user-guide.html`)
- CI `Documentation` job now validates the full site build (promotional site + mdbook)
- Crate renamed from `anvil-cli` to `anvil-dev` for crates.io availability (binary name stays `anvil`)
- `homepage` in `Cargo.toml` now points to `anvil.kafkade.com` instead of the GitHub repo
- Published crate is now lean — `exclude` list trims CI, docs, site, and scripts from the package

### Removed

- `scripts.health_check` field — use declarative `assertions` instead
- `--scripts-only` and `--script` flags from `anvil health` command
- `scripts.pre_install` and `scripts.post_install` fields — use the `commands` block instead
- `--skip-scripts`, `--skip-pre-scripts`, `--skip-post-scripts` flags from `anvil install` command

## [0.6.0] - 2026-04-17

### Added

- Declarative assertions for workload health validation (`assertions:` field in workload YAML)
- Condition engine with 9 predicate types: `command_exists`, `file_exists`, `dir_exists`, `env_var`, `path_contains`, `registry_value`, `shell`, plus `all_of`/`any_of` composition
- `--assertions-only` flag for `anvil health` to run only assertion checks
- `assertion_check` toggle in workload health configuration
- Assertion examples in `anvil init --template full` scaffold
- Multi-manager workload schema: `packages.brew` (Homebrew) and `packages.apt` (APT) fields alongside existing `packages.winget`
- Platform-aware validation warns when workload references an unavailable package manager
- Inline `commands:` block for workload command execution with `pre_install` and `post_install` phases
- Conditional command execution via `when:` field using the predicate engine
- `continue_on_error` option for commands that should not block the install flow
- Configurable workload search paths via `~/.anvil/config.yaml` (user paths prepended to defaults)
- Search precedence with conflict resolution: explicit path > user-configured > defaults; first match wins
- `anvil list --all-paths` to show all discovered paths including shadowed duplicates
- Cross-platform release builds for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64)
- Documentation website powered by mdBook, served at [anvil.kafkade.com](https://anvil.kafkade.com)
- Animated forge lettermark banner on `anvil --version` with molten gradient

### Changed

- Crate renamed to `anvil-dev` for crates.io publishing (binary name stays `anvil`; install via `cargo install anvil-dev`)
- Tracing/log output now writes to stderr instead of stdout, preventing pollution of structured output (JSON, YAML)

### Fixed

- Integration tests no longer fail on Linux and macOS due to winget-dependent tests running on non-Windows platforms

### Deprecated

- ~~`scripts.health_check` when used alongside `assertions`~~ — **removed** (see Removed above)
- `scripts.pre_install` and `scripts.post_install` when used alongside `commands` (migrate to inline commands; removal planned for v1.0)

## [0.5.0] - 2026-04-17

This is the first release from the new repository home at [kafkade/anvil](https://github.com/kafkade/anvil). It marks a fresh start with modernized CI/CD, updated documentation, and a clear cross-platform direction. Prior changelog entries (v0.1.0–v0.3.1) are preserved below for historical context.

### Added

- Architecture reference document for contributors (`docs/ARCHITECTURE.md`)
- Automated releases from CHANGELOG.md with SHA256 checksums
- Consolidated CI pipeline with formatting, linting, and testing in a single gate

### Changed

- Project rebranded to "Declarative Workstation Configuration Management" to reflect cross-platform direction
- Streamlined CI from 3 separate jobs to a single `Validate` gate plus release build
- Release workflow aligned with org-wide pattern (changelog-driven notes, semver pre-release detection)
- Resolved all clippy warnings and formatting issues across the codebase

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

- Initial release of Anvil
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
  - essentials: Core development tools (VS Code, Git, Windows Terminal, Oh My Posh) and productivity utilities
  - rust-developer: Rust toolchain with cargo tools (extends essentials)
  - python-developer: Python with uv package manager (extends essentials)

### Documentation

- Initial specification document
- Phase development prompts

---

[Unreleased]: https://github.com/kafkade/anvil/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/kafkade/anvil/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/kafkade/anvil/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/kafkade/anvil/compare/v0.6.0...v1.0.0
[0.6.0]: https://github.com/kafkade/anvil/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/kafkade/anvil/compare/v0.3.1...v0.5.0
[0.3.1]: https://github.com/kafkade/anvil/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/kafkade/anvil/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/kafkade/anvil/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kafkade/anvil/releases/tag/v0.1.0
