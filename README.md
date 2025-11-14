# Winforge - Workstation Configuration Management System v2

A comprehensive automated workstation setup and configuration management system built as a .NET console application with PowerShell integration for Windows environments, featuring the new **Workload Schema v2** system.

## 🚀 Overview

Winforge provides automated installation, configuration, and compliance management for development workstations. The system now includes a revolutionary **Simple Workload Schema v2** that dramatically simplifies workload definition while maintaining enterprise-grade capabilities. It supports multiple workload types and includes robust validation and reporting capabilities using a compiled .NET console application with PowerShell integration.

## ✨ What's New in v2

### 🎯 Simple Workload Schema v2

- **Streamlined YAML format** - Clean, readable workload definitions
- **Multiple package managers** - winget, chocolatey, npm, pip, powershell support
- **External PowerShell scripts** - Better organization and IDE support
- **Built-in validation** - Comprehensive testing framework
- **JSON Schema validation** - IntelliSense and error checking

### 🔧 Key Improvements

- **90% simpler** workload definitions compared to v1
- **Better maintainability** with external scripts
- **Enhanced validation** with multiple test types
- **IDE support** with syntax highlighting and debugging
- **Proven implementation** with real-world examples

## 📁 Project Structure

```
winforge/
├── winforge.ps1                           # PowerShell launcher script
├── winforge.bat                           # Windows batch launcher
├── src/                                   # .NET Console Application Source
│   ├── Program.cs                        # Application entry point
│   ├── Winforge.csproj                   # Project file
│   ├── README.md                         # Development documentation
│   ├── Core/                             # Core application logic
│   │   └── WinforgeApplication.cs        # Main application coordinator
│   ├── Models/                           # Data models
│   │   ├── WorkloadModels.cs             # Workload configuration models
│   │   └── WorkloadMetadata.cs           # Workload metadata models
│   ├── Services/                         # Business logic services
│   │   └── WorkloadManager.cs            # Workload discovery and management
│   └── UI/                               # User interface components
│       ├── WinforgeUI.cs                 # Main UI controller
│       └── IconProvider.cs               # Text-based icon system
├── schemas/                               # Schema Definitions (New in v2)
│   ├── workload-schema-v2.json           # JSON Schema for validation
│   ├── workload-schema-design.md         # Design documentation
│   └── workload-schema-v2-specification.md # Complete specification
├── workloads/                             # Workload Definitions
│   ├── data-science/                      # Data science workload
│   │   ├── workload.yaml                 # Workload definition
│   │   └── scripts/                      # Data science scripts
│   │       ├── setup-datascience.ps1
│   │       ├── create-sample-notebooks.ps1
│   │       └── install-python-packages.ps1
│   ├── web-development/                   # Web development workload
│   │   ├── workload.yaml                 # Workload definition
│   │   └── scripts/                      # Web development scripts
│   │       ├── setup-webdev.ps1
│   │       ├── configure-npm.ps1
│   │       └── install-vscode-extensions.ps1
│   ├── frontend-development/              # Frontend development workload
│   │   ├── workload.yaml                 # Workload definition
│   │   └── scripts/                      # Frontend development scripts
│   │       ├── setup-frontend.ps1
│   │       ├── install-frontend-extensions.ps1
│   │       └── create-project-templates.ps1
│   └── archive/                           # Legacy workload formats
│       ├── data-science.yaml             # Legacy v1 format
│       └── web-development.yaml          # Legacy v1 format
├── templates/                             # Configuration Templates
│   ├── vscode-settings.json              # VS Code settings
│   ├── PowerShell-Profile.ps1            # PowerShell profile
│   └── gitconfig.template                # Git configuration
└── README.md                             # This file
```

## 🎯 Features

### Core Functionality

- **Automated Application Installation** - Support for winget, Chocolatey, npm, pip, and PowerShell Gallery
- **Simple Workload Definitions** - New v2 schema with clean YAML format
- **External Script Support** - PowerShell scripts with IDE support and debugging
- **JSON Schema Validation** - Built-in validation with error checking
- **Configuration Management** - Template-based configuration deployment
- **Compliance Validation** - Security and policy compliance checking
- **Comprehensive Reporting** - Detailed compliance and audit reports

### Workload Schema v2 Features

