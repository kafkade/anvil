# User Guide

A comprehensive guide to using Anvil for workstation configuration management.

## 1. Introduction

### What is Anvil?

Anvil is a declarative configuration management tool for developer workstations. It allows you to define your development environment in YAML files and automatically:

- Install software packages via package managers (currently winget; Homebrew and APT planned)
- Copy and manage configuration files
- Execute setup and validation scripts
- Verify system health against your defined configuration

### Key Concepts

- **Workload**: A configuration bundle containing package definitions, files to deploy, and scripts to run
- **Package**: A software application to be installed via winget
- **File**: A configuration file to be copied to your system
- **Script**: A PowerShell or CMD script to execute during installation or health checks
- **Inheritance**: The ability to compose workloads by extending other workloads

### System Requirements

**Current platform: Windows**

- Windows 10 (version 1809 or later) or Windows 11
- [Windows Package Manager (winget)](https://github.com/microsoft/winget-cli) version 1.4 or later
- PowerShell 5.1 or later (included with Windows)
- Administrator access (for some operations)

> Cross-platform support (macOS, Linux) is on the [roadmap](SPECIFICATION.md#8-roadmap).

---

## 2. Installation

### Install from crates.io

```powershell
# Prerequisites: Rust 1.75+
cargo install anvil-cli
```

### Download Pre-built Binary

1. Download the latest release from the [Releases page](https://github.com/kafkade/anvil/releases)

2. Extract the archive:

   **PowerShell (Windows):**
   ```powershell
   Expand-Archive anvil-v0.3.1-windows-x64.zip -DestinationPath C:\Tools\anvil
   ```

   **Bash / Zsh (macOS / Linux):**
   ```bash
   tar xzf anvil-v0.3.1-linux-x64.tar.gz -C ~/.local/bin
   ```

3. Add to your PATH:

   **PowerShell:**
   ```powershell
   # Add to current session
   $env:PATH += ";C:\Tools\anvil"
   
   # Add permanently (User scope)
   [Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\Tools\anvil", "User")
   ```

   **Bash / Zsh:**
   ```bash
   # Add to current session
   export PATH="$HOME/.local/bin:$PATH"

   # Add permanently
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
   ```

### Build from Source

```powershell
# Prerequisites: Rust 1.75+ and Visual Studio Build Tools

# Clone the repository
git clone https://github.com/kafkade/anvil.git
cd anvil

# Build release binary
cargo build --release

# The binary is at target/release/anvil.exe
```

### Verify Installation

```powershell
anvil --version
# Output: anvil 0.3.1

anvil --help
# Shows available commands and options
```

### Shell Completions Setup

Generate and install shell completions for better command-line experience:

#### PowerShell

```powershell
# Generate completions
anvil completions powershell > $HOME\Documents\WindowsPowerShell\anvil.ps1

# Add to your PowerShell profile
Add-Content $PROFILE '. $HOME\Documents\WindowsPowerShell\anvil.ps1'
```

#### Bash (WSL/Git Bash)

```bash
# Generate completions
anvil completions bash > ~/.local/share/bash-completion/completions/anvil

# Or add to .bashrc
anvil completions bash >> ~/.bashrc
```

#### Zsh

```zsh
# Generate completions
anvil completions zsh > ~/.zfunc/_anvil

# Add to .zshrc (before compinit)
fpath+=~/.zfunc
```

---

## 3. Quick Start

### List Available Workloads

See what workloads are available:

```powershell
anvil list
```

Output:
```
Available Workloads:
  essentials         Core development tools and productivity utilities
  rust-developer     Rust development environment (extends essentials)
  python-developer   Python development environment (extends essentials)
```

### View Workload Details

Inspect what a workload will do:

```powershell
anvil show rust-developer
```

### Dry Run Installation

Preview what would happen without making changes:

```powershell
anvil install rust-developer --dry-run
```

### Install a Workload

Apply a workload configuration:

```powershell
anvil install rust-developer
```

### Check System Health

Verify your system matches the workload definition:

```powershell
anvil health rust-developer
```

---

## 4. Command Reference

### `install`

Apply a workload configuration to your system.

**Synopsis:**
```
anvil install <WORKLOAD> [OPTIONS]
```

**Arguments:**
- `<WORKLOAD>` - Name of the workload to install

**Options:**
| Option | Description |
|--------|-------------|
| `--dry-run` | Preview actions without making changes |
| `--force` | Skip confirmation prompts |
| `-p, --packages-only` | Only install packages, skip files |
| `--files-only` | Only process files, skip packages |
| `--skip-packages` | Skip package installation |
| `--skip-files` | Skip file operations |
| `--no-backup` | Don't backup existing files before overwriting |
| `--upgrade` | Upgrade existing packages to specified versions |
| `--retry-failed` | Retry only failed packages from previous run |
| `--parallel` | Run installations in parallel where safe |
| `-j, --jobs <N>` | Number of parallel installations (default: 4) |
| `--timeout <SECONDS>` | Global timeout for operations (default: 3600) |
| `--force-files` | Force overwrite files without checking hash |

**Examples:**
```powershell
# Standard installation
anvil install rust-developer

# Preview only
anvil install rust-developer --dry-run

# Skip packages (only copy files and run scripts)
anvil install rust-developer --skip-packages

# Only deploy files
anvil install rust-developer --files-only

# Upgrade existing packages
anvil install rust-developer --upgrade

# Retry previously failed packages
anvil install rust-developer --retry-failed

# Parallel installation with 8 workers
anvil install rust-developer --parallel -j 8
```

**Exit Codes:**
- `0` - Success
- `1` - General error
- `2` - Workload not found
- `3` - Package installation failed
- `4` - File operation failed
- `5` - Script execution failed

---

### `health`

Validate system state against a workload definition.

**Synopsis:**
```
anvil health <WORKLOAD> [OPTIONS]
```

**Arguments:**
- `<WORKLOAD>` - Name of the workload to check

**Options:**
| Option | Description |
|--------|-------------|
| `-o, --output <FORMAT>` | Output format: table, json, yaml, html |
| `-f, --file <PATH>` | Write output to file |
| `--fail-fast` | Stop on first failure |
| `--packages-only` | Only check packages |
| `--files-only` | Only check files |
| `--assertions-only` | Only evaluate declarative assertions |
| `-s, --strict` | Treat warnings as errors |
| `--fix` | Attempt to install missing packages |
| `--update` | Update packages with available updates |
| `--no-cache` | Skip cache and query winget directly |
| `--show-diff` | Show file differences for modified files |

**Examples:**
```powershell
# Basic health check
anvil health rust-developer

# Detailed output
anvil -v health rust-developer

# Generate JSON report
anvil health rust-developer --output json --file health-report.json

# Generate HTML report
anvil health rust-developer --output html --file report.html

# Only check packages
anvil health rust-developer --packages-only

# Only run assertions
anvil health rust-developer --assertions-only

# Auto-fix missing packages
anvil health rust-developer --fix

# Show file diffs for modified config files
anvil health rust-developer --show-diff
```

**Understanding Health Reports:**

Health checks verify:
- **Packages**: Are required packages installed? Correct versions?
- **Files**: Do configuration files exist with expected content?
- **Assertions**: Do declarative condition checks pass? (see [Assertion-Based Health Checks](#assertion-based-health-checks))

Status indicators:
- ✓ (Green) - Check passed
- ✗ (Red) - Check failed
- ! (Yellow) - Warning or partial match

---

### `list`

List available workloads.

**Synopsis:**
```
anvil list [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `-a, --all` | Include built-in and custom workloads |
| `-l, --long` | Show detailed information |
| `--path <DIR>` | Search for workloads in additional path |
| `--all-paths` | Show all discovered paths including shadowed duplicates |
| `-o, --output <FORMAT>` | Output format: table, json, yaml, html |

**Examples:**
```powershell
# Simple list
anvil list

# Detailed list with versions and descriptions
anvil list --long

# JSON output for scripting
anvil list --output json

# List from custom directory
anvil list --path C:\MyWorkloads

# Show all paths including shadowed duplicates
anvil list --all-paths
```

---

### `show`

Display detailed information about a workload.

**Synopsis:**
```
anvil show <WORKLOAD> [OPTIONS]
```

**Arguments:**
- `<WORKLOAD>` - Name of the workload to display

**Options:**
| Option | Description |
|--------|-------------|
| `--show-inheritance` | Show inheritance hierarchy |
| `-r, --resolved` | Show fully resolved workload (after inheritance) |
| `-o, --output <FORMAT>` | Output format: yaml (default), json |

**Examples:**
```powershell
# Show workload details
anvil show rust-developer

# Show inheritance tree
anvil show rust-developer --show-inheritance

# Export as JSON
anvil show rust-developer --output json

# Show resolved (merged) workload
anvil show rust-developer --resolved
```

---

### `validate`

Validate workload syntax and structure.

**Synopsis:**
```
anvil validate <PATH> [OPTIONS]
```

**Arguments:**
- `<PATH>` - Path to workload.yaml file or workload directory

**Options:**
| Option | Description |
|--------|-------------|
| `--strict` | Enable strict validation mode |
| `--schema` | Output JSON schema for workload definitions |
| `--check-scripts` | Validate script syntax using PowerShell parser |
| `--scripts-only` | Only validate scripts (skip other validation) |

**Examples:**
```sh
# Basic validation
anvil validate my-workload

# Strict mode (treats warnings as errors)
anvil validate my-workload --strict
```

Validate all bundled workloads:

**PowerShell:**
```powershell
anvil list --output json | ConvertFrom-Json | ForEach-Object { anvil validate $_.name }
```

**Bash / Zsh:**
```bash
anvil list --output json | jq -r '.[].name' | xargs -I {} anvil validate {} --strict
```

**Common Validation Errors:**
- Missing required fields (`name`, `version`)
- Invalid workload name format
- Circular inheritance dependencies
- Invalid package IDs
- Non-existent script paths
- Invalid file paths

---

### `init`

Create a new workload from a template.

**Synopsis:**
```
anvil init <NAME> [OPTIONS]
```

**Arguments:**
- `<NAME>` - Name for the new workload

**Options:**
| Option | Description |
|--------|-------------|
| `-t, --template <NAME>` | Template to use: minimal, standard (default), full |
| `-e, --extends <PARENT>` | Parent workload to extend |
| `-o, --output <PATH>` | Output directory |

**Examples:**
```powershell
# Create a standard workload
anvil init my-workload

# Create minimal workload
anvil init my-workload --template minimal

# Create workload that extends essentials
anvil init my-rust-env --extends essentials

# Create in custom directory
anvil init my-workload --output C:\Workloads
```

**Available Templates:**
- `minimal` - Basic structure with required fields only
- `standard` - Common sections with sensible defaults (default)
- `full` - Complete example with all features

---

### `status`

Show current installation status.

**Synopsis:**
```
anvil status [WORKLOAD] [OPTIONS]
```

**Arguments:**
- `[WORKLOAD]` - Optional workload to check status for

**Options:**
| Option | Description |
|--------|-------------|
| `-o, --output <FORMAT>` | Output format: table, json, yaml, html |
| `-l, --long` | Show detailed status including timestamps |
| `--clear` | Clear stored state for the specified workload |

**Examples:**
```powershell
# Overall status
anvil status

# Status for specific workload
anvil status rust-developer
```

---

### `backup`

Manage system state backups.

**Synopsis:**
```
anvil backup <SUBCOMMAND>
```

**Subcommands:**

#### `backup create`
Create a new backup of current system state.

```powershell
# Create backup before changes
anvil backup create

# Create named backup
anvil backup create --name "before-update"

# Create backup for specific workload
anvil backup create --workload rust-developer
```

#### `backup list`
List available backups.

```powershell
anvil backup list
anvil backup list --output json
```

#### `backup show`
Show details of a specific backup.

```powershell
anvil backup show <BACKUP_ID>
```

#### `backup restore`
Restore from a backup.

```powershell
# Restore from backup
anvil backup restore <BACKUP_ID>

# Preview restore
anvil backup restore <BACKUP_ID> --dry-run

# Restore all backups for a workload
anvil backup restore --workload rust-developer
```

#### `backup clean`
Remove old backups.

```powershell
# Remove backups older than 30 days (default)
anvil backup clean

# Remove backups older than 7 days
anvil backup clean --older-than 7

# Preview what would be removed
anvil backup clean --dry-run
```

#### `backup verify`
Verify backup integrity.

```powershell
# Verify all backups
anvil backup verify

# Verify backups for a specific workload
anvil backup verify --workload rust-developer

# Fix issues by removing corrupted entries
anvil backup verify --fix
```

---

### `config`

Manage Anvil configuration.

**Synopsis:**
```
anvil config <SUBCOMMAND>
```

**Subcommands:**

#### `config get`
Get a specific configuration value.

```powershell
anvil config get defaults.shell
anvil config get workloads.paths
```

#### `config set`
Set a configuration value.

```powershell
anvil config set defaults.shell powershell
anvil config set defaults.output_format json
anvil config set backup.auto_backup true
```

#### `config list`
Display all configuration values.

```powershell
anvil config list
anvil config list --output json
```

#### `config reset`
Reset configuration to defaults.

```powershell
anvil config reset
anvil config reset --force  # Skip confirmation prompt
```

#### `config edit`
Open configuration file in default editor.

```powershell
anvil config edit
```

#### `config path`
Show the configuration file path.

```powershell
anvil config path
```

---

### `completions`

Generate shell completion scripts.

**Synopsis:**
```
anvil completions <SHELL>
```

**Arguments:**
- `<SHELL>` - Target shell: powershell, bash, zsh, fish, elvish

**Examples:**

**PowerShell:**
```powershell
anvil completions powershell | Out-String | Invoke-Expression

# Or save to a file and source it from $PROFILE
anvil completions powershell > $HOME\Documents\WindowsPowerShell\anvil.ps1
```

**Bash:**
```bash
anvil completions bash > ~/.local/share/bash-completion/completions/anvil
```

---

### Global Options

These options work with all commands:

| Option | Short | Description |
|--------|-------|-------------|
| `--verbose` | `-v` | Increase verbosity (use multiple times: -v, -vv, -vvv) |
| `--quiet` | `-q` | Suppress non-essential output |
| `--config <PATH>` | `-c` | Use custom configuration file |
| `--no-color` | | Disable colored output |
| `--help` | `-h` | Show help information |
| `--version` | `-V` | Show version information |

**Examples:**
```powershell
# Verbose output
anvil -v install rust-developer

# Very verbose (debug level)
anvil -vvv health rust-developer

# Quiet mode for scripting
anvil -q install rust-developer

# No colors (for log files)
anvil --no-color list > workloads.txt
```

---

## 5. Configuration

### Configuration File Location

Anvil stores its configuration at:
```
~/.anvil/config.yaml
```

On Windows this is typically `%USERPROFILE%\.anvil\config.yaml`.

You can also specify a custom config file with the `-c`/`--config` global flag:
```powershell
anvil -c C:\config\anvil.yaml list
```

### Configuration Options

```yaml
# Global Anvil Configuration (~/.anvil/config.yaml)

defaults:
  shell: powershell           # Default script shell
  script_timeout: 300         # Default script timeout in seconds
  output_format: table        # Default output format (table, json, yaml)
  color: auto                 # Color mode (auto, always, never)

backup:
  auto_backup: true           # Enable automatic backups before changes
  retention_days: 30          # Days to keep backups

install:
  parallel: false             # Run installations in parallel by default
  max_parallel: 4             # Max concurrent package installations

workloads:
  paths:                      # Additional workload search paths
    - "~/my-workloads"
    - "~/work/team-workloads"

logging:
  level: info                 # Log level: error, warn, info, debug, trace
```

### View Current Configuration

```powershell
anvil config list
```

### Modify Configuration

```powershell
# Set a value
anvil config set defaults.output_format json

# Get a value
anvil config get defaults.shell

# Reset to default
anvil config reset
```

---

## 6. Configuring Workload Search Paths

Anvil searches for workloads in multiple directories. You can add custom paths to include your own workloads alongside the built-in ones.

### Adding a Search Path

```powershell
anvil config set workloads.paths '["~/my-workloads", "/shared/team-workloads"]'
```

Or edit `~/.anvil/config.yaml` directly:

```yaml
workloads:
  paths:
    - "~/my-workloads"
    - "/shared/team-workloads"
```

### Search Order

Anvil resolves workloads in this priority order:

1. **Explicit path** — passed via `--path` flag
2. **User-configured** — paths from `~/.anvil/config.yaml`
3. **Default locations** — bundled workloads, local data directory, current directory

When the same workload name exists in multiple paths, the first match wins. Use `anvil list --all-paths` to see all discovered paths including shadowed duplicates.

### Complete Config Example

```yaml
# Anvil global configuration
# Location: ~/.anvil/config.yaml

workloads:
  paths:
    - "~/my-workloads"           # Personal workloads
    - "~/work/team-workloads"    # Team-shared workloads

logging:
  level: info
```

---

## 7. Working with Workloads

### Discovering Workloads

Anvil searches for workloads in these locations (in priority order):

1. Path specified with `--path` option
2. User-configured paths (from `~/.anvil/config.yaml`)
3. Default locations (bundled workloads, local data directory, current directory)

```powershell
# List all available workloads
anvil list

# List with details
anvil list --long

# List from specific directory
anvil list --path C:\MyWorkloads
```

### Understanding Workload Inheritance

Workloads can extend other workloads to inherit their configuration:

```yaml
name: my-rust-env
version: "1.0.0"
extends:
  - essentials        # Inherits packages, files, scripts
  - rust-developer    # Adds Rust-specific config
```

View the inheritance tree:
```powershell
anvil show my-rust-env --show-inheritance
```

Output:
```
my-rust-env
├── essentials
└── rust-developer
    └── essentials
```

### Using Custom Workload Directories

```powershell
# One-time use
anvil list --path C:\MyWorkloads
anvil install my-workload --path C:\MyWorkloads

# Configure permanently
anvil config set workloads.paths '["C:\\MyWorkloads"]'
```

### Validating Before Install

Always validate workloads before installation:

```powershell
# Validate syntax
anvil validate my-workload

# Strict validation
anvil validate my-workload --strict

# Preview installation
anvil install my-workload --dry-run
```

---

## 8. Output Formats

Anvil supports multiple output formats for different use cases.

### Table (Default)

Human-readable format for terminal display:

```powershell
anvil list
```
```
┌──────────────────┬─────────┬────────────────────────────────────────────────────┐
│ Name             │ Version │ Description                                        │
├──────────────────┼─────────┼────────────────────────────────────────────────────┤
│ essentials       │ 2.0.0   │ Core development tools and productivity utilities  │
│ rust-developer   │ 1.0.0   │ Rust development environment                       │
└──────────────────┴─────────┴────────────────────────────────────────────────────┘
```

### JSON

Machine-readable format for scripting and automation:

```powershell
anvil list --output json
```
```json
[
  {
    "name": "essentials",
    "version": "2.0.0",
    "description": "Core development tools and productivity utilities"
  },
  {
    "name": "rust-developer",
    "version": "1.0.0",
    "description": "Rust development environment"
  }
]
```

### YAML

Configuration-friendly format:

```powershell
anvil show rust-developer --output yaml
```
```yaml
name: rust-developer
version: "1.0.0"
description: Rust development environment
extends:
  - essentials
packages:
  winget:
    - id: Rustlang.Rustup
```

### HTML

Rich reports for documentation:

```powershell
anvil health rust-developer --output html --file report.html
```

Generates a styled HTML document with:
- Summary statistics
- Detailed check results
- Pass/fail indicators
- Timestamp and system info

---

## 9. Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANVIL_CONFIG` | Configuration file path | `~/.anvil/config.yaml` |
| `ANVIL_WORKLOADS` | Additional workload search paths | (none) |
| `ANVIL_LOG` | Log level: error, warn, info, debug, trace | `warn` |
| `NO_COLOR` | Disable colored output (any value) | (unset) |
| `ANVIL_BACKUP_DIR` | Backup storage directory | `~/.anvil/backups` |

**Examples:**

**PowerShell:**
```powershell
# Use custom config file
$env:ANVIL_CONFIG = "C:\config\anvil.yaml"
anvil list

# Add workload search paths
$env:ANVIL_WORKLOADS = "C:\Workloads;D:\MoreWorkloads"
anvil list

# Enable debug logging
$env:ANVIL_LOG = "debug"
anvil install rust-developer

# Disable colors
$env:NO_COLOR = "1"
anvil list
```

**Bash / Zsh:**
```bash
# Use custom config file
export ANVIL_CONFIG="$HOME/.config/anvil.yaml"
anvil list

# Add workload search paths
export ANVIL_WORKLOADS="$HOME/workloads:$HOME/more-workloads"
anvil list

# Enable debug logging
export ANVIL_LOG=debug
anvil install rust-developer

# Disable colors
export NO_COLOR=1
anvil list
```

---

## 10. Best Practices

### Always Dry-Run First

Before applying any workload, preview the changes:

```powershell
anvil install my-workload --dry-run
```

This shows what will happen without making changes.

### Use Health Checks Regularly

Verify your system state periodically:

```powershell
# Quick check
anvil health rust-developer

# Detailed report
anvil -v health rust-developer --output html --file health.html
```

### Keep Backups

Enable automatic backups in configuration:

```powershell
anvil config set backup.auto_backup true
```

Or create manual backups before major changes:

```powershell
anvil backup create --name "before-upgrade"
```

### Version Your Workloads

Store your workloads in version control:

```
my-workloads/
├── .git/
├── team-base/
│   └── workload.yaml
├── frontend-dev/
│   └── workload.yaml
└── backend-dev/
    └── workload.yaml
```

### Use Inheritance Wisely

Create a base workload with common tools:

```yaml
# team-base/workload.yaml
name: team-base
version: "1.0.0"
packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
```

Then extend it for specific roles:

```yaml
# frontend-dev/workload.yaml
name: frontend-dev
extends:
  - team-base
packages:
  winget:
    - id: OpenJS.NodeJS
```

### Validate Before Committing

Add validation to your CI/CD pipeline:

**PowerShell:**
```powershell
# Validate all workloads
Get-ChildItem -Directory | ForEach-Object {
    anvil validate $_.Name --strict
}
```

**Bash / Zsh:**
```bash
# Validate all workloads
for dir in */; do
    anvil validate "${dir%/}" --strict
done
```

### Use Verbose Output for Debugging

When things go wrong:

```sh
# Increase verbosity
anvil -vvv install my-workload
```

**PowerShell:**
```powershell
# Enable trace logging
$env:ANVIL_LOG = "trace"
anvil install my-workload
```

**Bash / Zsh:**
```bash
# Enable trace logging
export ANVIL_LOG=trace
anvil install my-workload
```

### Script Error Handling

In your workload scripts, handle errors gracefully:

```powershell
# post-install.ps1
try {
    $rustVersion = rustc --version
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Rust not installed"
        exit 1
    }
    Write-Host "Rust installed: $rustVersion"
    exit 0
}
catch {
    Write-Error "Script failed: $_"
    exit 1
}
```

---

## 11. Assertion-Based Health Checks

In addition to package and file checks, workloads can define **declarative assertions** using a built-in condition engine. Assertions let you express health checks directly in YAML without writing PowerShell scripts.

### Defining Assertions

Add an `assertions` section to your `workload.yaml`:

```yaml
name: my-workload
version: "1.0.0"
assertions:
  - name: "Git is installed"
    check:
      type: command_exists
      command: git

  - name: "Config directory exists"
    check:
      type: dir_exists
      path: "~/.config/my-app"

  - name: "GOPATH is set"
    check:
      type: env_var
      name: GOPATH

  - name: "Cargo in PATH"
    check:
      type: path_contains
      substring: ".cargo\\bin"
```

### Available Condition Types

| Type | Fields | Description |
|------|--------|-------------|
| `command_exists` | `command` | Check if a command is available on PATH |
| `file_exists` | `path` | Check if a file exists (`~` expanded) |
| `dir_exists` | `path` | Check if a directory exists (`~` expanded) |
| `env_var` | `name`, `value` (optional) | Check if env var is set, optionally matching a value |
| `path_contains` | `substring` | Check if PATH contains a substring |
| `registry_value` | `hive`, `key`, `name`, `expected` (optional) | Query a Windows registry value |
| `shell` | `command`, `description` (optional) | Run a shell command; passes if exit code is 0 |
| `all_of` | `conditions` | All child conditions must pass (logical AND) |
| `any_of` | `conditions` | At least one child must pass (logical OR) |

### Composing Conditions

Use `all_of` and `any_of` to build complex checks:

```yaml
assertions:
  - name: "Rust toolchain ready"
    check:
      type: all_of
      conditions:
        - type: command_exists
          command: rustc
        - type: command_exists
          command: cargo
        - type: path_contains
          substring: ".cargo\\bin"
```

### Running Assertions

Assertions are evaluated as part of `anvil health`:

```powershell
# Run all health checks including assertions
anvil health my-workload

# Run only assertions
anvil health my-workload --assertions-only
```

### Controlling Assertions in Health Config

You can disable assertion evaluation per-workload:

```yaml
health:
  assertion_check: false   # Skip assertions during health checks
```

---

## Getting Help

### Built-in Help

```powershell
# General help
anvil --help

# Command-specific help
anvil install --help
anvil health --help
```

### Resources

- [GitHub Repository](https://github.com/kafkade/anvil)
- [Workload Authoring Guide](WORKLOAD_AUTHORING.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)
- [Issue Tracker](https://github.com/kafkade/anvil/issues)

### Reporting Issues

When reporting issues, include:

1. Anvil version: `anvil --version`
2. Windows version: `winver`
3. Command that failed
4. Verbose output: `anvil -vvv <command>`
5. Relevant workload files (sanitized)

---

*This guide is for Anvil v0.3.1. For other versions, check the corresponding documentation.*