# Winforge Console Application

A robust .NET 8 console application for Windows workstation setup and management.

## Project Structure

```
src/
├── Program.cs                 # Main entry point
├── Winforge.csproj           # Project configuration
├── Core/
│   └── WinforgeApplication.cs # Application coordinator
├── Models/
│   ├── WorkloadMetadata.cs   # Workload metadata model
│   └── WorkloadModels.cs     # All workload-related models
├── Services/
│   └── WorkloadManager.cs    # Workload management service
└── UI/
    ├── IconProvider.cs       # Icon and symbol management
    └── WinforgeUI.cs        # Main UI coordinator
```

## Building

### Prerequisites
- .NET 8.0 SDK
- Windows 10/11

### Build Commands

```powershell
# Restore dependencies
dotnet restore

# Build for development
dotnet build

# Build for release
dotnet build --configuration Release

# Publish single-file executable
dotnet publish --configuration Release --runtime win-x64 --self-contained true -p:PublishSingleFile=true
```

### Supported Runtimes
- `win-x64` - Windows 64-bit (recommended)
- `win-x86` - Windows 32-bit
- `win-arm64` - Windows ARM64

## Features

- **Self-contained deployment** - No .NET runtime required on target machines
- **Single-file executable** - Easy distribution and deployment
- **Windows-optimized** - Leverages Windows-specific features and PowerShell
- **Interactive UI** - Rich console interface using Spectre.Console
- **YAML workload configuration** - Easy-to-write workload definitions

## Dependencies

- **Spectre.Console 0.48.0** - Rich console UI components
- **YamlDotNet 15.1.0** - YAML parsing and serialization
- **System.Management.Automation 7.4.0** - PowerShell integration
- **Newtonsoft.Json 13.0.3** - JSON processing

## Development

### Running in Development
```powershell
dotnet run
```

### Running Tests
```powershell
dotnet test
```

### Code Style
- C# 12 with nullable reference types enabled
- Async/await patterns for I/O operations
- Dependency injection where appropriate
- Comprehensive XML documentation

## Deployment

The application is designed for deployment through GitHub Actions, which:
1. Builds for all supported Windows platforms
2. Creates optimized single-file executables
3. Packages with workload examples and documentation
4. Creates GitHub releases with proper versioning

## Configuration

The application looks for workloads in:
1. `workloads/` directory relative to the executable
2. `workloads/` directory in the current working directory

Each workload should have:
- `workload.yaml` - Main configuration file
- `scripts/` - PowerShell scripts (optional)
- `README.md` - Documentation (optional)

## Architecture

The application follows a layered architecture:
- **Presentation Layer** (UI) - Handles user interaction
- **Business Layer** (Services) - Implements core logic
- **Data Layer** (Models) - Defines data structures
- **Application Layer** (Core) - Coordinates components

This separation ensures maintainability, testability, and extensibility.