# Winforge

[![CI](https://github.com/javierfe_microsoft/winforge/actions/workflows/ci.yml/badge.svg)](https://github.com/javierfe_microsoft/winforge/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/javierfe_microsoft/winforge)](https://github.com/javierfe_microsoft/winforge/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

**Windows Workstation Configuration Management System**

Winforge is a declarative configuration management tool for Windows workstations. Define your development environment in YAML, and Winforge will install packages, copy configuration files, run setup scripts, and verify system health.

## ✨ Features

- 📦 **Package Management** - Install software via winget with version pinning
- 📁 **File Synchronization** - Copy configuration files with automatic backup
- 🔧 **Script Execution** - Run PowerShell setup and validation scripts
- 🧬 **Workload Inheritance** - Compose configurations using DRY principles
- ✅ **Health Checks** - Validate system state matches workload definition
- 📊 **Multiple Formats** - Output as table, JSON, YAML, or HTML reports
- 🔄 **Backup & Restore** - Save and restore system state
- 🐚 **Shell Completions** - Tab completion for PowerShell, Bash, Zsh, Fish

## 🚀 Quick Start

### Installation

**Option 1: Download from Releases**

```powershell
# Download latest release
Invoke-WebRequest -Uri "https://github.com/javierfe_microsoft/winforge/releases/latest/download/winforge-windows-x64.zip" -OutFile winforge.zip
Expand-Archive winforge.zip -DestinationPath C:\Tools\winforge
$env:PATH += ";C:\Tools\winforge"
```

**Option 2: Build from Source**

```powershell
# Prerequisites: Rust 1.75+
git clone https://github.com/javierfe_microsoft/winforge.git
cd winforge
cargo build --release
# Binary is at target/release/winforge.exe
```

### Basic Usage

```powershell
# List available workloads
winforge list

# Preview what would happen
winforge install rust-developer --dry-run

# Install a workload
winforge install rust-developer

# Check system health
winforge health rust-developer

# Generate HTML health report
winforge health rust-developer --output html --file report.html
```

## 📦 Bundled Workloads

| Workload | Description |
|----------|-------------|
| `dev-tools-base` | VS Code, Git, Windows Terminal, Oh My Posh |
| `rust-developer` | Rust toolchain with cargo tools |
| `python-developer` | Python 3.12 with uv package manager |
| `essentials` | Essential Windows utilities |

## 📋 Workload Structure

A workload is a configuration bundle:

```
my-workload/
├── workload.yaml       # Workload definition
├── files/              # Configuration files to deploy
└── scripts/            # Installation and health scripts
```

### Example Workload

```yaml
name: rust-developer
version: "1.0.0"
description: "Complete Rust development environment"

extends:
  - dev-tools-base

packages:
  winget:
    - id: Rustlang.Rustup
    - id: LLVM.LLVM

files:
  - source: config.toml
    destination: "~/.cargo/config.toml"
    backup: true

scripts:
  post_install:
    - path: scripts/setup.ps1
      description: "Install Rust components"
      
  health_check:
    - path: scripts/health.ps1
      name: "Rust Toolchain"
```

## 🔧 CLI Reference

```
winforge <COMMAND>

Commands:
  install      Apply a workload configuration
  health       Validate system against workload
  list         List available workloads
  show         Display workload details
  validate     Validate workload syntax
  init         Create new workload template
  status       Show installation status
  backup       Manage system backups
  config       Manage global configuration
  completions  Generate shell completions

Global Options:
  -v, --verbose    Increase verbosity (-v, -vv, -vvv)
  -q, --quiet      Suppress output
  --no-color       Disable colored output
  -h, --help       Show help
  -V, --version    Show version
```

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [User Guide](docs/USER_GUIDE.md) | Complete usage instructions |
| [Workload Authoring](docs/WORKLOAD_AUTHORING.md) | Creating custom workloads |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Common issues and solutions |
| [Specification](docs/SPECIFICATION.md) | Technical architecture |
| [Contributing](CONTRIBUTING.md) | Contribution guidelines |
| [Changelog](CHANGELOG.md) | Version history |

## ⚙️ Requirements

- Windows 10 (version 1809+) or Windows 11
- [Windows Package Manager (winget)](https://github.com/microsoft/winget-cli)
- PowerShell 5.1 or later

## 🛠️ Building from Source

### Prerequisites

- Rust 1.75 or later
- Visual Studio Build Tools (for Windows linking)

### Build

```powershell
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with verbose output
cargo run -- -vvv list
```

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on:

- Reporting bugs
- Suggesting features
- Submitting pull requests
- Creating workloads

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [winget](https://github.com/microsoft/winget-cli) - Windows Package Manager
- [clap](https://github.com/clap-rs/clap) - Command line argument parser
- [serde](https://github.com/serde-rs/serde) - Serialization framework
- [handlebars](https://github.com/sunng87/handlebars-rust) - Template engine

---

<p align="center">
  Made with ❤️ for Windows developers
</p>