- **Simple Structure** - Required fields: name, description, version, author
- **Multiple Package Managers** - winget, chocolatey, npm, pip, powershell
- **File Management** - Copy configuration files with environment variable support
- **Script Integration** - External PowerShell scripts for custom setup
- **Built-in Testing** - Command, file, and registry validation tests
- **Schema Validation** - JSON Schema with IDE support

### Supported Package Managers

- **Windows Package Manager (winget)** - Microsoft's official package manager
- **Chocolatey** - Community-driven package manager
- **NPM** - Node.js package manager (with global flag support)
- **pip** - Python package installer
- **PowerShell Gallery** - PowerShell modules and scripts

### Available Workloads (v2 Format)

- **Data Science Environment** - Python, Jupyter, data science libraries, and analytics tools
- **Web Development Environment** - Node.js, modern web frameworks, and development tools
- **Frontend Development Environment** - React, Vue, TypeScript, and modern frontend tools
- **Custom Workloads** - Define your own workload configurations using the simple schema

## 🛠️ Prerequisites

### System Requirements

- Windows 10 version 1809+ or Windows 11
- .NET 6.0 SDK or higher
- PowerShell 5.1 or PowerShell Core 7.0+
- Administrator privileges for system-level installations

### Required Tools

- **.NET 8.0 Runtime** - For running the console application
- **Git** - For version control integration (recommended)

### Optional Components

- Windows Package Manager (winget) - Recommended
- Chocolatey - Alternative package manager
- Node.js and ajv-cli - For full JSON Schema validation

## 🚀 Quick Start

### 1. Clone or Download

```powershell
# Clone the repository
git clone https://github.com/your-org/winforge.git
cd winforge

# Or download and extract the ZIP file
```

### 2. Build or Run the Application

```powershell
# Build the application (if from source)
cd src
dotnet build

# Or run directly from source
dotnet run
```

### 3. Run Winforge

Choose one of these methods to run Winforge:

#### Option A: PowerShell Launcher (Recommended)

```powershell
# Interactive mode
.\winforge.ps1

# Install specific workload
.\winforge.ps1 -Action Install -Workload data-science

# Validate system compliance
.\winforge.ps1 -Action Validate -Profile Corporate
```

#### Option B: Batch Launcher

```cmd
# Interactive mode
.\winforge.bat

# With command line arguments
.\winforge.bat -action install -workload data-science
```

#### Option C: Direct Console Application Execution

```powershell
# Interactive mode
.\src\bin\Debug\net8.0\winforge.exe

# Or from source directory
cd src
dotnet run

# With command line arguments
cd src
dotnet run -- help
dotnet run -- interactive
```

## 📖 Workload Schema v2 Usage

### Simple Workload Format

Create workload files using the new simple YAML format:

```yaml
name: "My Development Environment"
description: "Essential tools for development work"
version: "1.0.0"
author: "Your Name"

packages:
  - name: "Microsoft.VisualStudioCode"
    manager: "winget"
  - name: "Git.Git"
    manager: "winget"
  - name: "typescript"
    manager: "npm"
    global: true

files:
  - source: "configs/vscode-settings.json"
    destination: "%APPDATA%/Code/User/settings.json"
    overwrite: true

scripts:
  - name: "Setup Development Folders"
    file: "scripts/setup-dev-folders.ps1"
    runAs: "user"

tests:
  - name: "VS Code Installed"
    type: "command"
    target: "code --version"
  - name: "Git Available"
    type: "command"
    target: "git --version"
    expected: "git version"
```

### Validation

Validate your workloads before deployment:

```powershell
# Simple validation (PowerShell only)
.\scripts\validate-workloads-simple.ps1

# Full JSON Schema validation (requires Node.js)
.\scripts\validate-workloads.ps1
```

### Running Individual Scripts

Execute workload-specific setup scripts:

```powershell
# Setup data science environment
.\workloads\data-science\scripts\setup-datascience.ps1

# Configure NPM for web development
.\workloads\web-development\scripts\configure-npm.ps1

# Install VS Code extensions
.\workloads\web-development\scripts\install-vscode-extensions.ps1
```

## 📖 Usage Examples

### Installing a v2 Workload

```powershell
# Using PowerShell launcher
.\winforge.ps1 -Action Install -Workload data-science -Silent

# Using console application directly
cd src && dotnet run -- interactive
```

### Checking System Compliance

