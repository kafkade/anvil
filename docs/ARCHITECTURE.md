# Architecture

This document describes Anvil's internal architecture for contributors. For user-facing documentation, see the [User Guide](USER_GUIDE.md). For the project vision and roadmap, see the [Specification](SPECIFICATION.md).

## Overview

Anvil is a single-binary Rust CLI. The binary parses commands, loads YAML workload definitions, delegates work to providers, and tracks state on disk.

```
main.rs → cli/ → operations/ → providers/
                      ↓              ↓
                   config/        state/
```

| Layer | Role |
|-------|------|
| **cli/** | Command parsing (clap), output formatting (table/JSON/YAML/HTML), progress bars |
| **config/** | Workload YAML loading, schema validation, inheritance resolution, global config |
| **operations/** | One module per CLI command — each exposes `execute(args, cli) → Result<()>` |
| **providers/** | External system integrations: winget, filesystem, scripts, templates, backups |
| **state/** | Tracks installation records, file hashes, and package cache at `~/.anvil/` |

## Data Flow: `anvil install <workload>`

```
1. CLI parsing          cli/mod.rs        → InstallArgs
2. Workload discovery   config/mod.rs     → finds workload.yaml on search paths
3. YAML loading         config/mod.rs     → Workload struct (serde)
4. Inheritance          config/inheritance.rs → resolved Workload (parents merged)
5. Variable expansion   config/mod.rs     → ~ and ${VAR} expanded
6. Plan & confirm       operations/install.rs → dry-run plan, user confirmation
7. Package install      providers/winget.rs   → winget install per package
8. File copy            providers/filesystem.rs → copy with backup and hash
9. Script execution     providers/script.rs   → PowerShell pre/post scripts
10. State persistence   state/              → JSON files at ~/.anvil/state/
```

## Key Types

### Workload Schema (`config/workload.rs`)

```rust
pub struct Workload {
    pub name: String,
    pub version: String,
    pub description: String,
    pub extends: Option<Vec<String>>,
    pub packages: Option<Packages>,
    pub files: Option<Vec<FileEntry>>,
    pub scripts: Option<Scripts>,
    pub environment: Option<Environment>,
    pub health: Option<HealthConfig>,
}
```

The `Workload` struct is the central data type — deserialized from YAML via serde. All fields except `name`, `version`, and `description` are optional.

### CLI (`cli/mod.rs`, `cli/commands.rs`)

10 top-level commands: `install`, `health`, `list`, `show`, `validate`, `init`, `status`, `completions`, `backup`, `config`.

The `Cli` struct (clap-derived) holds global flags (`--verbose`, `--quiet`, `--no-color`, `--config`). Each command has its own args struct.

### Providers (`providers/`)

Providers wrap external systems. Each provider is a struct with methods — there is no shared trait yet (see roadmap).

| Provider | Responsibility |
|----------|----------------|
| `WingetProvider` | Package install/upgrade/query via `winget.exe` |
| `FilesystemProvider` | File copy with backup, hash verification, glob expansion |
| `ScriptProvider` | PowerShell script execution with timeout and output capture |
| `TemplateProcessor` | Handlebars template rendering for config files |
| `BackupManager` | System state backup and restore |

### State (`state/`)

All state is persisted as JSON under `~/.anvil/`:

```
~/.anvil/
├── state/
│   ├── <workload>.json      # InstallationState per workload
│   └── files.json           # FileStateIndex (all tracked files)
├── cache/
│   └── packages.json        # PackageCache (winget query results)
└── config.yaml              # GlobalConfig (user preferences)
```

| Type | File | Purpose |
|------|------|---------|
| `InstallationState` | `state/<workload>.json` | Package install records and status |
| `FileStateManager` | `state/files.json` | File hashes for drift detection |
| `PackageCache` | `cache/packages.json` | Cached winget query results |
| `GlobalConfig` | `config.yaml` | User settings and search paths |

## Config System

### Workload Discovery

`ConfigManager` searches for workloads in this order:

1. Direct path (if an absolute or relative path is given)
2. `<exe_dir>/workloads/<name>/workload.yaml`
3. `%LOCALAPPDATA%/anvil/workloads/<name>/workload.yaml`
4. `./workloads/<name>/workload.yaml`

Within each search path, it tries: `<name>/workload.yaml`, `<name>/workload.yml`, `<name>.yaml`.

### Inheritance

Workloads can extend other workloads via `extends: [parent]`. Resolution in `config/inheritance.rs`:

1. Build dependency graph from all `extends` references
2. Detect cycles (error) and enforce max depth of 10
3. Topological sort determines merge order (parents first)
4. Merge strategy:
   - **Packages**: append, child overrides same ID
   - **Files**: append, child overrides same destination
   - **Scripts**: concatenate (parent first, child after)
   - **Environment variables**: child overrides same name
   - **Path additions**: append unique entries

### Variable Expansion

Workload values support variable expansion:

| Variable | Expands to |
|----------|------------|
| `~` | `$env:USERPROFILE` |
| `${HOME}` | `$env:USERPROFILE` |
| `${ANVIL_WORKLOAD}` | Current workload name |
| `${ANVIL_VERSION}` | Anvil version |
| `${ANVIL_WORKLOAD_PATH}` | Path to workload directory |
| `${ENV_NAME}` | Any environment variable |

## Error Handling

Two-tier approach:

- **`thiserror`** for domain-specific error enums in each module:
  - `WingetError`, `FilesystemError`, `ScriptError`, `BackupError`, `TemplateError`
  - `InheritanceError` (includes `suggestion()` for user-friendly hints)
  - `FileStateError`
  - `ProviderError` (wraps all provider errors)
- **`anyhow`** for error propagation in operations and CLI code

Pattern: domain errors are created with `thiserror` and converted to `anyhow::Error` at operation boundaries using `.with_context(|| ...)`.

## Testing

### Unit Tests

Inline `#[cfg(test)] mod tests` in the same file as the code under test. 168 unit tests covering providers, config parsing, inheritance, state management, and formatting.

### Integration Tests

`tests/cli_tests.rs` uses `assert_cmd` + `predicates` for end-to-end CLI testing. 75 integration tests covering all commands with fixture workloads.

`tests/common/mod.rs` provides test fixture helpers:
- `create_test_workload()` — minimal valid workload
- `create_inherited_workload()` — parent + child workloads
- `create_invalid_workload()` — malformed YAML
- `create_circular_workloads()` — cycle detection fixtures
- `create_full_workload()` — workload with all features
- `create_template_workload()` — workload with template files

### Running Tests

```sh
cargo test                   # All tests (243 total)
cargo test --bin anvil       # Unit tests only (168)
cargo test --test cli_tests  # Integration tests only (75)
cargo test test_name         # Single test by name
```

## Output Formats

All commands that produce output support `--format`:

| Format | Module | Description |
|--------|--------|-------------|
| `table` | `cli/formats/table.rs` | Human-readable terminal tables (default) |
| `json` | `cli/formats/json.rs` | Machine-readable JSON |
| `yaml` | `cli/formats/yaml.rs` | YAML output |
| `html` | `cli/formats/html.rs` | Standalone HTML reports |

## CI Pipeline

`.github/workflows/ci.yml` runs on every push/PR:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo check`
4. `cargo test --bin anvil` (unit tests)
5. `cargo test --test cli_tests` (integration tests)
6. `cargo build --release`
7. Verify binary runs (`./anvil --version`)
