# Winforge - Windows Workstation Configuration Management System

## Specification Document

**Version:** 1.0.0  
**Status:** Draft  
**Last Updated:** 2025-01-15

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Technology Analysis](#2-technology-analysis)
3. [Technology Recommendation](#3-technology-recommendation)
4. [Project Structure](#4-project-structure)
5. [Workload Definition Schema](#5-workload-definition-schema)
6. [CLI Interface Design](#6-cli-interface-design)
7. [Core Components](#7-core-components)
8. [Implementation Roadmap](#8-implementation-roadmap)
9. [Appendices](#9-appendices)

---

## 1. Executive Summary

### 1.1 Purpose

Winforge is a Windows workstation configuration management system designed to automate the setup and validation of development environments. It provides a declarative approach to defining workstation configurations ("workloads") and offers both installation and health-check capabilities.

### 1.2 Core Modes of Operation

| Mode | Description |
|------|-------------|
| **Install** | Apply a workload configuration by installing packages, executing scripts, and copying files |
| **Health Check** | Validate the current system state against a workload definition |

### 1.3 Key Features

- **Declarative Configuration**: Define workloads in human-readable YAML files
- **Package Management**: Leverage winget for software installation
- **File Synchronization**: Copy configuration files to designated locations with integrity verification
- **Script Execution**: Run pre/post installation scripts and diagnostic validations
- **Workload Composition**: Inherit and extend base workloads for DRY configurations
- **Detailed Reporting**: Comprehensive logs and health reports

---

## 2. Technology Analysis

### 2.1 Evaluation Criteria

| Criterion | Weight | Description |
|-----------|--------|-------------|
| Windows Integration | 25% | Native Windows APIs, winget interoperability, registry access |
| Maintainability | 20% | Code clarity, testing support, community ecosystem |
| Extensibility | 15% | Plugin architecture, workload composition |
| Dependencies | 15% | Target machine requirements, deployment complexity |
| Performance | 10% | Startup time, execution speed |
| Error Handling | 15% | Exception management, detailed reporting |

### 2.2 Technology Comparison

#### 2.2.1 PowerShell Scripts

**Strengths:**
- Native Windows integration; first-class citizen on all modern Windows
- Direct winget command execution with native output parsing
- No additional runtime dependencies
- Excellent registry, file system, and Windows service access
- Built-in remoting capabilities

**Weaknesses:**
- Complex error handling patterns
- Limited testing frameworks compared to general-purpose languages
- Script organization becomes unwieldy at scale
- Type system limitations for complex data structures
- Inconsistent cross-version behavior (5.1 vs 7+)

**Dependency Requirements:** None (built into Windows)

**Winget Integration:** ★★★★★ Excellent - native command execution

**Score:** 72/100

---

#### 2.2.2 Python Scripts

**Strengths:**
- Rich ecosystem with excellent YAML/JSON/TOML parsing libraries
- Strong testing frameworks (pytest, unittest)
- Good readability and maintainability
- Cross-platform potential
- Excellent error handling with exceptions

**Weaknesses:**
- Requires Python runtime installation on target machines
- Virtual environment complexity for dependencies
- Subprocess management for winget feels indirect
- Windows-specific operations require additional libraries
- Version compatibility issues (3.x versions)

**Dependency Requirements:** Python 3.8+ runtime, pip packages

**Winget Integration:** ★★★☆☆ Good - via subprocess

**Score:** 68/100

---

#### 2.2.3 C# Script Files (dotnet-script)

**Strengths:**
- Strong typing with excellent IDE support
- Access to full .NET ecosystem
- Good Windows integration via .NET APIs
- Reasonable startup time for scripts
- NuGet package support

**Weaknesses:**
- Requires .NET SDK installation
- Less common tooling; smaller community
- Debugging experience inferior to full applications
- Script organization patterns less established
- Version pinning complexity

**Dependency Requirements:** .NET 6+ SDK, dotnet-script global tool

**Winget Integration:** ★★★☆☆ Good - via Process class

**Score:** 65/100

---

#### 2.2.4 C# Console Application

**Strengths:**
- Excellent type safety and refactoring support
- Comprehensive .NET ecosystem (DI, logging, configuration)
- Strong testing capabilities (xUnit, NUnit, MSTest)
- Native Windows API access via P/Invoke
- Can produce single-file executables
- Mature error handling with structured exceptions
- Excellent async/await support for parallel operations

**Weaknesses:**
- Requires .NET runtime (can be self-contained but increases size)
- Longer development cycle than scripting
- Compilation step required
- Larger deployment artifact if self-contained (~60MB+)

**Dependency Requirements:** .NET 8 runtime (or self-contained)

**Winget Integration:** ★★★★☆ Very Good - Process class with structured parsing

**Score:** 82/100

---

#### 2.2.5 Rust CLI Application

**Strengths:**
- Zero runtime dependencies; single static binary
- Excellent performance and minimal resource usage
- Strong type system with compile-time guarantees
- Superior error handling with Result<T, E> pattern
- Memory safety without garbage collection
- Excellent CLI frameworks (clap, structopt)
- Fast startup time

**Weaknesses:**
- Steeper learning curve
- Longer compilation times
- Smaller Windows-specific ecosystem
- Windows API interop more verbose than .NET
- Less familiar to typical Windows administrators

**Dependency Requirements:** None (static binary)

**Winget Integration:** ★★★★☆ Very Good - std::process::Command with structured parsing

**Score:** 85/100

---

### 2.3 Comparison Matrix

| Criterion | PowerShell | Python | C# Script | C# App | Rust |
|-----------|------------|--------|-----------|--------|------|
| Windows Integration | ★★★★★ | ★★★☆☆ | ★★★★☆ | ★★★★★ | ★★★★☆ |
| Maintainability | ★★★☆☆ | ★★★★☆ | ★★★☆☆ | ★★★★★ | ★★★★☆ |
| Extensibility | ★★★☆☆ | ★★★★☆ | ★★★☆☆ | ★★★★★ | ★★★★★ |
| Dependencies | ★★★★★ | ★★☆☆☆ | ★★☆☆☆ | ★★★★☆ | ★★★★★ |
| Performance | ★★★☆☆ | ★★★☆☆ | ★★★☆☆ | ★★★★☆ | ★★★★★ |
| Error Handling | ★★★☆☆ | ★★★★☆ | ★★★★☆ | ★★★★★ | ★★★★★ |
| **Weighted Score** | **72** | **68** | **65** | **82** | **85** |

---

## 3. Technology Recommendation

### 3.1 Primary Recommendation: Rust CLI Application

**Rationale:**

1. **Zero Dependencies**: The single static binary eliminates runtime requirements, crucial for bootstrapping fresh Windows installations where development tools aren't yet installed.

2. **Robust Error Handling**: Rust's `Result<T, E>` pattern enforces comprehensive error handling at compile time, preventing runtime surprises during critical system configuration.

3. **Performance**: Fast startup (~5ms) and execution makes the tool feel responsive, encouraging frequent health checks.

4. **Configuration Parsing**: Excellent libraries for YAML (`serde_yaml`), TOML (`toml`), and JSON (`serde_json`) with strong type safety.

5. **CLI Excellence**: The `clap` crate provides industry-leading CLI parsing with automatic help generation, shell completions, and argument validation.

6. **Future-Proof**: Rust's stability guarantees and backwards compatibility ensure long-term maintainability.

### 3.2 Configuration Format: YAML

**Rationale:**

- Human-readable and writable without tooling
- Supports comments for documentation
- Native multi-line string support for inline scripts
- Widely understood by developers and DevOps professionals
- Excellent library support across all evaluated languages

### 3.3 Secondary Components: PowerShell

For pre/post installation scripts and health check validators, PowerShell remains the optimal choice:

- Native Windows command execution
- No additional dependencies
- Familiar to Windows administrators
- Direct access to Windows-specific features

### 3.4 Architecture Decision Records (ADR)

| ADR | Decision | Rationale |
|-----|----------|-----------|
| ADR-001 | Use Rust for core CLI | Zero runtime deps, robust error handling |
| ADR-002 | Use YAML for workload definitions | Human-readable, supports comments |
| ADR-003 | Use PowerShell for scripts | Native Windows integration |
| ADR-004 | Use SHA-256 for file hashing | Industry standard, good performance |
| ADR-005 | Support workload inheritance | DRY principle, modular configurations |

---

## 4. Project Structure

### 4.1 Repository Layout

```
winforge/
├── .github/
│   └── workflows/
│       ├── ci.yml                    # Continuous integration
│       └── release.yml               # Release automation
├── docs/
│   ├── SPECIFICATION.md              # This document
│   ├── USER_GUIDE.md                 # End-user documentation
│   ├── WORKLOAD_AUTHORING.md         # Guide for creating workloads
│   └── TROUBLESHOOTING.md            # Common issues and solutions
├── src/
│   ├── main.rs                       # Application entry point
│   ├── cli/
│   │   ├── mod.rs                    # CLI module root
│   │   ├── commands.rs               # Command definitions
│   │   └── output.rs                 # Output formatting (table, JSON)
│   ├── config/
│   │   ├── mod.rs                    # Configuration module root
│   │   ├── workload.rs               # Workload struct definitions
│   │   ├── schema.rs                 # Schema validation
│   │   └── inheritance.rs            # Workload composition logic
│   ├── operations/
│   │   ├── mod.rs                    # Operations module root
│   │   ├── install.rs                # Installation operations
│   │   ├── health.rs                 # Health check operations
│   │   └── backup.rs                 # Backup/restore operations
│   ├── providers/
│   │   ├── mod.rs                    # Providers module root
│   │   ├── winget.rs                 # Winget package manager
│   │   ├── filesystem.rs             # File operations
│   │   └── script.rs                 # Script execution
│   ├── hashing/
│   │   ├── mod.rs                    # Hashing module root
│   │   └── sha256.rs                 # SHA-256 implementation
│   └── reporting/
│       ├── mod.rs                    # Reporting module root
│       ├── console.rs                # Console output
│       ├── json.rs                   # JSON report generation
│       └── html.rs                   # HTML report generation
├── workloads/
│   ├── essentials/
│   │   ├── workload.yaml             # Core dev tools (VS Code, Git, Terminal) and utilities
│   │   ├── files/
│   │   │   └── user/                 # User configuration files
│   │   └── scripts/
│   │       ├── health-check.ps1      # Essentials health validation
│   │       └── configure-sudo.ps1    # Windows sudo configuration
│   ├── rust-developer/
│   │   ├── workload.yaml             # Rust toolchain workload
│   │   ├── files/
│   │   │   └── config.toml           # Cargo config
│   │   └── scripts/
│   │       ├── post-install.ps1      # Rustup component installation
│   │       └── health-check.ps1      # Rust environment validation
│   └── python-developer/
│       ├── workload.yaml             # Python development workload
│       ├── files/
│       │   └── pip.ini               # Pip configuration
│       └── scripts/
│           ├── post-install.ps1      # Virtual environment setup
│           └── health-check.ps1      # Python environment validation
├── tests/
│   ├── integration/
│   │   ├── install_tests.rs          # Installation integration tests
│   │   └── health_tests.rs           # Health check integration tests
│   └── fixtures/
│       └── sample-workload/          # Test workload
├── Cargo.toml                        # Rust project manifest
├── Cargo.lock                        # Dependency lock file
├── README.md                         # Project overview
├── LICENSE                           # License file
└── .gitignore                        # Git ignore rules
```

### 4.2 Workload Directory Structure

Each workload follows a consistent structure:

```
<workload-name>/
├── workload.yaml           # Workload definition (required)
├── files/                  # Files to copy to target system (optional)
│   ├── .config/
│   │   └── app/
│   │       └── settings.json
│   └── .profile
└── scripts/                # Scripts for installation/validation (optional)
    ├── pre-install.ps1     # Runs before package installation
    ├── post-install.ps1    # Runs after package installation
    └── health-check.ps1    # Validation script for health mode
```

---

## 5. Workload Definition Schema

### 5.1 Schema Overview (YAML)

```yaml
# workload.yaml - Full Schema Reference

# Metadata (required)
name: string                    # Unique workload identifier
version: string                 # Semantic version (e.g., "1.0.0")
description: string             # Human-readable description

# Inheritance (optional)
extends:                        # List of parent workloads
  - string                      # Parent workload name

# Package Management (optional)
packages:
  winget:                       # Winget package definitions
    - id: string                # Winget package ID (required)
      version: string           # Specific version (optional, default: latest)
      source: string            # Package source (optional, default: winget)
      override: string[]        # Additional winget arguments (optional)
      
# File Management (optional)
files:
  - source: string              # Relative path from workload's files/ directory
    destination: string         # Absolute path or path with variables
    backup: boolean             # Backup existing file (optional, default: true)
    permissions: string         # File permissions (optional, future use)
    template: boolean           # Process as template (optional, default: false)
    
# Script Execution (optional)
scripts:
  pre_install:                  # Scripts to run before installation
    - path: string              # Relative path from workload's scripts/ directory
      shell: string             # Execution shell (optional, default: "powershell")
      elevated: boolean         # Require admin privileges (optional, default: false)
      timeout: integer          # Timeout in seconds (optional, default: 300)
      
  post_install:                 # Scripts to run after installation
    - path: string
      shell: string
      elevated: boolean
      timeout: integer
      
  health_check:                 # Scripts for health validation
    - path: string
      shell: string
      name: string              # Display name for the check
      description: string       # What this check validates
      
# Environment Configuration (optional)
environment:
  variables:                    # Environment variables to set
    - name: string              # Variable name
      value: string             # Variable value
      scope: string             # "user" or "machine" (default: "user")
      
  path_additions:               # Additions to PATH
    - string                    # Path to add

# Health Check Configuration (optional)
health:
  package_check: boolean        # Verify packages installed (default: true)
  file_check: boolean           # Verify files match (default: true)
  script_check: boolean         # Run health check scripts (default: true)
```

### 5.2 Example Workloads

#### 5.2.1 Essentials Workload

```yaml
# workloads/essentials/workload.yaml
name: essentials
version: "2.0.0"
description: "Core development tools and productivity utilities for everyday Windows use"

packages:
  winget:
    - id: Git.Git
      override:
        - --override
        - '/VERYSILENT /NORESTART /NOCANCEL /SP- /CLOSEAPPLICATIONS /RESTARTAPPLICATIONS /COMPONENTS="icons,ext\reg\shellhere,assoc,assoc_sh"'
    
    - id: Microsoft.VisualStudioCode
      override:
        - --override
        - '/VERYSILENT /NORESTART /MERGETASKS=!runcode,addcontextmenufiles,addcontextmenufolders,addtopath'
    
    - id: Microsoft.WindowsTerminal
    
    - id: JanDeDobbeleer.OhMyPosh
    
    - id: GitHub.cli

files:
  - source: user/.gitconfig
    destination: "~/.gitconfig"
    backup: true
    
  - source: user/.config/pwsh
    destination: "~/.config/pwsh"
    backup: true

scripts:
  post_install:
    - path: post-install.ps1
      shell: powershell
    - path: configure-sudo.ps1
      shell: powershell
      elevated: true
      
  health_check:
    - path: health-check.ps1
      name: "essentials Health Check"
      description: "Verifies essentials is properly configured"
    - path: verify-sudo.ps1
      name: "Windows Sudo"
      description: "Verifies Windows sudo is enabled and set to inline mode"
```

#### 5.2.2 Rust Developer Workload

```yaml
# workloads/rust-developer/workload.yaml
name: rust-developer
version: "1.0.0"
description: "Complete Rust development environment"

extends:
  - essentials

packages:
  winget:
    - id: Rustlang.Rustup
    
    - id: Microsoft.VisualStudio.2022.BuildTools
      override:
        - --override
        - '--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
    
    - id: LLVM.LLVM

files:
  - source: config.toml
    destination: "~/.cargo/config.toml"
    backup: true
    
  - source: rustfmt.toml
    destination: "~/rustfmt.toml"
    backup: false

scripts:
  post_install:
    - path: post-install.ps1
      shell: powershell
      description: "Install Rust components and common tools"
      timeout: 600
      
  health_check:
    - path: health-check.ps1
      name: "Rust Toolchain"
      description: "Verifies rustc, cargo, and components are properly installed"

environment:
  path_additions:
    - "~/.cargo/bin"
```

#### 5.2.3 Python Developer Workload

```yaml
# workloads/python-developer/workload.yaml
name: python-developer
version: "1.0.0"
description: "Python development environment with common tools"

extends:
  - essentials

packages:
  winget:
    - id: Python.Python.3.12
    
    - id: astral-sh.uv

files:
  - source: pip.ini
    destination: "~/AppData/Roaming/pip/pip.ini"
    backup: true

scripts:
  post_install:
    - path: post-install.ps1
      shell: powershell
      description: "Configure Python environment and install global tools"
      
  health_check:
    - path: health-check.ps1
      name: "Python Environment"
      description: "Verifies Python, pip, and uv are properly configured"

environment:
  variables:
    - name: PYTHONDONTWRITEBYTECODE
      value: "1"
      scope: user
```

### 5.3 Variable Expansion

Winforge supports variable expansion in destination paths and template files:

| Variable | Expansion |
|----------|-----------|
| `~` | User's home directory (`%USERPROFILE%`) |
| `${HOME}` | User's home directory |
| `${APPDATA}` | Application data directory |
| `${LOCALAPPDATA}` | Local application data |
| `${PROGRAMFILES}` | Program Files directory |
| `${PROGRAMFILES_X86}` | Program Files (x86) directory |
| `${WINFORGE_WORKLOAD}` | Current workload name |
| `${WINFORGE_VERSION}` | Winforge version |

---

## 6. CLI Interface Design

### 6.1 Command Overview

```
winforge <COMMAND> [OPTIONS]

Commands:
  install     Apply a workload configuration to the system
  health      Check system health against a workload definition
  list        List available workloads
  show        Display detailed workload information
  validate    Validate workload definition syntax
  init        Initialize a new workload template
  backup      Backup current system state
  restore     Restore from a backup
  completions Generate shell completions
  help        Print help information

Options:
  -v, --verbose       Increase output verbosity (can be repeated: -vvv)
  -q, --quiet         Suppress non-essential output
  -c, --config <PATH> Use custom configuration file
  --no-color          Disable colored output
  -h, --help          Print help
  -V, --version       Print version
```

### 6.2 Command Details

#### 6.2.1 Install Command

```
winforge install [OPTIONS] <WORKLOAD>

Apply a workload configuration to the system

Arguments:
  <WORKLOAD>  Name of the workload to install (or path to workload.yaml)

Options:
  -d, --dry-run           Show what would be done without making changes
  -f, --force             Skip confirmation prompts
  -p, --packages-only     Only install packages, skip files and scripts
  --skip-packages         Skip package installation
  --skip-files            Skip file operations
  --skip-scripts          Skip script execution
  --skip-pre-scripts      Skip pre-installation scripts
  --skip-post-scripts     Skip post-installation scripts
  --no-backup             Don't backup existing files
  -j, --jobs <N>          Number of parallel package installations (default: 4)
  --timeout <SECONDS>     Global timeout for operations (default: 3600)
  
Examples:
  winforge install rust-developer
  winforge install ./custom-workload/workload.yaml
  winforge install python-developer --dry-run
  winforge install essentials --packages-only
```

#### 6.2.2 Health Command

```
winforge health [OPTIONS] <WORKLOAD>

Check system health against a workload definition

Arguments:
  <WORKLOAD>  Name of the workload to check against

Options:
  -o, --output <FORMAT>   Output format: table, json, yaml, html (default: table)
  -f, --file <PATH>       Write report to file instead of stdout
  --fail-fast             Stop on first failure
  --packages-only         Only check packages
  --files-only            Only check files
  --scripts-only          Only run health check scripts
  -s, --strict            Treat warnings as errors
  
Exit Codes:
  0  All checks passed
  1  One or more checks failed
  2  Configuration or runtime error

Examples:
  winforge health rust-developer
  winforge health python-developer --output json --file report.json
  winforge health essentials --fail-fast
```

#### 6.2.3 List Command

```
winforge list [OPTIONS]

List available workloads

Options:
  -a, --all               Include built-in and custom workloads
  -l, --long              Show detailed information
  --path <PATH>           Search for workloads in additional path
  -o, --output <FORMAT>   Output format: table, json, yaml (default: table)

Examples:
  winforge list
  winforge list --long
  winforge list --output json
```

#### 6.2.4 Show Command

```
winforge show <WORKLOAD>

Display detailed workload information

Arguments:
  <WORKLOAD>  Name of the workload to display

Options:
  -r, --resolved          Show resolved configuration (with inheritance applied)
  -o, --output <FORMAT>   Output format: yaml, json (default: yaml)

Examples:
  winforge show rust-developer
  winforge show python-developer --resolved
```

#### 6.2.5 Validate Command

```
winforge validate [OPTIONS] <PATH>

Validate workload definition syntax

Arguments:
  <PATH>  Path to workload.yaml file or workload directory

Options:
  --strict               Enable strict validation mode
  --schema               Output JSON schema for workload definitions

Examples:
  winforge validate workloads/rust-developer/
  winforge validate ./custom-workload/workload.yaml --strict
```

#### 6.2.6 Init Command

```
winforge init [OPTIONS] <NAME>

Initialize a new workload template

Arguments:
  <NAME>  Name for the new workload

Options:
  -t, --template <NAME>   Base template: minimal, standard, full (default: standard)
  -e, --extends <PARENT>  Parent workload to extend
  -o, --output <PATH>     Output directory (default: ./workloads/<NAME>)

Examples:
  winforge init my-workload
  winforge init frontend-dev --extends essentials
  winforge init minimal-setup --template minimal
```

### 6.3 Output Formats

#### 6.3.1 Table Format (Default)

```
$ winforge health rust-developer

╭─────────────────────────────────────────────────────────────────────╮
│                    Winforge Health Check Report                      │
│                    Workload: rust-developer                          │
├─────────────────────────────────────────────────────────────────────┤
│ Component          │ Status  │ Details                              │
├────────────────────┼─────────┼──────────────────────────────────────┤
│ Packages                                                             │
├────────────────────┼─────────┼──────────────────────────────────────┤
│ Git.Git            │ ✓ OK    │ 2.43.0 installed                     │
│ Microsoft.VSCode   │ ✓ OK    │ 1.85.0 installed                     │
│ Rustlang.Rustup    │ ✓ OK    │ 1.26.0 installed                     │
│ LLVM.LLVM          │ ✗ FAIL  │ Not installed                        │
├────────────────────┼─────────┼──────────────────────────────────────┤
│ Files                                                                │
├────────────────────┼─────────┼──────────────────────────────────────┤
│ ~/.cargo/config    │ ✓ OK    │ Hash matches                         │
│ ~/.gitconfig       │ ⚠ WARN  │ Modified locally                     │
├────────────────────┼─────────┼──────────────────────────────────────┤
│ Health Scripts                                                       │
├────────────────────┼─────────┼──────────────────────────────────────┤
│ Rust Toolchain     │ ✓ OK    │ All components verified              │
│ Git Configuration  │ ✓ OK    │ User configured                      │
╰─────────────────────────────────────────────────────────────────────╯

Summary: 6 passed, 1 failed, 1 warning
```

#### 6.3.2 JSON Format

```json
{
  "workload": "rust-developer",
  "timestamp": "2025-01-15T10:30:00Z",
  "overall_status": "FAILED",
  "summary": {
    "total": 8,
    "passed": 6,
    "failed": 1,
    "warnings": 1
  },
  "checks": {
    "packages": [
      {
        "id": "Git.Git",
        "status": "OK",
        "expected_version": null,
        "installed_version": "2.43.0"
      },
      {
        "id": "LLVM.LLVM",
        "status": "FAILED",
        "expected_version": null,
        "installed_version": null,
        "error": "Package not installed"
      }
    ],
    "files": [
      {
        "source": ".cargo/config.toml",
        "destination": "C:\\Users\\dev\\.cargo\\config.toml",
        "status": "OK",
        "expected_hash": "sha256:abc123...",
        "actual_hash": "sha256:abc123..."
      }
    ],
    "scripts": [
      {
        "name": "Rust Toolchain",
        "status": "OK",
        "output": "rustc 1.75.0, cargo 1.75.0"
      }
    ]
  }
}
```

---

## 7. Core Components

### 7.1 Component Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                              CLI Layer                               │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────┐   │
│  │ install │ │ health  │ │  list   │ │  show   │ │  validate   │   │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └──────┬──────┘   │
└───────┼──────────┼────────────┼──────────┼─────────────┼───────────┘
        │          │            │          │             │
┌───────┴──────────┴────────────┴──────────┴─────────────┴───────────┐
│                          Operations Layer                           │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────┐    │
│  │ InstallOperation │  │ HealthOperation  │  │ BackupOperation│    │
│  └────────┬─────────┘  └────────┬─────────┘  └───────┬────────┘    │
└───────────┼─────────────────────┼────────────────────┼─────────────┘
            │                     │                    │
┌───────────┴─────────────────────┴────────────────────┴─────────────┐
│                          Providers Layer                            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │ WingetProvider  │  │ FilesystemProv. │  │ ScriptProvider  │     │
│  │                 │  │                 │  │                 │     │
│  │ - install()     │  │ - copy()        │  │ - execute()     │     │
│  │ - uninstall()   │  │ - compare()     │  │ - validate()    │     │
│  │ - list()        │  │ - backup()      │  │                 │     │
│  │ - search()      │  │ - restore()     │  │                 │     │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
└────────────────────────────────────────────────────────────────────┘
            │                     │                    │
┌───────────┴─────────────────────┴────────────────────┴─────────────┐
│                          Config Layer                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │ WorkloadParser  │  │ SchemaValidator │  │ InheritanceRes. │     │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
└────────────────────────────────────────────────────────────────────┘
```

### 7.2 Winget Provider

The Winget Provider interfaces with Windows Package Manager:

**Key Functions:**

| Function | Description |
|----------|-------------|
| `install(package)` | Install a package with optional version/overrides |
| `uninstall(package)` | Remove an installed package |
| `is_installed(package)` | Check if a package is installed |
| `get_version(package)` | Get installed version of a package |
| `list_installed()` | List all installed packages |
| `search(query)` | Search for packages |

**Error Handling:**

```rust
pub enum WingetError {
    PackageNotFound(String),
    InstallationFailed { package: String, exit_code: i32, stderr: String },
    VersionMismatch { package: String, expected: String, actual: String },
    NetworkError(String),
    AccessDenied(String),
    Timeout { package: String, timeout_seconds: u64 },
}
```

### 7.3 Filesystem Provider

Handles file operations with integrity verification:

**Key Functions:**

| Function | Description |
|----------|-------------|
| `copy_file(src, dest)` | Copy file with optional backup |
| `compute_hash(path)` | Calculate SHA-256 hash |
| `compare_files(a, b)` | Compare files by hash |
| `backup_file(path)` | Create timestamped backup |
| `restore_file(backup, dest)` | Restore from backup |
| `expand_path(path)` | Expand variables in path |

**Hash Format:**

```
sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

### 7.4 Script Provider

Executes PowerShell scripts with proper error handling:

**Key Functions:**

| Function | Description |
|----------|-------------|
| `execute(script, config)` | Run a script with configuration |
| `validate_syntax(script)` | Check script for syntax errors |
| `run_elevated(script)` | Execute with admin privileges |

**Script Execution Model:**

```rust
pub struct ScriptConfig {
    pub path: PathBuf,
    pub shell: Shell,           // PowerShell, Cmd, Bash (WSL)
    pub elevated: bool,
    pub timeout: Duration,
    pub working_dir: Option<PathBuf>,
    pub environment: HashMap<String, String>,
}

pub struct ScriptResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub success: bool,
}
```

### 7.5 Inheritance Resolution

Workloads can extend other workloads with the following merge rules:

| Field | Merge Strategy |
|-------|----------------|
| `name` | Child overwrites |
| `version` | Child overwrites |
| `description` | Child overwrites |
| `packages.winget` | Append (child packages added after parent) |
| `files` | Append (child files added, same destinations overwritten) |
| `scripts.pre_install` | Parent first, then child |
| `scripts.post_install` | Parent first, then child |
| `scripts.health_check` | Combine all |
| `environment.variables` | Child overwrites same-named variables |
| `environment.path_additions` | Append |

**Circular Dependency Detection:**

The inheritance resolver maintains a visited set to detect and reject circular dependencies:

```
A extends B extends C extends A  →  ERROR: Circular dependency detected
```

---

## 8. Implementation Roadmap

### 8.1 Phase Overview

```
Phase 1: Foundation (Week 1-2)
    │
    ├── Project scaffolding
    ├── CLI framework setup
    ├── Configuration parsing
    └── Basic workload validation
    
Phase 2: Core Operations (Week 3-4)
    │
    ├── Winget provider
    ├── Package installation
    ├── Basic health checks
    └── Console output formatting
    
Phase 3: File Management (Week 5-6)
    │
    ├── Filesystem provider
    ├── File copying with backup
    ├── Hash-based comparison
    └── Variable expansion
    
Phase 4: Script Execution (Week 7-8)
    │
    ├── Script provider
    ├── PowerShell integration
    ├── Pre/post install hooks
    └── Health check scripts
    
Phase 5: Advanced Features (Week 9-10)
    │
    ├── Workload inheritance
    ├── Multiple output formats
    ├── Backup/restore
    └── Shell completions
    
Phase 6: Polish & Release (Week 11-12)
    │
    ├── Documentation
    ├── Integration tests
    ├── CI/CD pipeline
    └── Initial release
```

### 8.2 Phase 1: Foundation

**Objectives:**
- Set up Rust project structure
- Implement CLI argument parsing
- Create workload YAML parser
- Implement schema validation

**Deliverables:**
- [ ] `Cargo.toml` with dependencies
- [ ] CLI structure with `clap`
- [ ] Workload struct definitions with `serde`
- [ ] YAML parsing with `serde_yaml`
- [ ] Basic `list` and `show` commands
- [ ] `validate` command

**Dependencies:**
```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
thiserror = "1"
anyhow = "1"
```

### 8.3 Phase 2: Core Operations

**Objectives:**
- Implement Winget provider
- Create install operation workflow
- Build basic health check functionality
- Design console output formatting

**Deliverables:**
- [ ] Winget command wrapper
- [ ] Package installation logic
- [ ] Package verification (is_installed, get_version)
- [ ] `install` command (packages only)
- [ ] `health` command (packages only)
- [ ] Table output with `comfy-table`

**Dependencies:**
```toml
[dependencies]
comfy-table = "7"
indicatif = "0.17"        # Progress bars
console = "0.15"          # Terminal colors/styling
```

### 8.4 Phase 3: File Management

**Objectives:**
- Implement filesystem provider
- Add file copying with backup
- Create hash-based file comparison
- Support variable expansion in paths

**Deliverables:**
- [ ] File copy operations
- [ ] SHA-256 hashing
- [ ] Backup creation (timestamped)
- [ ] Path variable expansion
- [ ] File health checks
- [ ] Extend `install` command for files
- [ ] Extend `health` command for files

**Dependencies:**
```toml
[dependencies]
sha2 = "0.10"
dirs = "5"                # Standard directories
glob = "0.3"              # Glob pattern matching
```

### 8.5 Phase 4: Script Execution

**Objectives:**
- Implement script provider
- Add PowerShell execution
- Create pre/post install hooks
- Add health check script execution

**Deliverables:**
- [ ] PowerShell script executor
- [ ] Elevated execution support
- [ ] Timeout handling
- [ ] Script output capture
- [ ] Pre/post install integration
- [ ] Health check script integration

**Key Implementation Notes:**
- Use `std::process::Command` for script execution
- Capture both stdout and stderr
- Implement timeout with thread-based watchdog
- Parse exit codes for success/failure determination

### 8.6 Phase 5: Advanced Features

**Objectives:**
- Implement workload inheritance
- Add multiple output formats
- Create backup/restore functionality
- Generate shell completions

**Deliverables:**
- [ ] Inheritance resolver
- [ ] Circular dependency detection
- [ ] JSON output format
- [ ] YAML output format
- [ ] HTML report generation
- [ ] System backup before install
- [ ] Restore from backup
- [ ] Shell completions (PowerShell, bash, zsh, fish)

**Dependencies:**
```toml
[dependencies]
clap_complete = "4"
```

### 8.7 Phase 6: Polish & Release

**Objectives:**
- Complete documentation
- Write integration tests
- Set up CI/CD
- Prepare initial release

**Deliverables:**
- [ ] User guide documentation
- [ ] Workload authoring guide
- [ ] Troubleshooting guide
- [ ] Integration test suite
- [ ] GitHub Actions CI workflow
- [ ] Release workflow (builds Windows binaries)
- [ ] README with installation instructions
- [ ] v0.1.0 release

---

## 9. Appendices

### 9.1 Sample Health Check Script

```powershell
# health-check.ps1 - Rust Developer Workload

$ErrorActionPreference = "Stop"
$exitCode = 0

function Test-Command {
    param([string]$Command, [string]$Name)
    
    try {
        $null = Get-Command $Command -ErrorAction Stop
        Write-Host "✓ $Name is available" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "✗ $Name is not available" -ForegroundColor Red
        return $false
    }
}

function Test-RustComponent {
    param([string]$Component)
    
    $installed = rustup component list --installed 2>&1
    if ($installed -match $Component) {
        Write-Host "✓ Rust component '$Component' is installed" -ForegroundColor Green
        return $true
    }
    else {
        Write-Host "✗ Rust component '$Component' is missing" -ForegroundColor Red
        return $false
    }
}

# Check core tools
if (-not (Test-Command "rustc" "Rust Compiler")) { $exitCode = 1 }
if (-not (Test-Command "cargo" "Cargo")) { $exitCode = 1 }
if (-not (Test-Command "rustup" "Rustup")) { $exitCode = 1 }

# Check Rust version
$rustVersion = rustc --version
Write-Host "Rust version: $rustVersion"

# Check essential components
if (-not (Test-RustComponent "rust-src")) { $exitCode = 1 }
if (-not (Test-RustComponent "rust-analyzer")) { $exitCode = 1 }
if (-not (Test-RustComponent "clippy")) { $exitCode = 1 }
if (-not (Test-RustComponent "rustfmt")) { $exitCode = 1 }

# Check cargo tools
if (-not (Test-Command "cargo-watch" "cargo-watch")) { 
    Write-Host "⚠ cargo-watch not installed (optional)" -ForegroundColor Yellow 
}

exit $exitCode
```

### 9.2 Sample Post-Install Script

```powershell
# post-install.ps1 - Rust Developer Workload

$ErrorActionPreference = "Stop"

Write-Host "Configuring Rust development environment..." -ForegroundColor Cyan

# Update Rust toolchain
Write-Host "Updating Rust toolchain..."
rustup update

# Install stable toolchain
Write-Host "Installing stable toolchain..."
rustup default stable

# Install essential components
Write-Host "Installing Rust components..."
rustup component add rust-src
rustup component add rust-analyzer
rustup component add clippy
rustup component add rustfmt

# Install common cargo tools
Write-Host "Installing cargo tools..."
cargo install cargo-watch
cargo install cargo-edit
cargo install cargo-expand

# Configure VS Code (if installed)
$vscodePath = Get-Command code -ErrorAction SilentlyContinue
if ($vscodePath) {
    Write-Host "Installing VS Code extensions for Rust..."
    code --install-extension rust-lang.rust-analyzer
    code --install-extension vadimcn.vscode-lldb
    code --install-extension serayuzgur.crates
}

Write-Host "Rust development environment configured successfully!" -ForegroundColor Green
```

### 9.3 Error Codes Reference

| Code | Name | Description |
|------|------|-------------|
| 0 | SUCCESS | Operation completed successfully |
| 1 | HEALTH_CHECK_FAILED | One or more health checks failed |
| 2 | CONFIG_ERROR | Configuration file error |
| 3 | WORKLOAD_NOT_FOUND | Specified workload does not exist |
| 4 | WINGET_ERROR | Winget operation failed |
| 5 | FILE_ERROR | File operation failed |
| 6 | SCRIPT_ERROR | Script execution failed |
| 7 | PERMISSION_ERROR | Insufficient permissions |
| 8 | NETWORK_ERROR | Network connectivity issue |
| 9 | TIMEOUT | Operation timed out |
| 10 | CIRCULAR_DEPENDENCY | Circular workload inheritance |

### 9.4 Glossary

| Term | Definition |
|------|------------|
| **Workload** | A named configuration bundle defining packages, files, and scripts |
| **Health Check** | Validation of system state against a workload definition |
| **Provider** | Component that interfaces with external systems (winget, filesystem, scripts) |
| **Inheritance** | Mechanism for workloads to extend and build upon other workloads |
| **Variable Expansion** | Replacement of placeholders like `~` or `${HOME}` with actual paths |

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-01-15 | Winforge Team | Initial specification |

---

*This specification serves as the authoritative reference for implementing Winforge. Subsequent implementation prompts should reference this document for architecture decisions, schemas, and interfaces.*