```powershell
# Generate detailed compliance report
.\winforge.ps1 -Action Validate -Profile Corporate

# Generate and save compliance report
.\winforge.ps1 -Action Report -Profile Developer
```

### Detecting Installed Applications

```powershell
# Scan for installed applications
.\winforge.ps1 -Action Detect

# Using console application directly
cd src && dotnet run -- help
```

### Command Line Options

- `-Action <string>` - Action to perform (Install, Validate, Report, Detect, Interactive)
- `-Workload <string>` - Workload to install (data-science, web-development, etc.)
- `-Profile <string>` - Compliance profile (Corporate, Developer, Standard)
- `-Silent` - Run without user prompts
- `-Help` - Show help information

## 🔧 Configuration

### Creating Custom Workloads

1. **Create a new YAML file** in the `workloads/` directory
2. **Follow the v2 schema format** with required fields (name, description, version, author)
3. **Define packages** using supported package managers
4. **Add configuration files** if needed
5. **Create PowerShell scripts** in `workloads/{workload-name}/scripts/` for custom setup
6. **Add validation tests** to ensure proper installation
7. **Validate your workload** using the validation scripts

### Configuration Templates

Template files in the `templates/` directory provide pre-configured settings for common tools:

- **VS Code Settings** - Editor preferences, extensions, formatting rules
- **PowerShell Profile** - Custom functions, aliases, and prompt
- **Git Configuration** - Aliases, merge tools, and best practices

### Compliance Profiles

The system supports multiple compliance profiles:

- **Standard** - Basic security and configuration requirements
- **Developer** - Enhanced settings for development workstations
- **Corporate** - Enterprise-level security and compliance requirements

## 📊 Reporting

### Compliance Reports

- **HTML Reports** - Rich, formatted compliance reports with charts
- **JSON Reports** - Machine-readable compliance data
- **CSV Reports** - Tabular data for analysis
- **XML Reports** - Structured compliance information

### Validation Reports

- **Schema Validation** - YAML structure and JSON Schema compliance
- **Package Verification** - Package manager and dependency checks
- **System Requirements** - Hardware and software requirement validation
- **Test Results** - Workload validation test outcomes

## 🔐 Security Features

### Compliance Validation

- **Windows Defender** - Antivirus and real-time protection status
- **Windows Firewall** - Firewall configuration and rules
- **User Account Control** - UAC settings and configuration
- **BitLocker Encryption** - Drive encryption status (Corporate profile)
- **Windows Updates** - Update installation status and pending updates

### Schema Security

- **Input Validation** - JSON Schema prevents malformed configurations
- **Script Isolation** - External scripts with controlled execution
- **Package Verification** - Package manager source validation
- **Permission Control** - User vs. administrator execution contexts

## 🛠️ Development and Deployment

### Building the Project

```powershell
# Build the console application
cd src
dotnet build

# Create a release build
dotnet build -c Release

# Publish for deployment
dotnet publish -c Release -o ../publish
```

### Migration from v1 to v2

The v2 schema is designed for new workloads. Existing v1 workloads continue to work:

- **v1 workloads** remain in the complex format for backward compatibility
- **v2 workloads** use the new simple schema format
- **Migration tools** are available to convert complex workloads to simple format
- **Validation** supports both formats with appropriate schema detection

### Extending the Application

1. Add new classes to the appropriate `src/` directories
2. Implement new services in `src/Services/`
3. Add new UI components in `src/UI/`
4. Update the dependency injection in [`src/Core/WinforgeApplication.cs`](src/Core/WinforgeApplication.cs)

### Adding Configuration Templates

1. Create template files in the `templates/` directory
2. Use placeholders like `{{USERNAME}}` for dynamic values
3. Reference templates in workload configurations
4. Implement template processing in installation scripts

## 📚 Documentation

### Schema Documentation

- **[Schema Design](schemas/workload-schema-design.md)** - Design principles and architecture
- **[Schema Specification](schemas/workload-schema-v2-specification.md)** - Complete JSON Schema specification
- **[Migration Guide](SCHEMA-COMPARISON.md)** - v1 to v2 migration and comparison
- **[Usage Examples](docs/WORKLOAD-EXAMPLES.md)** - Step-by-step workload creation guide

### Additional Resources

