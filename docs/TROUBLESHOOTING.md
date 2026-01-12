# Troubleshooting Guide

A guide to diagnosing and resolving common Winforge issues.

## Table of Contents

1. [Installation Issues](#1-installation-issues)
2. [Package Installation Issues](#2-package-installation-issues)
3. [File Operation Issues](#3-file-operation-issues)
4. [Script Execution Issues](#4-script-execution-issues)
5. [Health Check Issues](#5-health-check-issues)
6. [Configuration Issues](#6-configuration-issues)
7. [Error Reference](#7-error-reference)
8. [Getting Help](#8-getting-help)

---

## 1. Installation Issues

### Winforge won't run

**Symptoms:**
- "winforge" is not recognized as a command
- Missing DLL errors
- Application crashes immediately

**Solutions:**

1. **Check PATH configuration**
   ```powershell
   # Verify winforge is in PATH
   where.exe winforge
   
   # If not found, add to PATH
   $env:PATH += ";C:\path\to\winforge"
   ```

2. **Verify Windows version compatibility**
   - Winforge requires Windows 10 (1809+) or Windows 11
   ```powershell
   # Check Windows version
   winver
   ```

3. **Check for antivirus interference**
   - Some antivirus software may block unsigned executables
   - Add winforge.exe to your antivirus allowlist
   - Check Windows Defender exclusions

4. **Verify the download isn't corrupted**
   - Re-download from the releases page
   - Verify the file hash if provided

---

### "winget not found" error

**Symptoms:**
- Error: "winget is not recognized"
- Error: "Windows Package Manager not found"
- Package installation fails before starting

**Solutions:**

1. **Install Windows Package Manager**
   
   Option A - Microsoft Store:
   ```powershell
   # Open Microsoft Store to App Installer
   Start-Process "ms-windows-store://pdp/?ProductId=9NBLGGH4NNS1"
   ```
   
   Option B - Direct download:
   ```powershell
   # Download from GitHub releases
   Invoke-WebRequest -Uri "https://aka.ms/getwinget" -OutFile winget.msixbundle
   Add-AppxPackage winget.msixbundle
   ```

2. **Update App Installer**
   - Open Microsoft Store
   - Search for "App Installer"
   - Click "Update" if available

3. **Check PATH for winget**
   ```powershell
   # Test winget availability
   winget --version
   
   # If not found, check typical location
   & "$env:LOCALAPPDATA\Microsoft\WindowsApps\winget.exe" --version
   ```

4. **Restart your terminal**
   - After installing winget, restart PowerShell or Terminal
   - Environment variables may need to refresh

---

## 2. Package Installation Issues

### Package not found

**Symptoms:**
- Error: "No package found matching input criteria"
- Error: "Package 'X' not found in any source"

**Solutions:**

1. **Verify package ID**
   ```powershell
   # Search for the correct package ID
   winget search <package-name>
   
   # Get exact ID
   winget search --exact "Package Name"
   ```

2. **Check package source**
   ```yaml
   # In workload.yaml, specify the source
   packages:
     winget:
       - id: SomeApp.App
         source: msstore    # or winget
   ```

3. **Update winget sources**
   ```powershell
   winget source update
   ```

4. **List available sources**
   ```powershell
   winget source list
   ```

---

### Installation hangs

**Symptoms:**
- Installation appears frozen
- No progress for extended time
- Terminal unresponsive

**Solutions:**

1. **Check for interactive prompts**
   - Some installers require user interaction
   - Run winforge without `--quiet` to see prompts
   
2. **Use silent install overrides**
   ```yaml
   packages:
     winget:
       - id: SomeApp.App
         override:
           - "--silent"
           - "--accept-license"
   ```

3. **Increase timeout**
   - Some packages take longer to install
   - Cancel and retry with patience

4. **Check background processes**
   ```powershell
   # Look for installer processes
   Get-Process | Where-Object { $_.ProcessName -like "*install*" }
   ```

5. **Check Windows Update**
   - Pending Windows updates can block installations
   - Complete any pending updates and restart

---

### Version mismatch

**Symptoms:**
- Error: "Version X not found"
- Installed version differs from specified

**Solutions:**

1. **Check available versions**
   ```powershell
   winget show <package-id> --versions
   ```

2. **Remove version pin if not critical**
   ```yaml
   packages:
     winget:
       # Remove version to use latest
       - id: Package.Name
         # version: "1.2.3"  # Commented out
   ```

3. **Use version ranges if supported**
   - Some packages support partial version matching

---

### Access denied

**Symptoms:**
- Error: "Access is denied"
- Error: "Administrator privileges required"
- Installation fails with permission errors

**Solutions:**

1. **Run as Administrator**
   ```powershell
   # Start elevated PowerShell
   Start-Process powershell -Verb RunAs
   
   # Then run winforge
   winforge install my-workload
   ```

2. **Check installation scope**
   ```yaml
   packages:
     winget:
       - id: Package.Name
         override:
           - "--scope"
           - "user"    # Install for current user only
   ```

3. **Verify UAC settings**
   - Check User Account Control settings
   - Ensure UAC is not blocking installations

---

## 3. File Operation Issues

### Permission denied

**Symptoms:**
- Error: "Permission denied" when copying files
- Error: "Access to the path is denied"
- File operations fail

**Solutions:**

1. **Check target directory permissions**
   ```powershell
   # Check permissions on directory
   Get-Acl "C:\path\to\directory" | Format-List
   ```

2. **Run as administrator for system paths**
   - Files in `C:\Program Files`, `C:\Windows`, etc. require elevation

3. **Verify file isn't locked**
   ```powershell
   # Check if file is in use
   # Close any applications using the file
   
   # Or use handle.exe from Sysinternals
   handle.exe "filename"
   ```

4. **Use user-writable paths**
   ```yaml
   files:
     - source: config.json
       destination: "~/.config/app/config.json"  # User directory
   ```

---

### File not found

**Symptoms:**
- Error: "Source file not found"
- Error: "Cannot find path"

**Solutions:**

1. **Verify source path in workload**
   ```yaml
   files:
     # Path is relative to workload directory
     - source: files/config.json      # Correct: relative path
       destination: "~/.config/app/config.json"
   ```

2. **Check workload directory structure**
   ```
   my-workload/
   ├── workload.yaml
   └── files/              # Source files go here
       └── config.json
   ```

3. **List workload contents**
   ```powershell
   # Navigate to workload directory
   Get-ChildItem -Recurse
   ```

4. **Use absolute paths if needed**
   - For files outside workload directory
   - Not recommended for portable workloads

---

### Backup failed

**Symptoms:**
- Error: "Failed to create backup"
- Error: "Backup directory not accessible"

**Solutions:**

1. **Check disk space**
   ```powershell
   Get-PSDrive C | Select-Object Used, Free
   ```

2. **Verify backup directory permissions**
   ```powershell
   # Default backup location
   Test-Path "$env:APPDATA\winforge\backups"
   
   # Create if missing
   New-Item -ItemType Directory -Path "$env:APPDATA\winforge\backups" -Force
   ```

3. **Check path length**
   - Windows has a 260 character path limit by default
   - Use shorter paths or enable long paths:
   ```powershell
   # Enable long paths (requires admin)
   Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1
   ```

4. **Specify custom backup location**
   ```powershell
   $env:WINFORGE_BACKUP_DIR = "D:\Backups\winforge"
   ```

---

## 4. Script Execution Issues

### Script won't execute

**Symptoms:**
- Error: "Script execution failed"
- Script doesn't run at all
- No output from script

**Solutions:**

1. **Check script path**
   ```yaml
   scripts:
     post_install:
       # Path relative to workload directory
       - path: scripts/setup.ps1
   ```

2. **Verify script syntax**
   ```powershell
   # Check for syntax errors
   powershell -File "path\to\script.ps1" -WhatIf
   ```

3. **Test script manually**
   ```powershell
   # Run the script directly
   & "C:\workloads\my-workload\scripts\setup.ps1"
   ```

---

### Execution policy error

**Symptoms:**
- Error: "Running scripts is disabled on this system"
- Error: "cannot be loaded because the execution of scripts is disabled"

**Solutions:**

1. **Set execution policy**
   ```powershell
   # For current user (recommended)
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   
   # For current session only
   Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process
   ```

2. **Use bypass in workload**
   ```yaml
   scripts:
     post_install:
       - path: scripts/setup.ps1
         shell: powershell
         # Winforge handles execution policy internally
   ```

3. **Check group policy restrictions**
   - Corporate environments may have stricter policies
   - Contact your IT administrator

---

### Script timeout

**Symptoms:**
- Error: "Script execution timed out"
- Script killed before completion

**Solutions:**

1. **Increase timeout in workload**
   ```yaml
   scripts:
     post_install:
       - path: scripts/long-running.ps1
         timeout: 1800    # 30 minutes
   ```

2. **Check for infinite loops**
   - Review script for loops that may not terminate
   - Add progress indicators

3. **Verify external dependencies**
   - Network operations may be slow
   - Add retry logic with timeouts:
   ```powershell
   $maxRetries = 3
   $retryCount = 0
   while ($retryCount -lt $maxRetries) {
       try {
           # Operation here
           break
       }
       catch {
           $retryCount++
           Start-Sleep -Seconds 5
       }
   }
   ```

---

### Elevated script fails

**Symptoms:**
- Error: "Elevation required"
- Error: "Access denied" in elevated script
- UAC prompt doesn't appear

**Solutions:**

1. **Run winforge as administrator**
   ```powershell
   Start-Process powershell -Verb RunAs -ArgumentList "winforge install my-workload"
   ```

2. **Check UAC settings**
   - Open "User Account Control settings"
   - Ensure UAC is not disabled

3. **Verify elevated flag in workload**
   ```yaml
   scripts:
     post_install:
       - path: scripts/admin-setup.ps1
         elevated: true
   ```

4. **Test elevation manually**
   ```powershell
   # Test if running elevated
   ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
   ```

---

## 5. Health Check Issues

### False failures

**Symptoms:**
- Health check fails but software is working
- Intermittent failures
- Version mismatch reported incorrectly

**Solutions:**

1. **Review health check scripts**
   - Scripts may have outdated checks
   - Verify expected state matches reality

2. **Update version expectations**
   ```powershell
   # Check actual installed version
   winget list --id Package.Name
   ```

3. **Run with verbose output**
   ```powershell
   winforge health my-workload --verbose
   ```

4. **Check script logic**
   ```powershell
   # Example: Check for app in multiple locations
   $paths = @(
       "$env:LOCALAPPDATA\Programs\app.exe",
       "C:\Program Files\App\app.exe",
       "C:\Program Files (x86)\App\app.exe"
   )
   $found = $paths | Where-Object { Test-Path $_ } | Select-Object -First 1
   ```

---

### Partial results

**Symptoms:**
- Some checks don't run
- Health report incomplete
- "Skipped" status on some checks

**Solutions:**

1. **Review verbose output**
   ```powershell
   winforge health my-workload -vvv
   ```

2. **Check for script errors**
   - Earlier script failures may prevent later checks
   - Set `continue_on_error: true` for non-critical checks

3. **Verify all scripts exist**
   ```powershell
   # List all scripts in workload
   winforge show my-workload | Select-String "script"
   ```

---

## 6. Configuration Issues

### Config file not loading

**Symptoms:**
- Settings not applied
- Default values used instead of custom
- Error: "Failed to load configuration"

**Solutions:**

1. **Check file location**
   ```powershell
   # Default location
   Test-Path "$env:APPDATA\winforge\config.toml"
   
   # Or check environment variable
   $env:WINFORGE_CONFIG
   ```

2. **Validate TOML syntax**
   ```powershell
   # View config file
   Get-Content "$env:APPDATA\winforge\config.toml"
   
   # Common issues: missing quotes, invalid characters
   ```

3. **Reset configuration**
   ```powershell
   winforge config reset
   ```

4. **View current configuration**
   ```powershell
   winforge config show
   ```

---

### Workloads not found

**Symptoms:**
- Error: "Workload 'X' not found"
- Empty list from `winforge list`
- Custom workloads not discovered

**Solutions:**

1. **Check search paths**
   ```powershell
   winforge config show
   ```

2. **Verify workload directory structure**
   ```
   workload-name/
   └── workload.yaml    # Required file
   ```

3. **Use explicit path**
   ```powershell
   winforge list --path C:\MyWorkloads
   winforge install my-workload --path C:\MyWorkloads
   ```

4. **Configure additional paths**
   ```powershell
   winforge config set workload_paths "C:\Workloads;D:\MoreWorkloads"
   ```

5. **Check file permissions**
   - Ensure winforge can read the workload directories

---

## 7. Error Reference

### Common Error Codes

| Error | Description | Solution |
|-------|-------------|----------|
| `E001` | Workload not found | Check name and search paths |
| `E002` | Invalid workload schema | Run `winforge validate` for details |
| `E003` | Circular dependency | Review `extends` chain in workloads |
| `E004` | Package installation failed | Check winget logs, verify package ID |
| `E005` | File operation failed | Check permissions, verify paths |
| `E006` | Script execution failed | Review script output, check syntax |
| `E007` | Health check failed | Review check results, update scripts |
| `E008` | Configuration error | Validate config file syntax |
| `E009` | Backup operation failed | Check disk space and permissions |
| `E010` | Restore operation failed | Verify backup exists and is valid |

### Detailed Error Messages

#### E001: Workload not found
```
Error: Workload 'my-workload' not found

Searched in:
  - C:\Users\user\AppData\Roaming\winforge\workloads
  - C:\Program Files\winforge\workloads
```

**Solution:** 
- Verify workload name spelling
- Check if workload directory contains `workload.yaml`
- Use `--path` to specify custom location

#### E002: Invalid workload schema
```
Error: Invalid workload schema

Validation errors:
  - Line 5: 'name' field is required
  - Line 12: Invalid package ID format
```

**Solution:**
- Run `winforge validate my-workload` for full details
- Fix reported schema errors
- Refer to [Workload Authoring Guide](WORKLOAD_AUTHORING.md)

#### E003: Circular dependency
```
Error: Circular dependency detected

Dependency chain:
  workload-a -> workload-b -> workload-c -> workload-a
```

**Solution:**
- Review `extends` fields in each workload
- Remove circular references
- Restructure inheritance hierarchy

---

## 8. Getting Help

### Verbose Output

Get more detailed information about what Winforge is doing:

```powershell
# Increasing levels of verbosity
winforge -v <command>      # Some detail
winforge -vv <command>     # More detail  
winforge -vvv <command>    # Maximum detail (debug level)
```

### Enable Logging

Set environment variable for persistent logging:

```powershell
# Enable debug logging
$env:WINFORGE_LOG = "debug"
winforge install my-workload

# Available levels: error, warn, info, debug, trace
```

### Save Output to File

```powershell
# Capture output for sharing
winforge -vvv install my-workload 2>&1 | Tee-Object -FilePath "winforge-output.log"
```

### System Information

Gather helpful system information:

```powershell
# Winforge version
winforge --version

# Windows version
[System.Environment]::OSVersion

# PowerShell version
$PSVersionTable.PSVersion

# Winget version
winget --version
```

### Reporting Issues

When reporting issues on GitHub, include:

1. **Winforge version**: `winforge --version`
2. **Windows version**: Output of `winver` or `[System.Environment]::OSVersion`
3. **Command that failed**: Exact command you ran
4. **Error message**: Complete error output
5. **Verbose output**: Run with `-vvv` flag
6. **Workload file**: Contents of `workload.yaml` (sanitized if needed)
7. **Steps to reproduce**: Minimal steps to trigger the issue

**Issue Template:**

```markdown
## Environment
- Winforge version: 
- Windows version: 
- PowerShell version: 

## Description
Brief description of the issue.

## Command
```
winforge <command>
```

## Expected Behavior
What you expected to happen.

## Actual Behavior
What actually happened.

## Verbose Output
```
(paste -vvv output here)
```

## Workload (if applicable)
```yaml
(paste workload.yaml here)
```
```

### Resources

- [GitHub Issues](https://github.com/javierfe_microsoft/winforge/issues)
- [User Guide](USER_GUIDE.md)
- [Workload Authoring Guide](WORKLOAD_AUTHORING.md)
- [Winget Documentation](https://learn.microsoft.com/en-us/windows/package-manager/)

---

## Quick Reference

### Common Commands for Troubleshooting

```powershell
# Check Winforge installation
winforge --version

# Validate workload
winforge validate my-workload --strict

# Preview installation (dry run)
winforge install my-workload --dry-run

# Verbose health check
winforge health my-workload -vvv

# View current configuration
winforge config show

# Reset configuration
winforge config reset

# List workloads in custom path
winforge list --path C:\MyWorkloads
```

### Environment Variables

```powershell
# Custom config file
$env:WINFORGE_CONFIG = "C:\path\to\config.toml"

# Additional workload paths
$env:WINFORGE_WORKLOADS = "C:\Workloads;D:\More"

# Debug logging
$env:WINFORGE_LOG = "debug"

# Disable colors
$env:NO_COLOR = "1"

# Custom backup directory
$env:WINFORGE_BACKUP_DIR = "D:\Backups"
```

---

*This guide is for Winforge v0.3.1. For other versions, check the corresponding documentation.*