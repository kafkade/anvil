# Schema Reference

This document is the authoritative reference for both the global configuration file (`~/.anvil/config.yaml`) and the workload definition file (`workload.yaml`). All field names, types, and defaults are derived from the Rust source structs.

---

## Configuration File (`~/.anvil/config.yaml`)

The global configuration file stores user preferences and default settings. If the file does not exist, Anvil uses the built-in defaults shown below.

### defaults

Default settings applied to CLI commands.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `defaults.shell` | string | `"powershell"` | Default shell for script execution (`powershell`, `pwsh`). |
| `defaults.script_timeout` | integer | `300` | Default script timeout in seconds. |
| `defaults.output_format` | string | `"table"` | Output format for commands. Valid values: `table`, `json`, `yaml`, `html`. |
| `defaults.color` | string | `"auto"` | Color output mode. Valid values: `auto`, `always`, `never`. |

### backup

Controls automatic backup behavior during installs.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `backup.auto_backup` | boolean | `true` | Create a backup before install operations. |
| `backup.retention_days` | integer | `30` | Delete backups older than this many days. |
| `backup.max_backups` | integer | `10` | Maximum number of backups to keep per workload. |
| `backup.compress` | boolean | `false` | Compress backups by default. |

### install

Settings that govern package installation behavior.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `install.parallel_packages` | boolean | `false` | Install packages in parallel. |
| `install.skip_installed` | boolean | `true` | Skip packages that are already installed. |
| `install.confirm` | boolean | `true` | Prompt for confirmation before installing. |

### workloads

Workload discovery configuration.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `workloads.paths` | list of strings | `[]` | Additional directories to search for workloads. |

### logging

Logging configuration.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `logging.level` | string | `"info"` | Log verbosity level. Valid values: `error`, `warn`, `info`, `debug`, `trace`. |
| `logging.file` | string or null | `null` | Path to a log file. `null` disables file logging. |

### Full Example

```yaml
defaults:
  shell: powershell
  script_timeout: 300
  output_format: table
  color: auto

backup:
  auto_backup: true
  retention_days: 30
  max_backups: 10
  compress: false

install:
  parallel_packages: false
  skip_installed: true
  confirm: true

workloads:
  paths:
    - ~/my-workloads
    - /shared/team-workloads

logging:
  level: info
  file: ~/.anvil/anvil.log
```

---

## Workload Schema (`workload.yaml`)

A workload is defined by a `workload.yaml` file inside a workload directory. The directory may also contain `files/` and `scripts/` subdirectories.

