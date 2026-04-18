# CLI Reference

Complete reference for every Anvil command, flag, and option.

## Global Options

These options are available on **all** commands:

| Flag | Description |
|------|-------------|
| `-v, --verbose...` | Increase output verbosity (repeat for more: `-v`, `-vv`, `-vvv`) |
| `-q, --quiet` | Suppress non-essential output |
| `-c, --config <PATH>` | Use a custom configuration file |
| `--no-color` | Disable colored output |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

---

## Commands

### anvil install

**Synopsis:** `anvil install [OPTIONS] <WORKLOAD>`

**Description:** Apply a workload configuration to the system. Installs packages, deploys files, and runs scripts as defined in the workload.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<WORKLOAD>` | Yes | Name of the workload to install (or path to `workload.yaml`) |

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-d, --dry-run` | — | Show what would be done without making changes |
| `-f, --force` | — | Skip confirmation prompts |
| `-p, --packages-only` | — | Only install packages, skip files and scripts |
| `--skip-packages` | — | Skip package installation |
| `--skip-files` | — | Skip file operations |
| `--skip-scripts` | — | Skip all script execution |
| `--skip-pre-scripts` | — | Skip pre-installation scripts |
| `--skip-post-scripts` | — | Skip post-installation scripts |
| `--no-backup` | — | Don't backup existing files before overwriting |
| `--upgrade` | — | Upgrade existing packages to specified versions |
| `--retry-failed` | — | Retry only failed packages from previous run |
| `--parallel` | — | Run installations in parallel where safe |
| `-j, --jobs <N>` | `4` | Number of parallel package installations |
| `--timeout <SECONDS>` | `3600` | Global timeout for operations in seconds |
| `--files-only` | — | Only process files, skip packages and scripts |
| `--force-files` | — | Force overwrite files without checking hash |

**Examples:**

```sh
# Install a workload
anvil install essentials

# Dry run to preview changes
anvil install dev-tools --dry-run

# Install only packages, skip everything else
anvil install essentials --packages-only

# Force install with parallel jobs
anvil install dev-tools --force --parallel -j 8

# Retry previously failed packages
anvil install essentials --retry-failed

# Install from a specific path
anvil install ./my-workload/workload.yaml

# Deploy only files, no packages or scripts
anvil install essentials --files-only --force-files
```

**Exit Codes:** `0` = success, `1` = failure

---

### anvil health

**Synopsis:** `anvil health [OPTIONS] <WORKLOAD>`

**Description:** Check system health against a workload definition. Verifies packages are installed, files are deployed, and health check scripts pass.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<WORKLOAD>` | Yes | Name of the workload to check against |

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --output <OUTPUT>` | `table` | Output format (`table`, `json`, `yaml`, `html`) |
| `-f, --file <PATH>` | — | Write report to file instead of stdout |
| `--fail-fast` | — | Stop on first failure |
| `--packages-only` | — | Only check packages |
| `--files-only` | — | Only check files |
| `--scripts-only` | — | Only run health check scripts |
| `--assertions-only` | — | Only evaluate declarative assertions |
| `-s, --strict` | — | Treat warnings as errors |
| `--fix` | — | Attempt to install missing packages |
| `--update` | — | Update packages with available updates |
| `--no-cache` | — | Skip cache and query winget directly |
| `--show-diff` | — | Show file differences for modified files |
| `--script <NAME>` | — | Run only a specific health check script by name |

**Examples:**

```sh
# Check health of a workload
anvil health essentials

# JSON report saved to file
anvil health dev-tools -o json -f report.json

# Check only packages, stop on first failure
anvil health essentials --packages-only --fail-fast

# Auto-fix missing packages
anvil health essentials --fix

# Run a single health check script
anvil health essentials --script "Git Configuration"

# Strict mode — warnings become errors
anvil health essentials --strict
```

**Exit Codes:** `0` = all checks passed, `1` = one or more checks failed

---

### anvil list

**Synopsis:** `anvil list [OPTIONS]`

**Description:** List available workloads discovered from all configured search paths.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-a, --all` | — | Include built-in and custom workloads |
| `-l, --long` | — | Show detailed information |
| `--path <PATH>` | — | Search for workloads in additional path |
| `--all-paths` | — | Show all discovered paths including shadowed duplicates |
| `-o, --output <OUTPUT>` | — | Output format (`table`, `json`, `yaml`, `html`) |

**Examples:**

```sh
# List all workloads
anvil list

# Detailed listing with extra info
anvil list --long

# Include all sources
anvil list --all

# Search an additional directory
anvil list --path ~/my-workloads

# Machine-readable output
anvil list -o json
```

**Exit Codes:** `0` = success, `1` = failure

---

### anvil show

**Synopsis:** `anvil show [OPTIONS] <WORKLOAD>`

**Description:** Display detailed workload information including packages, files, scripts, and configuration.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<WORKLOAD>` | Yes | Name of the workload to display |

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-r, --resolved` | — | Show resolved configuration (with inheritance applied) |
| `--show-inheritance` | — | Show inheritance tree visualization |
| `-o, --output <OUTPUT>` | `yaml` | Output format (`yaml`, `json`) |

**Examples:**

```sh
# Show workload definition
anvil show essentials

