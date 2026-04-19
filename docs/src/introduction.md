# Anvil

**Declarative Workstation Configuration Management**

Anvil is a configuration management tool that lets you define your development
environment in YAML. Describe the packages, configuration files, scripts, and
environment variables your workstation needs, and Anvil takes care of the rest —
installing software, deploying dotfiles, running setup scripts, and validating
that everything is healthy.

## Key Features

- **Package management** — install software via winget with version pinning (Homebrew and APT planned)
- **File synchronization** — deploy configuration files with automatic backup and integrity checks
- **Script execution** — run PowerShell setup and validation scripts with timeout and elevation support
- **Workload inheritance** — compose configurations with DRY principles using `extends`
- **Health checks** — validate that the live system matches your workload definition
- **Multiple output formats** — table, JSON, YAML, and HTML reports
- **Backup and restore** — snapshot and roll back system state
- **Shell completions** — tab completion for PowerShell, Bash, Zsh, and Fish

## Quick Install

```sh
# From crates.io (requires Rust 1.75+)
cargo install anvil-dev
```

Or download a pre-built binary from the
[Releases](https://github.com/kafkade/anvil/releases) page.

## Quick Start

```sh
# List available workloads
anvil list

# Preview what an install would do
anvil install rust-developer --dry-run

# Install a workload
anvil install rust-developer

# Verify system health
anvil health rust-developer
```

## Learn More

- Read the **[User Guide](user-guide.md)** for complete usage instructions.
- See **[Workload Authoring](workload-authoring.md)** to create your own workloads.
- Check the **[Troubleshooting](troubleshooting.md)** guide if you run into issues.
- Review the **[Specification](specification.md)** for the technical design and roadmap.
- Explore the **[Architecture](architecture.md)** docs to understand the codebase.
- Want to help? Read the **[Contributing](contributing.md)** guide.

---

*If Anvil saves you time, consider [sponsoring the project](https://github.com/sponsors/kafkade).*
