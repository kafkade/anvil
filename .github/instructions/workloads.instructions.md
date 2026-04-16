---
applyTo: "workloads/**/*"
---

# Anvil Workload Authoring Instructions

You are editing Anvil workloads. Follow these conventions exactly.

## Workload Directory Structure

```
workloads/<name>/
├── workload.yaml           # Required — workload definition
├── files/                  # Optional — config files to deploy
│   └── user/.config/...    # Mirror target directory structure
└── scripts/                # Optional — PowerShell scripts
    ├── post-install.ps1
    ├── configure-*.ps1     # Configuration scripts
    ├── install-*.ps1       # Installation scripts
    └── health-check/       # Health check scripts (essentials pattern)
        ├── health-utils.ps1      # Shared utilities (essentials only)
        ├── check-*.ps1           # Individual health checks
```

## workload.yaml Schema (New Format)

All workloads in `workloads/` use the new schema. Do NOT use the old-workloads format.

```yaml
# Required fields
name: workload-name           # kebab-case identifier
version: "1.0.0"              # Semantic version
description: "..."            # Human-readable description

# Optional: inherit from parent workloads
extends:
  - essentials

# Packages — grouped by manager
packages:
  winget:
    - id: Publisher.PackageName
    # Optional fields:
      source: msstore          # Only for MS Store apps
      override:                # Custom install args
        - --override
        - "/VERYSILENT /NORESTART"

# Files — config files to deploy
files:
  - source: user/.config/app     # Relative to workload dir
    destination: "~/.config/app"  # ~ expands to $env:USERPROFILE
    backup: true                  # Back up existing files before overwriting

# Scripts — PowerShell scripts to execute
scripts:
  pre_install:                    # Runs BEFORE package installation
    - path: pre-install.ps1       # Relative to scripts/ dir
      shell: powershell           # Default: powershell
      description: "..."
      timeout: 60                 # Seconds, default: 300
      elevated: false             # Requires admin, default: false

  post_install:                   # Runs AFTER package installation
    - path: post-install.ps1
      shell: powershell
      description: "..."
      timeout: 300
      elevated: true              # Set true for registry/system changes

  health_check:                   # Health validation scripts
    - path: health-check/check-something.ps1
      name: "Display Name"       # Required for health_check entries
      description: "..."

# Environment variables to set
environment:
  variables:
    - name: VAR_NAME
      value: "value"
      scope: user                 # user or machine

  path_additions:
    - "~/.cargo/bin"

# Health check configuration
health:
  package_check: true             # Verify packages installed
  file_check: true                # Verify files deployed
  script_check: true              # Run health_check scripts
```

### Key Schema Rules

- `scripts.health_check` entries require a `name` field (other script types don't)
- `shell` defaults to `"powershell"` — omit it unless using something else
- `elevated: true` requires the script to have `#Requires -RunAsAdministrator`
- File `source` paths are relative to the workload directory
- Script `path` values are relative to the `scripts/` directory
- Use `~` for user home directory in destinations (expands to `$env:USERPROFILE`)

## Health Check Script Pattern

Health check scripts in the essentials workload use shared utilities from `health-utils.ps1`. Other workloads define their own inline helpers.

### Using health-utils.ps1 (essentials workload)

```powershell
# check-something.ps1 - Something Health Check
# Exit codes:
#   0 - All checks passed
#   1 - One or more checks failed

$ErrorActionPreference = "Continue"
$exitCode = 0

# Import shared utilities
. "$PSScriptRoot\health-utils.ps1"

Write-HealthCheckHeader "Something Health Check"

# Section grouping
Write-HealthCheckSection "section name"

# Test-Check returns $true/$false, prints [PASS]/[FAIL]
if (-not (Test-Check "description of check" { <scriptblock returning $true/$false> })) {
    $exitCode = 1
}

Write-HealthCheckFooter -ExitCode $exitCode
exit $exitCode
```

### Available health-utils.ps1 Functions

- `Test-Check -Name "..." -Test { <scriptblock> }` — Runs test, prints `[PASS]`/`[FAIL]`, returns boolean
- `Test-AppxPackageInstalled -PackageName "..."` — Checks if an AppX/MS Store package is installed
- `Write-HealthCheckHeader -Name "..."` — Prints header with `=` separator
- `Write-HealthCheckSection -Title "..."` — Prints `Checking <title>...` in yellow
- `Write-HealthCheckFooter -ExitCode <int>` — Prints pass/fail summary with `-` separator

### Standalone Health Check Pattern (non-essentials workloads)

```powershell
$ErrorActionPreference = "Stop"
$script:exitCode = 0

function Write-CheckResult {
    param(
        [string]$Name,
        [string]$Status,  # "OK" or "FAIL"
        [string]$Message
    )
    switch ($Status) {
        "OK"   { Write-Host "  [PASS] $Name" -ForegroundColor Green -NoNewline }
        "FAIL" { Write-Host "  [FAIL] $Name" -ForegroundColor Red -NoNewline; $script:exitCode = 1 }
    }
    if ($Message) { Write-Host " - $Message" -ForegroundColor Gray } else { Write-Host "" }
}

# ... checks ...
exit $script:exitCode
```

### Common Test-Check Patterns

```powershell
# Command exists
Test-Check "Git is installed" { Get-Command git -ErrorAction Stop }

# File exists
Test-Check "Config deployed" { Test-Path "$env:USERPROFILE\.config\app\config.json" }

# Directory exists
Test-Check "Config dir deployed" { Test-Path "$env:USERPROFILE\.config\app" }

# Registry value
Test-Check "Setting enabled" {
    $val = (Get-ItemProperty -Path "HKCU:\...\Key" -Name "Value" -ErrorAction Stop).Value
    $val -eq 1
}

# JSON property check
Test-Check "Color scheme is set" {
    $settings = Get-Content $path -Raw | ConvertFrom-Json
    $settings.profiles.defaults.colorScheme -eq "Sorcerer"
}

# AppX package installed
Test-Check "App installed" { Test-AppxPackageInstalled -PackageName "Publisher.App" }

# File content matches source
Test-Check "Config matches source" {
    $source = Get-Content -Path $sourcePath -Raw
    $target = Get-Content -Path $targetPath -Raw -ErrorAction SilentlyContinue
    $source -eq $target
}
```

## Configure Script Pattern

Configuration scripts modify system or application settings. They run as post_install scripts.

### JSON Settings Modification (e.g., Windows Terminal)

```powershell
$ErrorActionPreference = "Stop"

# 1. Locate settings file (check multiple paths)
$settingsLocations = @(
    (Join-Path $env:LOCALAPPDATA "Packages\App_hash\LocalState\settings.json"),
    (Join-Path $env:LOCALAPPDATA "Packages\AppPreview_hash\LocalState\settings.json")
)
$settingsPath = $null
foreach ($path in $settingsLocations) {
    if (Test-Path $path) { $settingsPath = $path; break }
}
if (-not $settingsPath) { Write-Host "Settings not found."; exit 1 }

# 2. Read and parse (strip JSONC comments if needed)
$raw = Get-Content $settingsPath -Raw
$lines = ($raw -split "`n") | ForEach-Object {
    if ($_ -match '^\s*//') { '' } else { $_ }
}
$settings = ($lines -join "`n") | ConvertFrom-Json