# Show with inheritance resolved
anvil show dev-tools --resolved

# View inheritance tree
anvil show dev-tools --show-inheritance

# Output as JSON
anvil show essentials -o json
```

**Exit Codes:** `0` = success, `1` = failure

---

### anvil validate

**Synopsis:** `anvil validate [OPTIONS] [PATH]`

**Description:** Validate workload definition syntax. Checks YAML structure, schema conformance, and optionally script syntax.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `[PATH]` | No | Path to `workload.yaml` file or workload directory |

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--strict` | — | Enable strict validation mode |
| `--schema` | — | Output JSON schema for workload definitions |
| `--check-scripts` | — | Validate script syntax using PowerShell parser |
| `--scripts-only` | — | Only validate scripts (skip other validation) |

**Examples:**

```sh
# Validate a workload directory
anvil validate workloads/essentials

# Strict mode validation
anvil validate workloads/dev-tools --strict

# Validate with script syntax checking
anvil validate workloads/essentials --check-scripts

# Dump the JSON schema
anvil validate --schema

# Validate only scripts
anvil validate workloads/essentials --scripts-only
```

**Exit Codes:** `0` = valid, `1` = validation errors found

---

### anvil init

**Synopsis:** `anvil init [OPTIONS] <NAME>`

**Description:** Initialize a new workload template with scaffolded directory structure and `workload.yaml`.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<NAME>` | Yes | Name for the new workload |

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-t, --template <TEMPLATE>` | `standard` | Base template (`minimal`, `standard`, `full`) |
| `-e, --extends <PARENT>` | — | Parent workload to extend |
| `-o, --output <PATH>` | — | Output directory |

**Template values:**

| Template | Description |
|----------|-------------|
| `minimal` | Minimal workload with just metadata |
| `standard` | Standard workload with common sections |
| `full` | Full workload with all sections and examples |

**Examples:**

```sh
# Create a standard workload
anvil init my-workload

# Create a minimal workload
anvil init my-workload --template minimal

# Create a workload extending essentials
anvil init my-workload --extends essentials

# Full template in a custom directory
anvil init my-workload --template full --output ~/workloads
```

**Exit Codes:** `0` = success, `1` = failure

---

### anvil status

**Synopsis:** `anvil status [OPTIONS] [WORKLOAD]`

**Description:** Show installation status and state for one or all workloads.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `[WORKLOAD]` | No | Name of the workload (shows all if omitted) |

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --output <OUTPUT>` | `table` | Output format (`table`, `json`, `yaml`, `html`) |
| `-l, --long` | — | Show detailed status including timestamps |
| `--clear` | — | Clear stored state for the specified workload |

**Examples:**

```sh
# Show status for all workloads
anvil status

# Status for a specific workload
anvil status essentials

# Detailed status with timestamps
anvil status essentials --long

# Clear stored state
anvil status essentials --clear

# Machine-readable output
anvil status -o json
```

**Exit Codes:** `0` = success, `1` = failure

---

### anvil completions

**Synopsis:** `anvil completions [OPTIONS] <SHELL>`

**Description:** Generate shell completion scripts for the specified shell.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<SHELL>` | Yes | Target shell (`bash`, `zsh`, `fish`, `powershell`, `elvish`) |

**Examples:**

**PowerShell:**
```powershell
anvil completions powershell | Out-String | Invoke-Expression

# Or persist to $PROFILE
anvil completions powershell >> $PROFILE
```

**Bash:**
```bash
anvil completions bash > ~/.local/share/bash-completion/completions/anvil
```

**Zsh:**
```zsh
anvil completions zsh > ~/.zfunc/_anvil
```

**Fish:**
```fish
anvil completions fish > ~/.config/fish/completions/anvil.fish
```

**Exit Codes:** `0` = success, `1` = failure

---

### anvil backup

**Synopsis:** `anvil backup <COMMAND>`

**Description:** Manage file backups created during workload installation. Anvil backs up existing files before overwriting them; this command lets you list, restore, verify, and clean those backups.

#### anvil backup create

**Synopsis:** `anvil backup create [OPTIONS]`

**Description:** Create a new backup.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-n, --name <NAME>` | — | Name for the backup |
| `-w, --workload <WORKLOAD>` | — | Only backup files related to workload |
| `--include-packages` | — | Include winget package list export |
| `--compress` | — | Create compressed archive |

**Examples:**

```sh
# Create a named backup
anvil backup create --name "before-upgrade"

# Backup files for a specific workload
anvil backup create --workload essentials

# Compressed backup with package list
anvil backup create --compress --include-packages
```

#### anvil backup list

**Synopsis:** `anvil backup list [OPTIONS]`

**Description:** List all backups.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-w, --workload <WORKLOAD>` | — | Filter by workload name |
| `-o, --output <OUTPUT>` | `table` | Output format (`table`, `json`, `yaml`, `html`) |
| `-l, --long` | — | Show detailed information |

**Examples:**

