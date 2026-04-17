# Anvil — Copilot Instructions

## Build & Test

```sh
cargo build                  # Debug build
cargo build --release        # Release build (LTO + stripped)
cargo test                   # All tests (243: 168 unit + 75 integration)
cargo test --bin anvil       # Unit tests only
cargo test --test cli_tests  # Integration tests only
cargo test test_name         # Single test by name
cargo test -- --nocapture    # Tests with stdout visible
cargo clippy --all-targets --all-features -- -D warnings  # Lint (CI enforces zero warnings)
cargo fmt --all -- --check   # Format check
cargo fmt                    # Auto-format
```

## Architecture

Anvil is a declarative workstation configuration tool. Users define environments in YAML workload files; Anvil installs packages (currently via winget, with cross-platform package managers planned), copies files, runs scripts, and validates system health.

See `docs/ARCHITECTURE.md` for the full internal architecture reference.

### Module layout

```
main.rs → cli/ → operations/ → providers/
                      ↓              ↓
                   config/        state/
```

- **`cli/`** — Clap-derived CLI parsing, output formatting (table/JSON/YAML/HTML), progress bars
- **`config/`** — Workload YAML parsing (`workload.rs`), schema validation (`schema.rs`), inheritance resolution (`inheritance.rs`), global config (`global.rs`). Central type: `ConfigManager`
- **`operations/`** — One file per CLI command (`install.rs`, `health.rs`, `list.rs`, etc.). Each exposes `execute(args, cli) → Result<()>`
- **`providers/`** — External system integrations: `winget.rs` (package manager), `filesystem.rs` (file copy with backup/hash), `script.rs` (script execution), `template.rs` (Handlebars), `backup.rs`
- **`state/`** — Installation state, file hashes, and package cache. Persisted as JSON at `~/.anvil/`

### Workload structure

A workload is a directory containing `workload.yaml` plus optional `files/` and `scripts/` subdirectories. Example workloads live in `examples/`. Core data type: `config::workload::Workload` (serde-deserialized from YAML).

### Workload inheritance

Workloads can `extends: [parent]` to inherit from other workloads. Resolution in `config/inheritance.rs`:
- Builds dependency graph, detects cycles, enforces max depth of 10
- Topological sort determines merge order (parent first)
- Merge: packages appended (child overrides same ID), files appended (child overrides same destination), scripts concatenated (parent first), env vars overridden by child

## Conventions

### Error handling

- **`thiserror`** for domain-specific error enums (`WingetError`, `InheritanceError`, `FilesystemError`, `ScriptError`, `TemplateError`). Each module defines its own error type.
- **`anyhow`** for propagation in operations/CLI code. Use `.with_context(|| ...)` for adding context.
- `InheritanceError` includes `suggestion()` for user-friendly hints.

### Serde patterns

All workload structs derive `Serialize, Deserialize`. Optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`. Custom defaults via helper functions (`default_true()`, `default_shell()`, `default_timeout()`).

### Testing

- Unit tests: inline `#[cfg(test)] mod tests` in the same file
- Integration tests: `tests/cli_tests.rs` using `assert_cmd` + `predicates`
- Fixtures: `tests/common/mod.rs` helpers (`create_test_workload`, `create_inherited_workload`, `create_full_workload`, etc.)
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

Create `examples/<name>/workload.yaml` (plus optional `files/` and `scripts/`). Validate with `anvil validate <name> --strict`. Use `extends: [essentials]` for common dev tools.

## Git Policy

**Never execute Git commands that modify history or submit code.** This includes `git commit`, `git push`, `git rebase`, `git merge`, `git reset`, `git cherry-pick`, `git revert`, and `git tag`. Read-only commands like `git status`, `git diff`, `git log`, and `git branch` are fine. The maintainer must always review and commit changes themselves.

## Key References

- `docs/SPECIFICATION.md` — Project spec, workload schema, and roadmap (v0.4–v1.0)
- `docs/ARCHITECTURE.md` — Internal code architecture for contributors
- `docs/USER_GUIDE.md` — End-user CLI usage guide
- `docs/WORKLOAD_AUTHORING.md` — Workload YAML authoring reference
- `CONTRIBUTING.md` — Contribution workflow and coding standards
- `CHANGELOG.md` — Release history