# 3. Modify settings (add members if missing)
if (-not $settings.someProperty) {
    $settings | Add-Member -NotePropertyName "someProperty" -NotePropertyValue @()
}

# 4. Backup and save
Copy-Item $settingsPath "$settingsPath.bak" -Force
$settings | ConvertTo-Json -Depth 32 | Set-Content $settingsPath -Encoding UTF8
```

### Registry Modification

```powershell
#Requires -Version 5.1
#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

$RegPath = "HKLM:\SOFTWARE\Microsoft\..."

if (-not (Test-Path $RegPath)) {
    New-Item -Path $RegPath -Force | Out-Null
}
Set-ItemProperty -Path $RegPath -Name "ValueName" -Value 1 -Type DWord -Force

# Verify
$actual = (Get-ItemProperty -Path $RegPath -Name "ValueName").ValueName
if ($actual -eq 1) { Write-Host "  Verified" -ForegroundColor Green }
```

### Using jq for JSON Modification (alternative pattern)

Some workloads use `jq` for surgical JSON edits. This preserves formatting better than ConvertTo-Json but requires jq to be installed.

```powershell
$jqFilter = '.profiles.defaults.font = {"face": "Cascadia Code NF", "size": 12}'
$updated = Get-Content -Path $settingsPath -Raw | jq "$jqFilter"
Set-Content -Path $settingsPath -Value $updated -Encoding UTF8
```

## Script Conventions

### General Rules
- Use `$ErrorActionPreference = "Stop"` in configure/install scripts
- Use `$ErrorActionPreference = "Continue"` in health check scripts
- Exit code 0 = success, 1 = failure
- Always create backups before modifying settings files
- Print colored status messages: Cyan for headers, Yellow for sections, Green for success, Red for failure, Gray for details
- Use `Write-Host "  ..." -ForegroundColor ...` (2-space indent) for sub-items

### Script Header Template

```powershell
# script-name.ps1 - Brief Description
# Longer description of what the script does
#
# Exit codes:
#   0 - Success / All checks passed
#   1 - Failure / One or more checks failed
```

### Elevated Scripts
- Add `#Requires -RunAsAdministrator` at the top
- Set `elevated: true` in workload.yaml
- Used for: registry HKLM changes, font installation, system-wide settings

## Workload Inheritance

- Use `extends: [essentials]` for developer workloads that need core tools
- Child workloads inherit: packages (appended), files (appended), scripts (concatenated, parent first), env vars (child overrides same name)
- Max depth: 10, cycles are detected and rejected

## Adding to an Existing Workload — Checklist

When adding a new capability to a workload:

1. **Script**: Create `scripts/configure-<feature>.ps1` (or `scripts/<feature>.ps1`)
2. **Health check**: Create matching `scripts/health-check/check-<feature>.ps1` (essentials) or add checks to existing health-check script (other workloads)
3. **workload.yaml**: Add entry under `scripts.post_install` (with description, timeout, elevated if needed)
4. **workload.yaml**: Add entry under `scripts.health_check` (with name, path, description)
5. **Packages**: If the feature requires a new package, add it under `packages.winget`
6. **Files**: If config files need to be deployed, add them under `files` and place source files in `files/` directory
