# Anvil User Guide

A comprehensive guide to using Anvil for workstation configuration management.

## Table of Contents

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [Quick Start](#3-quick-start)
4. [Command Reference](#4-command-reference)
5. [Configuration](#5-configuration)
6. [Working with Workloads](#6-working-with-workloads)
7. [Output Formats](#7-output-formats)
8. [Environment Variables](#8-environment-variables)
9. [Best Practices](#9-best-practices)

---

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

### Download Pre-built Binary

1. Download the latest release from the [Releases page](https://github.com/kafkade/anvil/releases)

2. Extract the archive:
   ```powershell
   Expand-Archive anvil-v0.3.1-windows-x64.zip -DestinationPath C:\Tools\anvil
   ```

3. Add to your PATH:
   ```powershell
   # Add to current session
   $env:PATH += ";C:\Tools\anvil"
   
   # Add permanently (User scope)
   [Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\Tools\anvil", "User")
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
| `--force` | Force reinstallation of packages |
| `--skip-packages` | Skip package installation |
| `--skip-files` | Skip file operations |
| `--skip-scripts` | Skip script execution |
| `--output <FORMAT>` | Output format: table, json, yaml |
| `--path <DIR>` | Custom workload search path |

**Examples:**
```powershell
# Standard installation
anvil install rust-developer

# Preview only
anvil install rust-developer --dry-run

# Skip packages (only copy files and run scripts)
anvil install rust-developer --skip-packages

# Use JSON output for scripting
anvil install rust-developer --dry-run --output json

# Install from custom directory
anvil install my-workload --path C:\Workloads
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
| `--output <FORMAT>` | Output format: table, json, yaml, html |
| `--file <PATH>` | Write output to file |
| `--verbose` | Show detailed check results |
| `--path <DIR>` | Custom workload search path |

**Examples:**
```powershell
# Basic health check
anvil health rust-developer

# Detailed output
anvil health rust-developer --verbose

# Generate JSON report
anvil health rust-developer --output json --file health-report.json

# Generate HTML report
anvil health rust-developer --output html --file report.html
```

**Understanding Health Reports:**

Health checks verify:
- **Packages**: Are required packages installed? Correct versions?
- **Files**: Do configuration files exist with expected content?
- **Scripts**: Do health check scripts pass?

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
| `--all` | Include hidden/system workloads |
| `--long` | Show detailed information |
| `--path <DIR>` | Custom workload search path |
| `--output <FORMAT>` | Output format: table, json, yaml |

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
| `--inheritance-tree` | Show inheritance hierarchy |
| `--resolved` | Show fully resolved workload (after inheritance) |
| `--output <FORMAT>` | Output format: table, json, yaml |
| `--path <DIR>` | Custom workload search path |

**Examples:**
```powershell
# Show workload details
anvil show rust-developer

# Show inheritance tree
anvil show rust-developer --inheritance-tree

# Export as YAML
anvil show rust-developer --output yaml

# Show resolved (merged) workload
anvil show rust-developer --resolved
```

---

### `validate`

Validate workload syntax and structure.

**Synopsis:**
```
anvil validate <WORKLOAD> [OPTIONS]
```

**Arguments:**
- `<WORKLOAD>` - Name of the workload to validate

**Options:**
| Option | Description |
|--------|-------------|
| `--strict` | Enable strict validation mode |
| `--output <FORMAT>` | Output format: table, json, yaml |
| `--path <DIR>` | Custom workload search path |

**Examples:**
```powershell
# Basic validation
anvil validate my-workload

# Strict mode (treats warnings as errors)
anvil validate my-workload --strict

# Validate all bundled workloads
anvil list --output json | ConvertFrom-Json | ForEach-Object { anvil validate $_.name }
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
anvil init <PATH> [OPTIONS]
```

**Arguments:**
- `<PATH>` - Directory path for the new workload

**Options:**
| Option | Description |
|--------|-------------|
| `--template <NAME>` | Template to use: minimal, full, rust, python |
| `--force` | Overwrite existing files |
| `--name <NAME>` | Workload name (defaults to directory name) |

**Examples:**
```powershell
# Create minimal workload
anvil init C:\Workloads\my-workload

# Create from template
anvil init C:\Workloads\my-rust-env --template rust

# Overwrite existing
anvil init C:\Workloads\existing --force
```

**Available Templates:**
- `minimal` - Basic structure with required fields only
- `full` - Complete example with all features
- `rust` - Rust development environment template
- `python` - Python development environment template

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
| `--output <FORMAT>` | Output format: table, json, yaml |

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
```

#### `backup delete`
Delete a backup.

```powershell
anvil backup delete <BACKUP_ID>
anvil backup delete <BACKUP_ID> --force
```

---

### `config`

Manage Anvil configuration.

**Synopsis:**
```
anvil config <SUBCOMMAND>
```

**Subcommands:**

#### `config show`
Display current configuration.

```powershell
anvil config show
anvil config show --output json
```

#### `config set`
Set a configuration value.

```powershell
anvil config set workload_paths "C:\Workloads;D:\MoreWorkloads"
anvil config set default_output json
anvil config set backup.enabled true
```

#### `config reset`
Reset configuration to defaults.

```powershell
anvil config reset
anvil config reset --key workload_paths
```

#### `config edit`
Open configuration file in editor.

```powershell
anvil config edit
```

---

### `completions`

Generate shell completion scripts.

**Synopsis:**
```
anvil completions <SHELL>
```

**Arguments:**
- `<SHELL>` - Target shell: powershell, bash, zsh, fish

**Examples:**
```powershell
# Generate PowerShell completions
anvil completions powershell

# Generate and install bash completions
anvil completions bash > /etc/bash_completion.d/anvil
```

---

### Global Options

These options work with all commands:

| Option | Short | Description |
|--------|-------|-------------|
| `--verbose` | `-v` | Increase verbosity (use multiple times: -v, -vv, -vvv) |
| `--quiet` | `-q` | Suppress non-essential output |
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
%APPDATA%\anvil\config.toml
```

Or if `ANVIL_CONFIG` is set:
```
$env:ANVIL_CONFIG
```

### Configuration Options

```toml
# Global Anvil Configuration

# Default output format (table, json, yaml)
default_output = "table"

# Workload search paths (semicolon-separated)
workload_paths = "C:\\Workloads;D:\\MyWorkloads"

# Enable colored output
color = true

# Default verbosity level (0-3)
verbosity = 0

[backup]
# Enable automatic backups before changes
enabled = true

# Backup directory
path = "%APPDATA%\\anvil\\backups"

# Maximum number of backups to keep
max_count = 10

[packages]
# Default package source
default_source = "winget"

# Allow prerelease versions
allow_prerelease = false

[scripts]
# Default script timeout (seconds)
default_timeout = 300

# Shell for script execution
default_shell = "powershell"
```

### View Current Configuration

```powershell
anvil config show
```

### Modify Configuration

```powershell
# Set a value
anvil config set default_output json

# Reset to default
anvil config reset
```

---

## 6. Working with Workloads

### Discovering Workloads

Anvil searches for workloads in these locations (in order):

1. Bundled workloads (included with Anvil)
2. Custom paths specified in configuration
3. Path specified with `--path` option

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
anvil show my-rust-env --inheritance-tree
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
anvil config set workload_paths "C:\MyWorkloads"
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

## 7. Output Formats

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

## 8. Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANVIL_CONFIG` | Configuration file path | `%APPDATA%\anvil\config.toml` |
| `ANVIL_WORKLOADS` | Additional workload search paths | (none) |
| `ANVIL_LOG` | Log level: error, warn, info, debug, trace | `warn` |
| `NO_COLOR` | Disable colored output (any value) | (unset) |
| `ANVIL_BACKUP_DIR` | Backup storage directory | `%APPDATA%\anvil\backups` |

**Examples:**

```powershell
# Use custom config file
$env:ANVIL_CONFIG = "C:\config\anvil.toml"
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

---

## 9. Best Practices

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
anvil health rust-developer --verbose --output html --file health.html
```

### Keep Backups

Enable automatic backups in configuration:

```powershell
anvil config set backup.enabled true
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

```powershell
# Validate all workloads
Get-ChildItem -Directory | ForEach-Object {
    anvil validate $_.Name --strict
}
```

### Use Verbose Output for Debugging

When things go wrong:

```powershell
# Increase verbosity
anvil -vvv install my-workload

# Enable trace logging
$env:ANVIL_LOG = "trace"
anvil install my-workload
```

### Script Error Handling

In your workload scripts, handle errors gracefully:

```powershell
# health-check.ps1
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
    Write-Error "Health check failed: $_"
    exit 1
}
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