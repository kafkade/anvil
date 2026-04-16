# Copilot Instructions for Anvil

## Build, Test, and Lint

```powershell
cargo build                  # Debug build
cargo build --release        # Release build (LTO + stripped)
cargo test                   # All tests
cargo test --lib             # Unit tests only
cargo test --test '*'        # Integration tests only
cargo test test_name         # Single test by name
cargo test -- --nocapture    # Tests with stdout visible
cargo clippy --all-targets --all-features -- -D warnings  # Lint (CI enforces zero warnings)
cargo fmt --all -- --check   # Format check
cargo fmt                    # Auto-format
```

## Architecture

Anvil is a declarative Windows workstation configuration tool. Users define environments in YAML workload files; Anvil installs packages (via winget), copies files, runs PowerShell scripts, and validates system health.

### Module layout and data flow

```
main.rs → cli/ → operations/ → providers/
                      ↓              ↓
                   config/        state/
```

- **`cli/`** — Clap-derived CLI parsing, output formatting (table/JSON/YAML/HTML), progress bars
- **`config/`** — Workload YAML parsing (`workload.rs`), schema validation (`schema.rs`), inheritance resolution (`inheritance.rs`), global config (`global.rs`). The central type is `ConfigManager` which discovers and loads workloads.
- **`operations/`** — One file per CLI command (`install.rs`, `health.rs`, `list.rs`, etc.). Each exposes an `execute(args, cli)` function called from `main.rs`.
- **`providers/`** — External system integrations: `winget.rs` (package manager), `filesystem.rs` (file copy with backup/hash verification), `script.rs` (PowerShell execution), `template.rs` (Handlebars templates), `backup.rs`
- **`state/`** — Tracks installation state (`installation.rs`), file state with hashes (`files.rs`), and package cache (`cache.rs`). State persisted to `~/.anvil/state/`.

### Workload structure

A workload is a directory containing `workload.yaml` plus optional `files/` and `scripts/` subdirectories. Bundled workloads live in `workloads/`. The core data type is `config::workload::Workload` (deserialized from YAML via serde).

### Workload inheritance

Workloads can `extends: [parent]` to inherit from other workloads. Resolution in `config/inheritance.rs`:
- Builds a dependency graph, detects cycles, enforces max depth of 10
- Topological sort determines merge order (parent first)
- Merge strategy: packages are appended (child overrides same ID), files are appended (child overrides same destination), scripts are concatenated (parent first), env vars are overridden by child if same name

## Conventions

### Error handling

Two-tier approach:
- **`thiserror`** for domain-specific error enums (e.g., `WingetError`, `InheritanceError`, `FilesystemError`, `ScriptError`, `TemplateError`). Each provider/module defines its own error type with structured variants.
- **`anyhow`** for error propagation in application/operation code. Use `.with_context(|| ...)` for adding context.
- `InheritanceError` includes a `suggestion()` method for user-friendly hints.

### Serde patterns

All workload structs derive `Serialize, Deserialize`. Optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`. Custom defaults use helper functions like `default_true()`, `default_shell()`, `default_timeout()`.

### Testing

- Unit tests are inline `#[cfg(test)] mod tests` in the same file
- Integration tests in `tests/cli_tests.rs` use `assert_cmd` + `predicates` to test CLI end-to-end
- Test fixtures created via helpers in `tests/common/mod.rs` (`create_test_workload`, `create_inherited_workload`, `create_invalid_workload`, `create_circular_workloads`, `create_full_workload`, `create_template_workload`)
- Use `tempfile::TempDir` for isolated filesystem tests
- Use `pretty_assertions` for readable diff output

### PR conventions

Title format: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`

### Adding a new CLI command

1. Add variant to `Commands` enum in `cli/commands.rs`
2. Create args struct with clap derives
3. Create `operations/new_command.rs` with `pub fn execute(args, cli) -> Result<()>`
4. Add match arm in `main.rs`

### Adding a new workload

Create `workloads/<name>/workload.yaml` (plus optional `files/` and `scripts/`). Validate with `anvil validate <name> --strict`. Use `extends: [essentials]` for common dev tools.