- **[Implementation Summary](README-workload-system-v2.md)** - Complete v2 implementation details
- **PowerShell Module Reference** - Module documentation
- **Compliance Framework** - Security and policy documentation
- **C# Script Development** - Advanced customization guide

## 🐛 Troubleshooting

### Common Issues

#### .NET Runtime Not Found

```powershell
# Download and install .NET 8.0 Runtime
# From: https://dotnet.microsoft.com/download

# Verify installation
dotnet --version
```

#### .NET SDK Not Found

```powershell
# Download and install .NET 6.0 SDK or higher
# From: https://dotnet.microsoft.com/download

# Verify installation
dotnet --version
```

#### PowerShell Execution Policy

```powershell
# Set execution policy for current user
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Or bypass for specific script
powershell.exe -ExecutionPolicy Bypass -File winforge.ps1
```

#### Workload Validation Errors

```powershell
# Validate specific workload
.\scripts\validate-workloads-simple.ps1

# Check JSON Schema compliance (requires Node.js)
.\scripts\validate-workloads.ps1

# Fix common issues:
# - Ensure required fields are present (name, description, version, author)
# - Check package manager names (winget, chocolatey, npm, pip, powershell)
# - Verify script file extensions (.ps1 required)
# - Validate test types (command, file, registry)
```

#### Package Installation Failures

- Check internet connectivity
- Verify package manager is installed and updated
- Run as administrator for system-wide installations
- Check antivirus software for blocking
- Validate package names and IDs

### Debug Mode

Enable verbose output for troubleshooting:

```powershell
# Run with help to see available commands
.\winforge.ps1

# Check application directly
cd src && dotnet run -- help

# Run interactive mode for troubleshooting
cd src && dotnet run -- interactive
```

## 📚 External Links

- [.NET 8.0 Documentation](https://docs.microsoft.com/en-us/dotnet/core/whats-new/dotnet-8)
- [Windows Package Manager Documentation](https://docs.microsoft.com/en-us/windows/package-manager/)
- [Chocolatey Documentation](https://chocolatey.org/docs)
- [PowerShell Documentation](https://docs.microsoft.com/en-us/powershell/)
- [JSON Schema Specification](https://json-schema.org/)
- [YAML Specification](https://yaml.org/spec/)

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines for details on:

- Code style and standards
- Testing requirements
- Documentation standards
- Pull request process

### Development Setup

1. Fork the repository
2. Clone your fork locally
3. Install .NET 8.0+ SDK
4. Create or modify workloads using the v2 schema format
5. Test by running `cd src && dotnet run`
6. Update documentation
7. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🏷️ Version History

### v2.0.0 (Current - .NET Console Application Edition)

- **Modern .NET Architecture** - Compiled console application for better performance
- **Modular Design** - Clear separation of concerns with organized source structure
- **Simple Workload Schema v2** - 90% reduction in complexity from v1
- **PowerShell Integration** - Seamless integration with existing PowerShell workflows
- **JSON Schema Validation** - Built-in validation with error checking
- **Multiple Package Managers** - Enhanced support for npm, pip, powershell
- **Proven Implementation** - Real-world data science and web development workloads

### v1.0.0 (Previous - Complex Schema Edition)

- Initial release with complex YAML schema
- Core PowerShell modules
- Web development and data science workloads
- Basic compliance reporting
- Configuration templates

### Planned Features

- **v2.1.0** - Enhanced GUI interface with web-based dashboard
- **v2.2.0** - Cloud integration (Azure DevOps, AWS)
- **v2.3.0** - Advanced compliance policies and automation
- **v3.0.0** - Cross-platform support (Linux, macOS)

## 🆘 Support

For support and questions:

- Create an issue in the GitHub repository
- Check the troubleshooting section above
- Review the schema documentation
- Validate your workloads using the validation scripts
- Contact the development team

---

**Built with ❤️ for developers by developers**

_Winforge v2 - Simplified. Validated. Enterprise-Ready._

## 📊 Benefits of Schema v2

- ✅ **90% simpler** workload definitions
- ✅ **Better maintainability** with external scripts
- ✅ **Enhanced validation** with JSON Schema
- ✅ **IDE support** with IntelliSense and debugging
- ✅ **Proven capability** handling complex enterprise workloads
- ✅ **Multiple package managers** with unified syntax
- ✅ **Future-proof design** with extensible architecture
