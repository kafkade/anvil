# Winforge - Workstation Setup & Management System v2 (Preview)

A comprehensive automated workstation setup and management system built as a .NET console application. Winforge v2 is currently in **Preview Mode**, focusing on workload discovery, validation, and execution planning.

## 🚀 Current Status: Preview & Analysis

Winforge v2 is a complete rewrite of the legacy PowerShell-based system, moving to a robust .NET 8 architecture. The current release provides **Workload Analysis and Preview** capabilities, allowing you to:

- **🔍 Discover Workloads**: Automatically find and parse workload definitions
- **📋 Validate Configurations**: Check YAML syntax and structure
- **🔬 Analyze Impact**: Preview exactly what packages, scripts, and files would be installed
- **📊 Estimate Time**: Get execution time estimates based on workload complexity

> **Note**: Actual package installation and script execution features are currently being migrated from the v1 architecture. The current version runs in "Preview Mode" to safely validate your workloads without making system changes.

## 📋 Quick Start

### Prerequisites

- **OS**: Windows 10 (1809+) or Windows 11
- **.NET**: 8.0 Runtime or higher
- **PowerShell**: 5.1 or 7.0+ (for legacy script support)

### Running Winforge

1. **Clone the repository**

   ```powershell
   git clone https://github.com/your-org/winforge.git
   cd winforge
   ```

2. **Run the Application**

   **Using PowerShell Launcher (Recommended)**

   ```powershell
   .\winforge.ps1
   ```

   **Using Batch Launcher**

   ```cmd
   .\winforge.bat
   ```

   **Directly via .NET**

   ```powershell
   cd src
   dotnet run
   ```

3. **Interactive Mode**
   The application launches in interactive mode by default. Use the arrow keys to navigate the menu:
   - **Discover**: Scan for workloads and preview installation plans
   - **Validate**: (Coming Soon) Deep system compliance checking
   - **Report**: (Coming Soon) Generate audit reports
   - **Help**: View documentation and tips

## 🏗️ Architecture

Winforge v2 moves away from pure PowerShell scripts to a structured C# application:

- **Core**: .NET 8 Console Application
- **UI**: Interactive terminal UI using [Spectre.Console](https://spectreconsole.net/)
- **Configuration**: YAML-based workload definitions
- **Engine**: Modular services for package management, script execution, and file operations

### Directory Structure

- `src/`: Core C# application source code
- `workloads/`: Directory containing workload definitions (YAML) and scripts
- `v1/`: Legacy PowerShell implementation (reference only)
- `docs/`: Documentation

## 🛠️ Creating Workloads

Workloads are defined in simple YAML files. See `schemas/simple-workload-schema.md` for details.

Example `workload.yaml`:

```yaml
name: "My Development Setup"
description: "Personal development environment"
version: "1.0.0"

packages:
  - name: "Microsoft.VisualStudioCode"
    manager: "winget"
  - name: "git"
    manager: "chocolatey"

scripts:
  - name: "Setup Git"
    file: "scripts/setup-git.ps1"
```

## 📖 Documentation

- **[User Guide](docs/user-guide.md)**: Detailed usage instructions
- **[Developer Guide](docs/developer-guide.md)**: Architecture and contribution guide
- **[Migration Guide](docs/migration-guide.md)**: Moving from v1 to v2

## 🤝 Contributing

We welcome contributions! Please see the [Developer Guide](docs/developer-guide.md) for details on the architecture and development workflow.

## 📄 License

This project is licensed under the MIT License.
