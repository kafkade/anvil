# Winforge User Guide

A comprehensive guide to using Winforge for Windows workstation configuration management.

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

### What is Winforge?

Winforge is a declarative configuration management tool for Windows workstations. It allows you to define your development environment in YAML files and automatically:

- Install software packages via Windows Package Manager (winget)
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

- Windows 10 (version 1809 or later) or Windows 11
- [Windows Package Manager (winget)](https://github.com/microsoft/winget-cli) version 1.4 or later
- PowerShell 5.1 or later (included with Windows)
- Administrator access (for some operations)

---

## 2. Installation

### Download Pre-built Binary

1. Download the latest release from the [Releases page](https://github.com/kafkade/anvil/releases)

2. Extract the archive:
   ```powershell
   Expand-Archive winforge-v0.3.1-windows-x64.zip -DestinationPath C:\Tools\winforge
   ```

3. Add to your PATH:
   ```powershell
   # Add to current session
   $env:PATH += ";C:\Tools\winforge"
   
   # Add permanently (User scope)
   [Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\Tools\winforge", "User")
   ```

### Build from Source

```powershell
# Prerequisites: Rust 1.75+ and Visual Studio Build Tools

# Clone the repository
git clone https://github.com/kafkade/anvil.git
cd winforge

# Build release binary
cargo build --release

# The binary is at target/release/winforge.exe
```

### Verify Installation

```powershell
winforge --version
# Output: winforge 0.3.1

winforge --help
# Shows available commands and options
```

### Shell Completions Setup

Generate and install shell completions for better command-line experience:

#### PowerShell

```powershell
# Generate completions
winforge completions powershell > $HOME\Documents\WindowsPowerShell\winforge.ps1

# Add to your PowerShell profile
Add-Content $PROFILE '. $HOME\Documents\WindowsPowerShell\winforge.ps1'
```

#### Bash (WSL/Git Bash)

```bash
# Generate completions
winforge completions bash > ~/.local/share/bash-completion/completions/winforge

# Or add to .bashrc
winforge completions bash >> ~/.bashrc
```

#### Zsh

```zsh
# Generate completions
winforge completions zsh > ~/.zfunc/_winforge

# Add to .zshrc (before compinit)
fpath+=~/.zfunc
```

---

## 3. Quick Start

### List Available Workloads

See what workloads are available:

```powershell
winforge list
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
winforge show rust-developer
```

### Dry Run Installation

Preview what would happen without making changes:

```powershell
winforge install rust-developer --dry-run
```

### Install a Workload

Apply a workload configuration:

```powershell
winforge install rust-developer
```

### Check System Health

Verify your system matches the workload definition:

```powershell
winforge health rust-developer
```

---

## 4. Command Reference

### `install`

Apply a workload configuration to your system.

**Synopsis:**
```
winforge install <WORKLOAD> [OPTIONS]
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
winforge install rust-developer

# Preview only
winforge install rust-developer --dry-run

# Skip packages (only copy files and run scripts)
winforge install rust-developer --skip-packages

# Use JSON output for scripting
winforge install rust-developer --dry-run --output json

# Install from custom directory
winforge install my-workload --path C:\Workloads
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
winforge health <WORKLOAD> [OPTIONS]
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
winforge health rust-developer

# Detailed output
winforge health rust-developer --verbose

# Generate JSON report
winforge health rust-developer --output json --file health-report.json

# Generate HTML report
winforge health rust-developer --output html --file report.html
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
winforge list [OPTIONS]
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
winforge list

# Detailed list with versions and descriptions
winforge list --long

# JSON output for scripting
winforge list --output json

# List from custom directory
winforge list --path C:\MyWorkloads
```

---

### `show`

Display detailed information about a workload.

**Synopsis:**
```
winforge show <WORKLOAD> [OPTIONS]
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
winforge show rust-developer

# Show inheritance tree
winforge show rust-developer --inheritance-tree

# Export as YAML
winforge show rust-developer --output yaml

# Show resolved (merged) workload
winforge show rust-developer --resolved
```

---

### `validate`

Validate workload syntax and structure.

**Synopsis:**
```
winforge validate <WORKLOAD> [OPTIONS]
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
winforge validate my-workload

# Strict mode (treats warnings as errors)
winforge validate my-workload --strict

# Validate all bundled workloads
winforge list --output json | ConvertFrom-Json | ForEach-Object { winforge validate $_.name }
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
winforge init <PATH> [OPTIONS]
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
winforge init C:\Workloads\my-workload

# Create from template
winforge init C:\Workloads\my-rust-env --template rust

# Overwrite existing
winforge init C:\Workloads\existing --force
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
winforge status [WORKLOAD] [OPTIONS]
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
winforge status

# Status for specific workload
winforge status rust-developer
```

---

### `backup`

Manage system state backups.

**Synopsis:**
```
winforge backup <SUBCOMMAND>
```

**Subcommands:**

#### `backup create`
Create a new backup of current system state.

```powershell
# Create backup before changes
winforge backup create

# Create named backup
winforge backup create --name "before-update"

# Create backup for specific workload
winforge backup create --workload rust-developer
```

#### `backup list`
List available backups.

```powershell
winforge backup list
winforge backup list --output json
```

#### `backup show`
Show details of a specific backup.

```powershell
winforge backup show <BACKUP_ID>
```

#### `backup restore`
Restore from a backup.

```powershell
# Restore from backup
winforge backup restore <BACKUP_ID>

# Preview restore
winforge backup restore <BACKUP_ID> --dry-run
```

#### `backup delete`
Delete a backup.

```powershell
winforge backup delete <BACKUP_ID>
winforge backup delete <BACKUP_ID> --force
```

---

### `config`

Manage Winforge configuration.

**Synopsis:**
```
winforge config <SUBCOMMAND>
```

**Subcommands:**

#### `config show`
Display current configuration.

```powershell
winforge config show
winforge config show --output json
```

#### `config set`
Set a configuration value.

```powershell
winforge config set workload_paths "C:\Workloads;D:\MoreWorkloads"
winforge config set default_output json
winforge config set backup.enabled true
```

#### `config reset`
Reset configuration to defaults.

```powershell
winforge config reset
winforge config reset --key workload_paths
```

#### `config edit`
Open configuration file in editor.

```powershell
winforge config edit
```

---

### `completions`

Generate shell completion scripts.

**Synopsis:**
```
winforge completions <SHELL>
```

**Arguments:**
- `<SHELL>` - Target shell: powershell, bash, zsh, fish

**Examples:**
```powershell
# Generate PowerShell completions
winforge completions powershell

# Generate and install bash completions
winforge completions bash > /etc/bash_completion.d/winforge
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
winforge -v install rust-developer

# Very verbose (debug level)
winforge -vvv health rust-developer

# Quiet mode for scripting
winforge -q install rust-developer

# No colors (for log files)
winforge --no-color list > workloads.txt
```

---

## 5. Configuration

### Configuration File Location

Winforge stores its configuration at:
```
%APPDATA%\winforge\config.toml
```

Or if `WINFORGE_CONFIG` is set:
```
$env:WINFORGE_CONFIG
```

### Configuration Options

```toml
# Global Winforge Configuration

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
path = "%APPDATA%\\winforge\\backups"

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
winforge config show
```

### Modify Configuration

```powershell
# Set a value
winforge config set default_output json

# Reset to default
winforge config reset
```

---

## 6. Working with Workloads

### Discovering Workloads

Winforge searches for workloads in these locations (in order):

1. Bundled workloads (included with Winforge)
2. Custom paths specified in configuration
3. Path specified with `--path` option

```powershell
# List all available workloads
winforge list

# List with details
winforge list --long

# List from specific directory
winforge list --path C:\MyWorkloads
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
winforge show my-rust-env --inheritance-tree
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
winforge list --path C:\MyWorkloads
winforge install my-workload --path C:\MyWorkloads

# Configure permanently
winforge config set workload_paths "C:\MyWorkloads"
```

### Validating Before Install

Always validate workloads before installation:

```powershell
# Validate syntax
winforge validate my-workload

# Strict validation
winforge validate my-workload --strict

# Preview installation
winforge install my-workload --dry-run
```

---

## 7. Output Formats

Winforge supports multiple output formats for different use cases.

### Table (Default)

Human-readable format for terminal display:

```powershell
winforge list
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
winforge list --output json
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
winforge show rust-developer --output yaml
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
winforge health rust-developer --output html --file report.html
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
| `WINFORGE_CONFIG` | Configuration file path | `%APPDATA%\winforge\config.toml` |
| `WINFORGE_WORKLOADS` | Additional workload search paths | (none) |
| `WINFORGE_LOG` | Log level: error, warn, info, debug, trace | `warn` |
| `NO_COLOR` | Disable colored output (any value) | (unset) |
| `WINFORGE_BACKUP_DIR` | Backup storage directory | `%APPDATA%\winforge\backups` |

**Examples:**

```powershell
# Use custom config file
$env:WINFORGE_CONFIG = "C:\config\winforge.toml"
winforge list

# Add workload search paths
$env:WINFORGE_WORKLOADS = "C:\Workloads;D:\MoreWorkloads"
winforge list

# Enable debug logging
$env:WINFORGE_LOG = "debug"
winforge install rust-developer

# Disable colors
$env:NO_COLOR = "1"
winforge list
```

---

## 9. Best Practices

### Always Dry-Run First

Before applying any workload, preview the changes:

```powershell
winforge install my-workload --dry-run
```

This shows what will happen without making changes.

### Use Health Checks Regularly

Verify your system state periodically:

```powershell
# Quick check
winforge health rust-developer

# Detailed report
winforge health rust-developer --verbose --output html --file health.html
```

### Keep Backups

Enable automatic backups in configuration:

```powershell
winforge config set backup.enabled true
```

Or create manual backups before major changes:

```powershell
winforge backup create --name "before-upgrade"
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
    winforge validate $_.Name --strict
}
```

### Use Verbose Output for Debugging

When things go wrong:

```powershell
# Increase verbosity
winforge -vvv install my-workload

# Enable trace logging
$env:WINFORGE_LOG = "trace"
winforge install my-workload
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
winforge --help

# Command-specific help
winforge install --help
winforge health --help
```

### Resources

- [GitHub Repository](https://github.com/kafkade/anvil)
- [Workload Authoring Guide](WORKLOAD_AUTHORING.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)
- [Issue Tracker](https://github.com/kafkade/anvil/issues)

### Reporting Issues

When reporting issues, include:

1. Winforge version: `winforge --version`
2. Windows version: `winver`
3. Command that failed
4. Verbose output: `winforge -vvv <command>`
5. Relevant workload files (sanitized)

---

*This guide is for Winforge v0.3.1. For other versions, check the corresponding documentation.*