```sh
# List all backups
anvil backup list

# Filter by workload
anvil backup list --workload essentials

# Detailed listing
anvil backup list --long
```

#### anvil backup show

**Synopsis:** `anvil backup show [OPTIONS] <ID>`

**Description:** Show details for a specific backup.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<ID>` | Yes | Backup ID |

**Examples:**

```sh
anvil backup show abc123
```

#### anvil backup restore

**Synopsis:** `anvil backup restore [OPTIONS] [ID]`

**Description:** Restore files from a backup.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `[ID]` | No | Backup ID to restore (or use `--workload`) |

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-w, --workload <WORKLOAD>` | — | Restore all backups for a workload |
| `-d, --dry-run` | — | Show what would be done without making changes |
| `-f, --force` | — | Skip confirmation prompts |

**Examples:**

```sh
# Restore a specific backup
anvil backup restore abc123

# Dry run restore
anvil backup restore abc123 --dry-run

# Restore all backups for a workload
anvil backup restore --workload essentials --force
```

#### anvil backup clean

**Synopsis:** `anvil backup clean [OPTIONS]`

**Description:** Remove old backups.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--older-than <DAYS>` | `30` | Remove backups older than N days |
| `-d, --dry-run` | — | Show what would be done without making changes |
| `-f, --force` | — | Skip confirmation prompts |

**Examples:**

```sh
# Clean backups older than 30 days (default)
anvil backup clean

# Clean backups older than 7 days
anvil backup clean --older-than 7

# Preview what would be removed
anvil backup clean --dry-run
```

#### anvil backup verify

**Synopsis:** `anvil backup verify [OPTIONS]`

**Description:** Verify backup integrity by checking that backup files exist and are uncorrupted.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-w, --workload <WORKLOAD>` | — | Only verify backups for a specific workload |
| `--fix` | — | Fix issues by removing corrupted/missing entries |

**Examples:**

```sh
# Verify all backups
anvil backup verify

# Verify and auto-fix issues
anvil backup verify --fix

# Verify backups for one workload
anvil backup verify --workload essentials
```

---

### anvil config

**Synopsis:** `anvil config <COMMAND>`

**Description:** Manage global Anvil configuration. Settings are persisted in a YAML file at `~/.anvil/config.yaml`.

#### anvil config get

**Synopsis:** `anvil config get [OPTIONS] <KEY>`

**Description:** Get a configuration value.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Configuration key (e.g., `defaults.shell`) |

**Examples:**

```sh
anvil config get defaults.shell
anvil config get workload_paths
```

#### anvil config set

**Synopsis:** `anvil config set [OPTIONS] <KEY> <VALUE>`

**Description:** Set a configuration value.

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Configuration key (e.g., `defaults.shell`) |
| `<VALUE>` | Yes | Value to set |

**Examples:**

```sh
anvil config set defaults.shell powershell
anvil config set defaults.timeout 600
```

#### anvil config list

**Synopsis:** `anvil config list [OPTIONS]`

**Description:** List all configuration values.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --output <OUTPUT>` | `table` | Output format (`table`, `json`, `yaml`, `html`) |

**Examples:**

```sh
# Show all config
anvil config list

# JSON output
anvil config list -o json
```

#### anvil config reset

**Synopsis:** `anvil config reset [OPTIONS]`

**Description:** Reset configuration to defaults.

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --force` | — | Skip confirmation prompt |

**Examples:**

```sh
# Reset with confirmation
anvil config reset

# Force reset without prompt
anvil config reset --force
```

#### anvil config edit

**Synopsis:** `anvil config edit [OPTIONS]`

**Description:** Open configuration file in the default editor.

**Examples:**

```sh
anvil config edit
```

#### anvil config path

**Synopsis:** `anvil config path [OPTIONS]`

**Description:** Show configuration file path.

**Examples:**

```sh
anvil config path
```

---

## Environment Variables

| Variable | Description | Used by |
|----------|-------------|---------|
| `NO_COLOR` | Disables colored output when set (any value). Standard convention ([no-color.org](https://no-color.org)). | All commands |
| `ANVIL_WORKLOAD` | Name of the current workload being processed. | Scripts, templates |
| `ANVIL_WORKLOAD_PATH` | Absolute path to the workload directory. | Scripts |
| `ANVIL_DRY_RUN` | Set to `"true"` or `"false"` during script execution. | Scripts |
| `ANVIL_VERBOSE` | Verbosity level (`0`–`3`) during script execution. | Scripts |
| `ANVIL_PHASE` | Current execution phase: `pre_install`, `post_install`, or `health_check`. | Scripts |
| `ANVIL_VERSION` | Anvil version string. Available in scripts and Handlebars templates. | Scripts, templates |
| `RUST_LOG` | Controls Rust log output (e.g., `RUST_LOG=debug`). Standard `env_logger` / `tracing` variable. | Debugging |
| `RUST_BACKTRACE` | Enables Rust backtraces on panic (`1` = short, `full` = complete). | Debugging |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success — command completed, all checks passed |
| `1` | General failure — operation failed or health checks did not pass |
| `2` | Usage error — invalid arguments or missing required parameters |