### Top-Level Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | — | Unique workload identifier (kebab-case). |
| `version` | string | yes | — | Semantic version (e.g., `"1.0.0"`). |
| `description` | string | yes | — | Human-readable description of the workload. |
| `extends` | list of strings | no | `null` | Parent workload names to inherit from. |
| `packages` | object | no | `null` | Package definitions, grouped by manager. |
| `files` | list of [FileEntry](#files) | no | `null` | Configuration files to deploy. |
| `scripts` | object | no | `null` | Scripts to execute at various phases. |
| `commands` | object | no | `null` | Inline commands to execute at various phases. |
| `environment` | object | no | `null` | Environment variables and PATH additions. |
| `health` | object | no | `null` | Health check configuration flags. |
| `assertions` | list of [Assertion](#assertions) | no | `null` | Declarative assertions for health validation. |

### packages

The `packages` object groups package definitions by package manager. Each manager key is optional.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `packages.winget` | list of [WingetPackage](#winget-package) | no | Windows packages managed by winget. |
| `packages.brew` | list of [BrewPackage](#brew-package) | no | macOS/Linux packages managed by Homebrew. |
| `packages.apt` | list of [AptPackage](#apt-package) | no | Debian/Ubuntu packages managed by APT. |

#### Winget Package

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | yes | — | Winget package identifier (e.g., `Git.Git`). |
| `version` | string | no | latest | Specific version to install. |
| `source` | string | no | `null` | Package source (e.g., `msstore` for Microsoft Store apps). |
| `override` | list of strings | no | `null` | Override arguments passed to the installer. |
| `override_args` | list of strings | no | `null` | Additional winget arguments. |

```yaml
packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
      version: "1.95.0"
    - id: Notepad++.Notepad++
      override:
        - --override
        - "/VERYSILENT /NORESTART"
    - id: 9NBLGGH4NNS1
      source: msstore
```

#### Brew Package

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | — | Package name (e.g., `git`, `node`). |
| `cask` | boolean | no | `false` | Whether this is a cask (GUI app) vs formula (CLI tool). |
| `tap` | string | no | `null` | Tap source (e.g., `homebrew/cask-fonts`). |

```yaml
packages:
  brew:
    - name: git
    - name: visual-studio-code
      cask: true
    - name: font-cascadia-code
      cask: true
      tap: homebrew/cask-fonts
```

#### APT Package

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | — | Package name (e.g., `git`, `build-essential`). |
| `version` | string | no | `null` | Specific version constraint. |

```yaml
packages:
  apt:
    - name: git
    - name: build-essential
      version: "12.9"
```

### files

Each entry in the `files` list describes a configuration file to deploy.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `source` | string | yes | — | Path relative to the workload's `files/` directory. |
| `destination` | string | yes | — | Target path. `~` expands to the user's home directory. |
| `backup` | boolean | no | `true` | Back up the existing file before overwriting. |
| `permissions` | string | no | `null` | File permissions (reserved for future use). |
| `template` | boolean | no | `false` | Process the file as a Handlebars template before deploying. |

```yaml
files:
  - source: user/.config/starship.toml
    destination: "~/.config/starship.toml"
  - source: user/.gitconfig
    destination: "~/.gitconfig"
    backup: true
    template: true
```

### scripts

The `scripts` object organizes script entries by execution phase. All phase keys are optional.

| Field | Type | Description |
|-------|------|-------------|
| `scripts.pre_install` | list of [ScriptEntry](#scriptentry) | Scripts to run **before** package installation. |
| `scripts.post_install` | list of [ScriptEntry](#scriptentry) | Scripts to run **after** package installation. |

#### ScriptEntry

Used in `pre_install` and `post_install` phases.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | Path relative to the workload's `scripts/` directory. |
| `shell` | string | no | `"powershell"` | Execution shell. |
| `description` | string | no | `null` | Human-readable description of what the script does. |
| `elevated` | boolean | no | `false` | Whether the script requires administrator privileges. |
| `timeout` | integer | no | `300` | Timeout in seconds. |

```yaml
scripts:
  pre_install:
    - path: pre-install.ps1
      description: "Prepare system for installation"
      timeout: 60

  post_install:
    - path: configure-terminal.ps1
      description: "Configure Windows Terminal settings"
      timeout: 300
      elevated: true
```

### commands

The `commands` object defines inline shell commands to execute at install phases. This is a lightweight alternative to scripts for simple one-liners.

| Field | Type | Description |
|-------|------|-------------|
| `commands.pre_install` | list of [CommandEntry](#commandentry) | Commands to run **before** package installation. |
| `commands.post_install` | list of [CommandEntry](#commandentry) | Commands to run **after** package installation. |

#### CommandEntry

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `run` | string | yes | — | Shell command string to execute. |
| `description` | string | no | `null` | Human-readable description. |
| `timeout` | integer | no | `300` | Timeout in seconds. |
| `elevated` | boolean | no | `false` | Whether the command requires administrator privileges. |
| `when` | [Condition](#condition-types) | no | `null` | Condition that must be true for this command to run. |
| `continue_on_error` | boolean | no | `false` | Whether to continue if this command fails. |

```yaml
commands:
  post_install:
    - run: "rustup default stable"
      description: "Set Rust stable as default toolchain"
      timeout: 120
    - run: "cargo install cargo-watch"
      description: "Install cargo-watch"
      when:
        type: command_exists
        command: cargo
      continue_on_error: true
```

### environment

Environment variable and PATH configuration.

| Field | Type | Description |
|-------|------|-------------|
| `environment.variables` | list of [EnvVariable](#envvariable) | Environment variables to set. |
| `environment.path_additions` | list of strings | Directory paths to add to the system PATH. `~` expands to the user's home. |

#### EnvVariable

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | — | Variable name. |
| `value` | string | yes | — | Variable value. |
| `scope` | string | no | `"user"` | Scope: `user` or `machine`. |

```yaml
environment:
  variables:
    - name: EDITOR
      value: code
      scope: user
    - name: DOTNET_CLI_TELEMETRY_OPTOUT
      value: "1"

  path_additions:
    - "~/.cargo/bin"
    - "~/.local/bin"
```

### health

Flags that control which categories of health checks are evaluated during `anvil health`.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `package_check` | boolean | no | `true` | Verify that declared packages are installed. |
| `file_check` | boolean | no | `true` | Verify that declared files are deployed. |
| `assertion_check` | boolean | no | `true` | Evaluate declarative assertions. |

```yaml
health:
  package_check: true
  file_check: true
  assertion_check: true
```

### assertions

Declarative health assertions using the condition engine. Each assertion pairs a display name with a condition to evaluate.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Display name for the assertion. |
| `check` | [Condition](#condition-types) | yes | The condition to evaluate. |

#### Condition Types

Conditions are serialized as tagged objects with a `type` discriminator field. The following condition types are available:

##### `command_exists`

Check whether a command is available on PATH.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"command_exists"` |
| `command` | string | yes | Command name to look for. |

##### `file_exists`

Check whether a file exists at the given path (`~` is expanded).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"file_exists"` |
| `path` | string | yes | File path to check. |

##### `dir_exists`

Check whether a directory exists at the given path (`~` is expanded).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"dir_exists"` |
| `path` | string | yes | Directory path to check. |

##### `env_var`

Check whether an environment variable is set, optionally matching an expected value.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"env_var"` |
| `name` | string | yes | Environment variable name. |
| `value` | string | no | Expected value. If omitted, only checks that the variable is set. |

##### `path_contains`

Check whether the system PATH contains a given substring.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"path_contains"` |
| `substring` | string | yes | Substring to search for in PATH entries. |

##### `registry_value`

Query a Windows registry value under HKCU or HKLM.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"registry_value"` |
| `hive` | string | yes | Registry hive: `"HKCU"` or `"HKLM"`. |
| `key` | string | yes | Full key path (e.g., `"SOFTWARE\\Microsoft\\Windows\\CurrentVersion"`). |
| `name` | string | yes | Value name inside the key. |
| `expected` | string | no | Expected data. If omitted, only asserts the value exists. |

##### `shell`

Run a shell command; the condition passes when the exit code is 0.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"shell"` |
| `command` | string | yes | Shell command to execute. |
| `description` | string | no | Human-readable description of the check. |

##### `all_of`

Logical AND — all child conditions must pass.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"all_of"` |
| `conditions` | list of Condition | yes | Child conditions to evaluate. |

##### `any_of`

Logical OR — at least one child condition must pass.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | `"any_of"` |
| `conditions` | list of Condition | yes | Child conditions to evaluate. |

#### Assertions Example

```yaml
assertions:
  - name: "Git is installed"
    check:
      type: command_exists
      command: git

  - name: "Config directory exists"
    check:
      type: dir_exists
      path: "~/.config/app"

  - name: "EDITOR is set to VS Code"
    check:
      type: env_var
      name: EDITOR
      value: code

  - name: "Cargo bin is in PATH"
    check:
      type: path_contains
      substring: ".cargo/bin"

  - name: "Dark mode enabled"
    check:
      type: registry_value
      hive: HKCU
      key: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
      name: AppsUseLightTheme
      expected: "0x0"

  - name: "Rust toolchain ready"
    check:
      type: all_of
      conditions:
        - type: command_exists
          command: rustc
        - type: command_exists
          command: cargo
        - type: path_contains
          substring: ".cargo/bin"
```

### Full Example

A complete `workload.yaml` demonstrating all sections:

```yaml
name: developer-tools
version: "1.0.0"
description: "Full developer workstation setup"
extends:
  - essentials

packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
    - id: Rustlang.Rustup
    - id: Starship.Starship

files:
  - source: user/.config/starship.toml
    destination: "~/.config/starship.toml"
    backup: true
  - source: user/.gitconfig
    destination: "~/.gitconfig"
    template: true

scripts:
  pre_install:
    - path: pre-install.ps1
      description: "System preparation"
      timeout: 60

  post_install:
    - path: configure-terminal.ps1
      description: "Configure Windows Terminal"
      timeout: 300
      elevated: true

commands:
  post_install:
    - run: "rustup default stable"
      description: "Set default Rust toolchain"
      timeout: 120
    - run: "cargo install cargo-watch"
      description: "Install cargo-watch"
      when:
        type: command_exists
        command: cargo
      continue_on_error: true

environment:
  variables:
    - name: EDITOR
      value: code
    - name: DOTNET_CLI_TELEMETRY_OPTOUT
      value: "1"
      scope: user
  path_additions:
    - "~/.cargo/bin"

health:
  package_check: true
  file_check: true
  assertion_check: true

assertions:
  - name: "Git is installed"
    check:
      type: command_exists
      command: git
  - name: "VS Code is installed"
    check:
      type: command_exists
      command: code
  - name: "Starship config deployed"
    check:
      type: file_exists
      path: "~/.config/starship.toml"
```
