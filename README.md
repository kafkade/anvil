# Anvil

[![CI](https://github.com/kafkade/anvil/actions/workflows/ci.yml/badge.svg)](https://github.com/kafkade/anvil/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/kafkade/anvil)](https://github.com/kafkade/anvil/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

**Windows Workstation Configuration Management System**

Anvil is a declarative configuration management tool for Windows workstations. Define your development environment in YAML, and Anvil will install packages, copy configuration files, run setup scripts, and verify system health.

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
Invoke-WebRequest -Uri "https://github.com/kafkade/anvil/releases/latest/download/anvil-windows-x64.zip" -OutFile anvil.zip
Expand-Archive anvil.zip -DestinationPath C:\Tools\anvil
$env:PATH += ";C:\Tools\anvil"
```

**Option 2: Build from Source**

```powershell
# Prerequisites: Rust 1.75+
git clone https://github.com/kafkade/anvil.git
cd anvil
cargo build --release
# Binary is at target/release/anvil.exe
```

### Basic Usage

```powershell
# List available workloads
anvil list

# Preview what would happen
anvil install rust-developer --dry-run

# Install a workload
anvil install rust-developer

# Check system health
anvil health rust-developer

# Generate HTML health report
anvil health rust-developer --output html --file report.html
```

## 📦 Bundled Workloads

| Workload | Description |
|----------|-------------|
| `essentials` | Core development tools (VS Code, Git, Windows Terminal) and productivity utilities |
| `rust-developer` | Rust toolchain with cargo tools (extends essentials) |
| `python-developer` | Python 3.12 with uv package manager (extends essentials) |

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
  - essentials

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
anvil <COMMAND>

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