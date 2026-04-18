# Workload Authoring

A comprehensive guide to creating custom workloads for Anvil.

## 1. Workload Structure

A workload is a directory containing configuration files, scripts, and assets that define a system configuration.

### Directory Structure

```
workload-name/
├── workload.yaml       # Required: workload definition
├── files/              # Optional: files to deploy
│   ├── .config/
│   │   └── app.conf
│   └── settings.json
└── scripts/            # Optional: installation/health scripts
    ├── pre-install.ps1
    ├── post-install.ps1
    └── health-check.ps1
```

### Required Files

- **workload.yaml**: The main workload definition file (required)

### Optional Directories

- **files/**: Contains configuration files to be copied to the target system
- **scripts/**: Contains PowerShell or CMD scripts for installation and validation

### Naming Conventions

- Workload directory names should be lowercase with hyphens: `my-workload-name`
- Use descriptive names that indicate the workload's purpose
- Avoid spaces and special characters

---

## 2. Schema Reference

The `workload.yaml` file defines all aspects of your workload configuration.

### Complete Schema

```yaml
# Required fields
name: string                    # Workload identifier
version: string                 # Semantic version (e.g., "1.0.0")

# Optional fields
description: string             # Human-readable description
extends: string[]               # Parent workloads to inherit from
packages: object                # Package definitions
files: array                    # File deployment definitions
scripts: object                 # Script execution definitions
commands: object                # Inline command definitions
environment: object             # Environment variable configuration
assertions: array               # Declarative health assertions
health: object                  # Health check configuration
```

### Required Fields

#### `name`
Unique identifier for the workload. Must be alphanumeric with hyphens and underscores only.

```yaml
name: my-workload           # Valid
name: my_workload_v2        # Valid
name: "my workload"         # Invalid (spaces)
name: my.workload           # Invalid (dots)
```

#### `version`
Semantic version string. Recommended to follow [SemVer](https://semver.org/).

```yaml
version: "1.0.0"
version: "2.1.0-beta"
version: "0.1.0"
```

### Optional Fields

#### `description`
Human-readable description displayed in listings.

```yaml
description: "Development environment for Rust projects with debugging tools"
```

#### `extends`
List of parent workloads to inherit from.

```yaml
extends:
  - essentials
  - rust-developer
```

#### `packages`
Package installation definitions (see [Package Definitions](#3-package-definitions)).

#### `files`
File deployment definitions (see [File Definitions](#4-file-definitions)).

#### `scripts`
Script execution definitions (see [Script Definitions](#5-script-definitions)).

#### `commands`
Inline command definitions (see [Commands (Inline)](#6-commands-inline)).

#### `environment`
Environment variable configuration (see [Environment Configuration](#7-environment-configuration)).

---

## 3. Package Definitions

Define software packages to install. Anvil supports multiple package managers: **winget** (Windows), **brew** (macOS/Linux), and **apt** (Debian/Ubuntu).

### Basic Structure

```yaml
packages:
  winget:
    - id: Publisher.PackageName
    - id: Another.Package
```

### Full Package Options

```yaml
packages:
  winget:
    - id: Git.Git
      version: "2.43.0"           # Optional: pin to specific version
      source: winget              # Optional: winget, msstore
      override:                    # Optional: additional winget arguments
        - "--scope"
        - "machine"
```

### Package Fields

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Winget package identifier |
| `version` | No | Specific version to install |
| `source` | No | Package source: `winget` or `msstore` |
| `override` | No | Additional arguments passed to winget |

### Finding Package IDs

Use winget to search for packages:

```powershell
# Search for packages
winget search vscode

# Get exact ID
winget search --exact "Visual Studio Code"

# Show package details
winget show Microsoft.VisualStudioCode
```

Common package IDs:

| Package | ID |
|---------|-----|
| Visual Studio Code | `Microsoft.VisualStudioCode` |
| Git | `Git.Git` |
| Windows Terminal | `Microsoft.WindowsTerminal` |
| Node.js | `OpenJS.NodeJS` |
| Python | `Python.Python.3.12` |
| Rust | `Rustlang.Rustup` |
| PowerShell | `Microsoft.PowerShell` |

### Version Pinning

Pin specific versions when compatibility matters:

```yaml
packages:
  winget:
    # Pin exact version
    - id: Python.Python.3.12
      version: "3.12.0"
    
    # Use latest (default)
    - id: Git.Git
```

Check available versions:

```powershell
winget show Python.Python.3.12 --versions
```

### Override Arguments

Pass custom arguments to winget:

```yaml
packages:
  winget:
    - id: Microsoft.VisualStudioCode
      override:
        - "--scope"
        - "machine"           # Install for all users
        - "--override"
        - "/SILENT"
```

### Multi-Manager Packages

Define packages for multiple package managers in a single workload. Anvil selects the appropriate manager for the current platform.

```yaml
packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
  brew:
    - name: git
    - name: visual-studio-code
      cask: true
  apt:
    - name: git
    - name: build-essential
```

### Homebrew Packages (macOS/Linux)

```yaml
packages:
  brew:
    - name: git                        # CLI formula
    - name: visual-studio-code         # GUI cask
      cask: true
    - name: font-cascadia-code         # Cask from a tap
      cask: true
      tap: "homebrew/cask-fonts"
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | - | Homebrew formula or cask name |
| `cask` | No | `false` | Whether this is a cask (GUI app) vs formula (CLI tool) |
| `tap` | No | - | Tap source (e.g., `"homebrew/cask-fonts"`) |

### APT Packages (Debian/Ubuntu)

```yaml
packages:
  apt:
    - name: git
    - name: build-essential
      version: "12.9"
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | - | APT package name |
| `version` | No | - | Specific version constraint |

> **Note:** Homebrew and APT support is schema-complete but not yet fully implemented. Winget is the primary supported manager today.

---

## 4. File Definitions

Define configuration files to copy to the target system.

### Basic Structure

```yaml
files:
  - source: config.json
    destination: "~/.config/app/config.json"
```

### Full File Options

```yaml
files:
  - source: relative/path/in/workload/file.conf
    destination: "~/target/path/file.conf"
    backup: true                  # Backup existing file
    permissions: "0644"           # Optional: file permissions
    template: false               # Process as Handlebars template
```

### File Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source` | Yes | - | Path relative to workload directory |
| `destination` | Yes | - | Target path on system |
| `backup` | No | `true` | Backup existing files before overwriting |
| `permissions` | No | - | File permissions (Unix-style, informational on Windows) |
| `template` | No | `false` | Process file as Handlebars template |

### Path Variables

Use these variables in destination paths:

| Variable | Description | Example |
|----------|-------------|---------|
| `~` | User home directory | `C:\Users\username` |
| `${HOME}` | User home directory | `C:\Users\username` |
| `${USERPROFILE}` | User profile directory | `C:\Users\username` |
| `${APPDATA}` | Application data | `C:\Users\username\AppData\Roaming` |
| `${LOCALAPPDATA}` | Local app data | `C:\Users\username\AppData\Local` |
| `${WORKLOAD_DIR}` | Workload directory | Path to current workload |

Examples:

```yaml
files:
  # Home directory
  - source: .gitconfig
    destination: "~/.gitconfig"
  
  # AppData
  - source: settings.json
    destination: "${APPDATA}/MyApp/settings.json"
  
  # Local AppData
  - source: cache.db
    destination: "${LOCALAPPDATA}/MyApp/cache.db"
```

### Templating

Enable Handlebars templating for dynamic content:

```yaml
files:
  - source: config.toml.hbs
    destination: "~/.config/app/config.toml"
    template: true
```

Template file (`config.toml.hbs`):

```toml
# Configuration for {{username}}
# Generated on {{date}}

[user]
name = "{{username}}"
home = "{{home}}"

[paths]
workload = "{{workload_dir}}"
```

Available template variables:

| Variable | Description |
|----------|-------------|
| `{{username}}` | Current username |
| `{{home}}` | Home directory path |
| `{{computername}}` | Computer name |
| `{{workload_dir}}` | Workload directory |
| `{{workload_name}}` | Workload name |
| `{{date}}` | Current date |
| `{{env.VAR_NAME}}` | Environment variable |

### Directory Copying

Copy entire directories:

```yaml
files:
  - source: config/
    destination: "~/.config/myapp/"
```

---

## 5. Script Definitions

Define PowerShell or CMD scripts for installation steps and health checks.

### Script Categories

```yaml
scripts:
  pre_install:     # Run before package installation
    - path: scripts/pre-install.ps1
    
  post_install:    # Run after package installation
    - path: scripts/post-install.ps1
```

### Full Script Options

```yaml
scripts:
  post_install:
    - path: scripts/setup.ps1
      shell: powershell           # powershell, pwsh, cmd
      description: "Configure application settings"
      elevated: false             # Run as administrator
      timeout: 300                # Timeout in seconds
```

### Script Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `path` | Yes | - | Path relative to workload directory |
| `shell` | No | `powershell` | Shell: `powershell`, `pwsh`, `cmd` |
| `description` | No | - | Human-readable description |
| `elevated` | No | `false` | Run with administrator privileges |
| `timeout` | No | `300` | Timeout in seconds |

> **Note:** `scripts.health_check` was removed in v1.0. Use declarative `assertions` instead.

### Script Guidelines

#### Exit Codes

- `0` - Success
- Non-zero - Failure

```powershell
# Good: explicit exit codes
if (Test-Path $requiredFile) {
    exit 0
} else {
    Write-Error "Required file not found"
    exit 1
}
```

#### Output Handling

- Use `Write-Host` for informational output
- Use `Write-Error` for error messages
- Use `Write-Warning` for warnings

```powershell
Write-Host "Installing components..."
Write-Warning "This may take a while"
Write-Error "Installation failed"
```

#### Idempotency

Scripts should be safe to run multiple times:

```powershell
# Good: check before acting
if (-not (Test-Path "C:\Tools\mytool")) {
    Write-Host "Installing mytool..."
    # Install logic here
} else {
    Write-Host "mytool already installed"
}
```

#### Error Handling

Use try-catch for robust error handling:

```powershell
try {
    # Risky operation
    Invoke-WebRequest -Uri $url -OutFile $path
    exit 0
}
catch {
    Write-Error "Failed to download: $_"
    exit 1
}
```

### Sample Scripts

#### Pre-Install Check

```powershell
# scripts/pre-install.ps1
# Check prerequisites before installation

$ErrorActionPreference = "Stop"

# Check Windows version
$version = [Environment]::OSVersion.Version
if ($version.Major -lt 10) {
    Write-Error "Windows 10 or later required"
    exit 1
}

# Check available disk space (need 1GB)
$drive = Get-PSDrive C
$freeGB = [math]::Round($drive.Free / 1GB, 2)
if ($freeGB -lt 1) {
    Write-Error "Insufficient disk space. Need 1GB, have ${freeGB}GB"
    exit 1
}

Write-Host "Prerequisites check passed"
exit 0
```

#### Post-Install Configuration

```powershell
# scripts/post-install.ps1
# Configure application after installation

$ErrorActionPreference = "Stop"

try {
    # Install Rust components
    Write-Host "Installing Rust components..."
    rustup component add rustfmt
    rustup component add clippy
    
    # Install common cargo tools
    Write-Host "Installing cargo tools..."
    cargo install cargo-watch
    cargo install cargo-edit
    
    Write-Host "Post-install configuration complete"
    exit 0
}
catch {
    Write-Error "Post-install failed: $_"
    exit 1
}
```

#### Health Check

```powershell
# scripts/health-check.ps1
# Verify Rust development environment

$ErrorActionPreference = "Stop"
$errors = @()

# Check rustc
try {
    $rustVersion = rustc --version
    Write-Host "✓ Rust compiler: $rustVersion"
}
catch {
    $errors += "rustc not found"
}

# Check cargo
try {
    $cargoVersion = cargo --version
    Write-Host "✓ Cargo: $cargoVersion"
}
catch {
    $errors += "cargo not found"
}

# Check rustfmt
try {
    $null = rustfmt --version
    Write-Host "✓ rustfmt installed"
}
catch {
    $errors += "rustfmt not installed"
}

# Report results
if ($errors.Count -gt 0) {
    Write-Error "Health check failed:"
    $errors | ForEach-Object { Write-Error "  - $_" }
    exit 1
}

Write-Host "All health checks passed"
exit 0
```

---

## 6. Commands (Inline)

Commands let you run arbitrary shell commands directly from `workload.yaml` without creating separate script files. They support conditional execution via the same predicate engine used by [Assertions](#8-assertions).

Use commands for simple one-liners (e.g., `cargo install`, `npm install -g`). Use scripts for complex multi-step logic that benefits from a dedicated file.

### Basic Structure

```yaml
commands:
  pre_install:
    - run: "echo Preparing environment"
  post_install:
    - run: "cargo install ripgrep"
      description: "Install ripgrep via cargo"
```

### Full Command Options

```yaml
commands:
  post_install:
    - run: "cargo install cargo-watch"
      description: "Install cargo-watch"
      timeout: 600                    # Timeout in seconds (default: 300)
      elevated: false                 # Require admin privileges (default: false)
      continue_on_error: false        # Continue if this command fails (default: false)
      when:                           # Condition — skip if not met
        type: command_exists
        command: cargo
```

### Command Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `run` | Yes | - | Shell command string to execute |
| `description` | No | - | Human-readable description |
| `timeout` | No | `300` | Timeout in seconds |
| `elevated` | No | `false` | Require administrator privileges |
| `continue_on_error` | No | `false` | Continue to next command if this one fails |
| `when` | No | - | Condition predicate; command is skipped when not met |

### Conditional Execution

The `when` field accepts any condition type from the [Assertions](#8-assertions) predicate engine:

```yaml
commands:
  post_install:
    # Only runs if cargo is on PATH
    - run: "cargo install sccache"
      description: "Install sccache"
      when:
        type: command_exists
        command: cargo

    # Only runs if the config file doesn't already exist
    - run: "echo '{}' > ~/.config/app/config.json"
      description: "Create default config"
      when:
        type: file_exists
        path: "~/.config/app/config.json"
```

### Command Phases

| Phase | When it runs |
|-------|-------------|
| `pre_install` | Before package installation |
| `post_install` | After package installation |

### Error Handling

By default, if a command fails (non-zero exit code), execution stops and subsequent commands are skipped. Set `continue_on_error: true` to keep going:

```yaml
commands:
  post_install:
    - run: "cargo install cargo-watch"
      continue_on_error: true        # Failure won't stop the next command
    - run: "cargo install cargo-edit"
```

---

## 7. Environment Configuration

Configure environment variables and PATH additions.

### Basic Structure

```yaml
environment:
  variables:
    - name: MY_VARIABLE
      value: "my-value"
      scope: user
      
  path_additions:
    - "C:\\Tools\\bin"
    - "~\\.local\\bin"
```

### Environment Variables

```yaml
environment:
  variables:
    - name: RUST_BACKTRACE
      value: "1"
      scope: user              # user or machine
      
    - name: EDITOR
      value: "code"
      scope: user
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | - | Variable name |
| `value` | Yes | - | Variable value |
| `scope` | No | `user` | Scope: `user` or `machine` |

### PATH Additions

Add directories to the system PATH:

```yaml
environment:
  path_additions:
    - "C:\\Tools\\bin"
    - "~\\.cargo\\bin"
    - "${LOCALAPPDATA}\\Programs\\bin"
```

PATH additions are appended to the existing PATH for the specified scope.

### Scope

| Scope | Description | Requires Admin |
|-------|-------------|----------------|
| `user` | Current user only | No |
| `machine` | All users on system | Yes |

---

## 8. Assertions

Assertions are declarative health checks defined directly in `workload.yaml`. They let you validate system state — such as installed commands, existing files, environment variables, and PATH entries — without writing PowerShell scripts.

Use assertions when your checks are simple conditions (command exists, file exists, env var set). Use health check scripts for complex validation that requires multi-step logic or custom output.

### Assertion Structure

Each assertion has a `name` and a `check` that specifies a condition:

```yaml
assertions:
  - name: Git is installed
    check:
      type: command_exists
      command: git
```

### Condition Types

#### `command_exists`

Checks whether a command is available on `PATH`.

```yaml
- name: cargo is available
  check:
    type: command_exists
    command: cargo
```

#### `file_exists`

Checks whether a file exists at the given path. Supports `~` expansion.

```yaml
- name: Git config exists
  check:
    type: file_exists
    path: "~/.gitconfig"
```

#### `dir_exists`

Checks whether a directory exists at the given path. Supports `~` expansion.

```yaml
- name: Cargo directory exists
  check:
    type: dir_exists
    path: "~/.cargo"
```

#### `env_var`

Checks whether an environment variable is set, optionally matching a specific value.

```yaml
# Check existence only
- name: RUST_BACKTRACE is set
  check:
    type: env_var
    name: RUST_BACKTRACE

# Check existence and value
- name: RUST_BACKTRACE is 1
  check:
    type: env_var
    name: RUST_BACKTRACE
    value: "1"
```

#### `path_contains`

Checks whether the system `PATH` contains a given substring.

```yaml
- name: Cargo bin on PATH
  check:
    type: path_contains
    substring: ".cargo/bin"
```

#### `registry_value`

Queries a Windows registry value under `HKCU` or `HKLM`. If `expected` is omitted, the check only asserts the value exists.

```yaml
- name: Developer mode enabled
  check:
    type: registry_value
    hive: HKLM
    key: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock"
    name: AllowDevelopmentWithoutDevLicense
    expected: "1"
```

#### `shell`

Runs an arbitrary shell command; the condition passes when the exit code is 0.

```yaml
- name: Rust compiler responds
  check:
    type: shell
    command: "rustc --version"
    description: "Rust compiler version check"
```

### Composition with `all_of` and `any_of`

Combine conditions with logical operators for complex checks.

#### `all_of` — all conditions must pass (AND)

```yaml
- name: Full Rust toolchain
  check:
    type: all_of
    conditions:
      - type: command_exists
        command: rustc
      - type: command_exists
        command: cargo
      - type: dir_exists
        path: "~/.cargo"
```

#### `any_of` — at least one must pass (OR)

```yaml
- name: Python is available
  check:
    type: any_of
    conditions:
      - type: command_exists
        command: python
      - type: command_exists
        command: python3
```

### Enabling Assertion Checks

Assertions are evaluated during `anvil health` when `assertion_check` is enabled in the health config (it defaults to `true`):

```yaml
health:
  package_check: true
  file_check: true
  assertion_check: true   # Evaluate declarative assertions
```

### Complete Example

```yaml
name: rust-developer
version: "1.0.0"
description: "Rust development environment"

packages:
  winget:
    - id: Rustlang.Rustup

assertions:
  - name: cargo command exists
    check:
      type: command_exists
      command: cargo
  - name: rustc command exists
    check:
      type: command_exists
      command: rustc
  - name: Cargo directory exists
    check:
      type: dir_exists
      path: "~/.cargo"
  - name: Cargo bin on PATH
    check:
      type: path_contains
      substring: ".cargo/bin"
  - name: RUST_BACKTRACE is set
    check:
      type: env_var
      name: RUST_BACKTRACE
      value: "1"

health:
  package_check: true
  assertion_check: true
```

---

## 9. Inheritance

Workloads can extend other workloads to inherit their configuration.

### Basic Inheritance

```yaml
name: my-rust-dev
version: "1.0.0"

extends:
  - essentials      # Inherit base development tools
```

### Multiple Inheritance

```yaml
name: full-stack-dev
version: "1.0.0"

extends:
  - essentials
  - rust-developer
  - python-developer
```

### Merge Behavior

When a workload extends parents, configuration is merged:

| Section | Merge Behavior |
|---------|----------------|
| `packages` | Merged; child packages added to parent packages |
| `files` | Merged; child files override parent files with same destination |
| `scripts` | Merged; child scripts run after parent scripts |
| `environment` | Merged; child variables override parent variables |

### Override Example

Parent (`base/workload.yaml`):
```yaml
name: base
version: "1.0.0"

packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode

files:
  - source: .gitconfig
    destination: "~/.gitconfig"
```

Child (`extended/workload.yaml`):
```yaml
name: extended
version: "1.0.0"

extends:
  - base

packages:
  winget:
    # Adds to parent packages
    - id: Rustlang.Rustup

files:
  # Overrides parent's .gitconfig
  - source: .gitconfig
    destination: "~/.gitconfig"
```

### Inheritance Chains

Anvil resolves inheritance chains automatically:

```
my-workload
└── rust-developer
    └── essentials
```

Circular dependencies are detected and rejected:

```yaml
# workload-a extends workload-b
# workload-b extends workload-a
# ERROR: Circular dependency detected
```

### View Inheritance

```powershell
# Show inheritance tree
anvil show my-workload --inheritance-tree

# Show fully resolved workload
anvil show my-workload --resolved
```

---

## 10. Variable Expansion

Use variables in paths and values for dynamic configuration.

### Supported Variables

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `~` | User home directory | `C:\Users\username` |
| `${HOME}` | User home directory | `C:\Users\username` |
| `${USERNAME}` | Current username | `username` |
| `${COMPUTERNAME}` | Machine name | `WORKSTATION-01` |
| `${WORKLOAD_DIR}` | Workload directory | `C:\Workloads\my-workload` |
| `${USERPROFILE}` | User profile path | `C:\Users\username` |
| `${APPDATA}` | Roaming AppData | `C:\Users\username\AppData\Roaming` |
| `${LOCALAPPDATA}` | Local AppData | `C:\Users\username\AppData\Local` |
| `${env:VAR_NAME}` | Any environment variable | (varies) |

### Usage Examples

```yaml
files:
  # Home directory shorthand
  - source: .bashrc
    destination: "~/.bashrc"
  
  # Explicit home variable
  - source: config.json
    destination: "${HOME}/.config/myapp/config.json"
  
  # AppData paths
  - source: settings.json
    destination: "${APPDATA}/MyApp/settings.json"
  
  # Environment variable
  - source: custom.conf
    destination: "${env:MY_CUSTOM_PATH}/config.conf"

environment:
  variables:
    - name: MY_APP_HOME
      value: "${HOME}/.myapp"
  
  path_additions:
    - "${HOME}/.local/bin"
    - "${LOCALAPPDATA}/Programs/bin"
```

---

## 11. Best Practices

### Workload Design

1. **Use Descriptive Names**
   ```yaml
   name: rust-developer           # Good
   name: workload1                # Bad
   ```

2. **Include Version Information**
   ```yaml
   version: "1.2.0"               # Good: semantic versioning
   version: "latest"              # Bad: not meaningful
   ```

3. **Write Helpful Descriptions**
   ```yaml
   description: "Complete Rust development environment with debugging tools and VS Code extensions"
   ```

4. **Use Inheritance for Common Bases**
   ```yaml
   # Create a base workload for team-wide tools
   # Then extend it for role-specific setups
   extends:
     - team-base
   ```

### Package Management

1. **Pin Versions When Necessary**
   ```yaml
   packages:
     winget:
       # Pin when compatibility matters
       - id: Python.Python.3.12
         version: "3.12.0"
       
       # Use latest for frequently updated tools
       - id: Git.Git
   ```

2. **Use Machine Scope for Shared Tools**
   ```yaml
   packages:
     winget:
       - id: Microsoft.VisualStudioCode
         override:
           - "--scope"
           - "machine"
   ```

### File Management

1. **Always Enable Backups for Important Files**
   ```yaml
   files:
     - source: .gitconfig
       destination: "~/.gitconfig"
       backup: true
   ```

2. **Use Templates for Dynamic Content**
   ```yaml
   files:
     - source: config.toml.hbs
       destination: "~/.config/app/config.toml"
       template: true
   ```

### Script Safety

1. **Make Scripts Idempotent**
   ```powershell
   # Check before acting
   if (-not (Test-Path $target)) {
       # Create/install
   }
   ```

2. **Handle Errors Gracefully**
   ```powershell
   try {
       # Risky operation
   }
   catch {
       Write-Error "Operation failed: $_"
       exit 1
   }
   ```

3. **Include Assertions**
   ```yaml
   assertions:
     - name: "Installation verified"
       check:
         type: command_exists
         command: my-tool
   ```

### Testing

1. **Validate Before Committing**
   ```powershell
   anvil validate my-workload --strict
   ```

2. **Test with Dry Run**
   ```powershell
   anvil install my-workload --dry-run
   ```

3. **Test on Clean System**
   - Use a VM or container
   - Document dependencies

---

## 12. Example Workloads

### Minimal Workload

The simplest valid workload:

```yaml
# minimal/workload.yaml
name: minimal
version: "1.0.0"
description: "A minimal workload example"
```

### Package-Only Workload

Install software without files or scripts:

```yaml
# dev-essentials/workload.yaml
name: dev-essentials
version: "1.0.0"
description: "Essential development tools"

packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
    - id: Microsoft.WindowsTerminal
    - id: JanDeDobbeleer.OhMyPosh
```

### Full-Featured Workload

Complete example with all features:

```yaml
# full-example/workload.yaml
name: full-example
version: "1.0.0"
description: "Complete workload demonstrating all features"

extends:
  - essentials

packages:
  winget:
    - id: Rustlang.Rustup
    - id: LLVM.LLVM
      version: "17.0.6"
    - id: Microsoft.VisualStudio.2022.BuildTools
      override:
        - "--add"
        - "Microsoft.VisualStudio.Workload.VCTools"

files:
  - source: files/.cargo/config.toml
    destination: "~/.cargo/config.toml"
    backup: true
    
  - source: files/vscode/settings.json.hbs
    destination: "${APPDATA}/Code/User/settings.json"
    template: true

scripts:
  pre_install:
    - path: scripts/pre-check.ps1
      description: "Check prerequisites"
      
  post_install:
    - path: scripts/setup-rust.ps1
      description: "Configure Rust toolchain"
      elevated: false
      timeout: 600

commands:
  post_install:
    - run: "rustup component add rustfmt clippy"
      description: "Add Rust components"
      when:
        type: command_exists
        command: rustup

assertions:
  - name: cargo is available
    check:
      type: command_exists
      command: cargo
  - name: Cargo bin on PATH
    check:
      type: path_contains
      substring: ".cargo/bin"

environment:
  variables:
    - name: RUST_BACKTRACE
      value: "1"
      scope: user
      
  path_additions:
    - "~/.cargo/bin"
```

### Inherited Workload

Workload that builds on others:

```yaml
# rust-advanced/workload.yaml
name: rust-advanced
version: "1.0.0"
description: "Advanced Rust development with WASM and embedded support"

extends:
  - rust-developer

packages:
  winget:
    - id: Docker.DockerDesktop
    - id: WasmEdge.WasmEdge

scripts:
  post_install:
    - path: scripts/setup-targets.ps1
      description: "Add Rust compilation targets"

environment:
  path_additions:
    - "~/.wasmedge/bin"
```

With corresponding script:

```powershell
# rust-advanced/scripts/setup-targets.ps1
# Add additional Rust compilation targets

$ErrorActionPreference = "Stop"

try {
    Write-Host "Adding WASM target..."
    rustup target add wasm32-unknown-unknown
    
    Write-Host "Adding embedded targets..."
    rustup target add thumbv7em-none-eabihf
    
    Write-Host "Installing cargo tools for embedded..."
    cargo install cargo-embed
    cargo install probe-run
    
    exit 0
}
catch {
    Write-Error "Setup failed: $_"
    exit 1
}
```

---

## 13. Private Workload Repositories

You can maintain your own workloads in a separate Git repository and configure Anvil to discover them.

### Recommended Directory Layout

```
my-workloads/
├── my-dev-env/
│   ├── workload.yaml
│   ├── files/
│   │   └── .gitconfig
│   └── scripts/
│       └── setup.ps1
├── team-tools/
│   ├── workload.yaml
│   └── scripts/
│       └── post-install.ps1
└── README.md
```

Each subdirectory containing a `workload.yaml` is treated as a separate workload, following the same [directory structure](#1-workload-structure) as bundled workloads.

### Setup

1. Clone your workloads repository:
   ```powershell
   git clone https://github.com/your-org/workloads ~/my-workloads
   ```

2. Configure Anvil to search this path:
   ```powershell
   anvil config set workloads.paths '["~/my-workloads"]'
   ```

   Or edit `~/.anvil/config.yaml` directly:
   ```yaml
   workloads:
     paths:
       - "~/my-workloads"
   ```

3. Verify discovery:
   ```powershell
   anvil list
   ```

### Tips

- **Inheritance**: Private workloads can `extends:` built-in workloads (e.g., `extends: [essentials]`)
- **Version control**: Keep workloads in Git for team sharing and history
- **Multiple repos**: Add multiple paths for team-shared and personal workloads
- **Precedence**: If your workload has the same name as a built-in one, yours takes priority
- **Validation**: Run `anvil validate <name> --strict` to check your workloads before committing

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

## Resources

- [Anvil User Guide](USER_GUIDE.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)
- [GitHub Repository](https://github.com/kafkade/anvil)
- [Winget Documentation](https://learn.microsoft.com/en-us/windows/package-manager/)
- [Handlebars Templating](https://handlebarsjs.com/guide/)

---

*This guide is for Anvil v0.3.1. For other versions, check the corresponding documentation.*