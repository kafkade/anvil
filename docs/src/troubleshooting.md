# Troubleshooting

A guide to diagnosing and resolving common Anvil issues.

## 1. Installation Issues

### Anvil won't run

**Symptoms:**
- "anvil" is not recognized as a command
- Application crashes immediately

**Solutions:**

1. **Check PATH configuration**

   Windows:
   ```powershell
   where.exe anvil
   # If not found, add to PATH
   $env:PATH += ";C:\path\to\anvil"
   ```

   Linux/macOS:
   ```sh
   which anvil
   # If not found, add to PATH
   export PATH="$PATH:/path/to/anvil"
   ```

2. **Verify platform compatibility**
   - Anvil currently requires Windows 10 (1809+) or Windows 11
   - Cross-platform support is on the roadmap

3. **Verify the download isn't corrupted**
   - Re-download from the [releases page](https://github.com/kafkade/anvil/releases)
   - Verify the file hash if provided

---

### Package manager not found

**Symptoms:**
- Error: "winget is not recognized"
- Error: "Package manager not available"
- Package installation fails before starting

**Solutions (Windows — winget):**

1. **Install Windows Package Manager**

   ```powershell
   # Open Microsoft Store to App Installer
   Start-Process "ms-windows-store://pdp/?ProductId=9NBLGGH4NNS1"
   ```

2. **Update App Installer**
   - Open Microsoft Store → search "App Installer" → click "Update"

3. **Test availability**
   ```powershell
   winget --version
   ```

4. **Restart your terminal** after installing winget

---

## 2. Package Installation Issues

### Package not found

**Symptoms:**
- Error: "No package found matching input criteria"
- Error: "Package 'X' not found in any source"

**Solutions:**

1. **Verify package ID**
   ```powershell
   winget search <package-name>
   winget search --exact "Package Name"
   ```

2. **Check package source in workload**
   ```yaml
   packages:
     winget:
       - id: SomeApp.App
         source: msstore    # For Microsoft Store apps
   ```

3. **Update package sources**
   ```powershell
   winget source update
   ```

---

### Installation hangs

**Symptoms:**
- Installation appears frozen
- No progress for extended time

**Solutions:**

1. **Check for interactive prompts**
   - Some installers require user interaction
   - Run anvil without `--quiet` to see prompts

2. **Use silent install overrides**
   ```yaml
   packages:
     winget:
       - id: SomeApp.App
         override:
           - "--silent"
           - "--accept-license"
   ```

3. **Check for pending system updates**
   - Pending OS updates can block installations
   - Complete any pending updates and restart

---

### Access denied during installation

**Symptoms:**
- Error: "Access is denied"
- Error: "Administrator privileges required"

**Solutions:**

1. **Run with elevation** (Windows)
   ```powershell
   Start-Process powershell -Verb RunAs
   anvil install my-workload
   ```

2. **Use user-scope installation**
   ```yaml
   packages:
     winget:
       - id: Package.Name
         override:
           - "--scope"
           - "user"
   ```

---

## 3. File Operation Issues

### Permission denied

**Symptoms:**
- Error: "Permission denied" when copying files
- File operations fail

**Solutions:**

1. **Use user-writable paths**
   ```yaml
   files:
     - source: config.json
       destination: "~/.config/app/config.json"  # Expands to user home
   ```

2. **Check target directory permissions**

   Windows:
   ```powershell
   Get-Acl "C:\path\to\directory" | Format-List
   ```

   Linux/macOS:
   ```sh
   ls -la /path/to/directory
   ```

3. **Run as administrator** for system-level paths

---

### File not found

**Symptoms:**
- Error: "Source file not found"

**Solutions:**

1. **Verify source path** — paths are relative to the workload directory
   ```yaml
   files:
     - source: files/config.json      # Relative to workload dir
       destination: "~/.config/app/config.json"
   ```

2. **Check workload directory structure**
   ```
   my-workload/
   ├── workload.yaml
   └── files/
       └── config.json
   ```

---

### Backup failed

**Symptoms:**
- Error: "Failed to create backup"

**Solutions:**

1. **Check disk space**
2. **Verify backup directory** exists and is writable
   ```powershell
   $env:ANVIL_BACKUP_DIR = "D:\Backups\anvil"
   ```

---

## 4. Script Execution Issues

### Script won't execute

**Symptoms:**
- Error: "Script execution failed"
- No output from script

**Solutions:**

1. **Verify script path** in workload.yaml — paths are relative to `scripts/`
   ```yaml
   scripts:
     post_install:
       - path: setup.ps1    # Relative to scripts/ directory
   ```

2. **Test script manually**
   ```powershell
   & "C:\workloads\my-workload\scripts\setup.ps1"
   ```

---

### Execution policy error (Windows)

**Symptoms:**
- Error: "Running scripts is disabled on this system"

**Solutions:**

1. **Set execution policy**
   ```powershell
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

2. **Check group policy** — corporate environments may have stricter policies

---

### Script timeout

**Symptoms:**
- Error: "Script execution timed out"

**Solutions:**

1. **Increase timeout in workload**
   ```yaml
   scripts:
     post_install:
       - path: scripts/long-running.ps1
         timeout: 1800    # 30 minutes (default: 300)
   ```

2. **Check for infinite loops** or slow network operations in the script

---

### Elevated script fails (Windows)

**Symptoms:**
- Error: "Elevation required"

**Solutions:**

1. **Run anvil as administrator**
2. **Set the elevated flag** in workload.yaml
   ```yaml
   scripts:
     post_install:
       - path: scripts/admin-setup.ps1
         elevated: true
   ```

---

## 5. Health Check Issues

### False failures

**Symptoms:**
- Health check fails but software is working
- Intermittent failures

**Solutions:**

1. **Run with verbose output**
   ```powershell
   anvil health my-workload --verbose
   ```

2. **Review health check scripts** — they may have outdated expectations

3. **Update version expectations**
   ```powershell
   winget list --id Package.Name
   ```

---

### Partial results

**Symptoms:**
- Some checks don't run
- Health report incomplete

**Solutions:**

1. **Check verbose output** for errors in earlier checks
   ```powershell
   anvil health my-workload -vvv
   ```

2. **Verify all referenced scripts exist** in the workload directory

---

## 6. Configuration Issues

### Config file not loading

**Symptoms:**
- Settings not applied
- Default values used instead of custom

**Solutions:**

1. **Check config location**
   ```powershell
   anvil config path
   ```

2. **View current configuration**
   ```powershell
   anvil config list
   ```

3. **Reset to defaults**
   ```powershell
   anvil config reset
   ```

---

### Workloads not found

**Symptoms:**
- Error: "Workload 'X' not found"
- Empty list from `anvil list`

**Solutions:**

1. **Check search paths**
   ```powershell
   anvil config list
   ```

2. **Verify workload structure** — needs a `workload.yaml` file
   ```
   workload-name/
   └── workload.yaml
   ```

3. **Use explicit path**
   ```powershell
   anvil install my-workload --path /path/to/workloads
   ```

---

## 7. Workload Inheritance Issues

### Circular dependency detected

**Symptoms:**
- Error: "Circular dependency detected: A → B → A"

**Solutions:**

1. **Inspect the `extends` chain** in each workload
   ```yaml
   # workload-a/workload.yaml
   extends:
     - workload-b     # workload-b also extends workload-a → cycle!
   ```

2. **Break the cycle** by removing one of the `extends` references
3. **Run `anvil validate`** to see the detected chain

---

### Maximum inheritance depth exceeded

**Symptoms:**
- Error: "Maximum inheritance depth (10) exceeded for workload 'X'"

**Solutions:**

1. **Flatten the hierarchy** — the maximum allowed depth is 10
2. **Merge intermediate workloads** to reduce nesting
3. **Check for unintended long chains**
   ```powershell
   anvil show my-workload    # Review resolved inheritance
   ```

---

### Parent workload not found

**Symptoms:**
- Error: "Parent workload 'X' not found"

**Solutions:**

1. **Verify the parent name** matches an available workload
   ```powershell
   anvil list    # See all available workloads
   ```

2. **Check search paths** — ensure the parent workload directory is discoverable
   ```powershell
   anvil config list    # Review configured workload search paths
   ```

3. **Use `--path`** if the parent is in a custom location
   ```powershell
   anvil install my-workload --path /path/to/workloads
   ```

---

## 8. Platform-Specific Warnings

### Workload references unavailable package manager

**Symptoms:**
- Warning: "Homebrew packages defined but Homebrew is not available on Windows"
- Warning: "APT packages defined but APT is not available on Windows"
- Warning: "Winget packages defined but winget is only available on Windows"

**Solutions:**

1. **Run `anvil validate --strict`** to surface these warnings
2. **Use platform-appropriate package managers** in your workload
   ```yaml
   packages:
     winget:           # Windows only
       - id: Package.Name
     brew:             # macOS only (planned)
       - name: package
     apt:              # Linux only (planned)
       - name: package
   ```

3. **Split into platform-specific workloads** if targeting multiple OSes

---

## 9. Assertion and Command Failures

### Assertion failure

**Symptoms:**
- Health check assertion fails with `[FAIL]`
- Error: condition evaluation returned false

**Solutions:**

1. **Run the health check with verbose output** to see which assertion failed
   ```powershell
   anvil health my-workload -vvv
   ```

2. **Review the condition** — assertions check system state (command existence, file presence, environment variables, registry values)

3. **Common assertion types and fixes:**
   - **Command not found** — install the missing tool or add it to PATH
   - **File missing** — re-run `anvil install` to deploy files
   - **Environment variable not set** — check your shell profile or restart the terminal

---

### Command execution failure

**Symptoms:**
- Error: "Command not found: X"
- Error: "Command timed out after N seconds"
- Error: "Command requires elevated privileges"

**Solutions:**

1. **Command not found** — ensure the executable is installed and on PATH
   ```powershell
   Get-Command <command-name>
   ```

2. **Timeout** — increase the timeout in your workload definition
   ```yaml
   scripts:
     post_install:
       - path: slow-script.ps1
         timeout: 1800    # 30 minutes
   ```

3. **Elevation required** — run anvil as administrator, or set `elevated: true`

---

## 10. Error Reference

### Common Error Codes

| Error | Description | Solution |
|-------|-------------|----------|
| `E001` | Workload not found | Check name and search paths |
| `E002` | Invalid workload schema | Run `anvil validate` for details |
| `E003` | Circular dependency | Review `extends` chain in workloads |
| `E004` | Package installation failed | Check package manager logs, verify package ID |
| `E005` | File operation failed | Check permissions, verify paths |
| `E006` | Script execution failed | Review script output, check syntax |
| `E007` | Health check failed | Review check results, update scripts |
| `E008` | Configuration error | Validate config file syntax |
| `E009` | Backup operation failed | Check disk space and permissions |
| `E010` | Restore operation failed | Verify backup exists and is valid |

---

## 11. Getting Help

### Verbose Output

Get more detailed information about what Anvil is doing:

```sh
anvil -v <command>      # Some detail
anvil -vv <command>     # More detail
anvil -vvv <command>    # Maximum detail (debug level)
```

### Enable Logging

```sh
# Set log level via environment variable
ANVIL_LOG=debug anvil install my-workload

# Available levels: error, warn, info, debug, trace
```

### System Information

Gather helpful information when reporting issues:

```sh
# Anvil version
anvil --version

# Operating system
# Windows: winver
# Linux/macOS: uname -a

# Package manager version
winget --version        # Windows
```

### Reporting Issues

When [reporting issues](https://github.com/kafkade/anvil/issues), include:

1. **Anvil version**: `anvil --version`
2. **Operating system and version**
3. **Command that failed**: exact command you ran
4. **Error message**: complete error output
5. **Verbose output**: run with `-vvv` flag
6. **Workload file**: contents of `workload.yaml` (sanitized if needed)

### Resources

- [GitHub Issues](https://github.com/kafkade/anvil/issues)
- [User Guide](user-guide.md)
- [Workload Authoring Guide](workload-authoring.md)
- [Architecture](architecture.md)

---

## Quick Reference

### Common Commands for Troubleshooting

```sh
# Check Anvil installation
anvil --version

# Validate workload
anvil validate my-workload --strict

# Preview installation (dry run)
anvil install my-workload --dry-run

# Verbose health check
anvil health my-workload -vvv

# View current configuration
anvil config list

# Reset configuration
anvil config reset
```

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `ANVIL_CONFIG` | Custom config file path |
| `ANVIL_WORKLOADS` | Additional workload search paths |
| `ANVIL_LOG` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `ANVIL_BACKUP_DIR` | Custom backup directory |
| `NO_COLOR` | Disable colored output |