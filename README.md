# winforge

A personal Windows development environment setup tool that makes it easy to configure your dev machine exactly 
how you want it.

## Overview

Winforge helps solve common Windows development environment headaches:

- Sets up your dev environment in minutes instead of hours
- Ensures all your tools are installed and configured consistently
- Makes it easy to share your setup with team members
- Lets you customize different setups for different project types

## Key Features

- **Quick Setup**: One command to install your complete dev environment
- **Flexible Configs**: Define different setups for different project types
- **Package Management**: Uses both winget and chocolatey for maximum coverage
- **Smart Validation**: Checks that everything is set up correctly
- **Security Aware**: Handles permissions and security settings properly
- **VS Code Ready**: Configures Visual Studio Code with your preferred settings

## Prerequisites

- Windows 11
- PowerShell 7.0 or later
- Administrator privileges
- Internet connectivity
- Windows Package Manager (winget)
- Chocolatey package manager (auto-installed if missing)

## Quick Start

1. Clone the repository:

```powershell
git clone https://microsoft.visualstudio.com/DefaultCollection/Xbox.Developer/_git/javierfe
```

2. Navigate to the source directory:

```powershell
cd src
```

3. Run the setup validation:

```powershell
.\check-setup.ps1
```

4. Install a workload:

```powershell
.\check-setup.ps1 -Workload developer
```

## Available Workloads

### Essentials

The basics you need for most development.

- Git setup
- Common dev tools
- Basic security settings

### Frontend

Everything for web development.

- Node.js and npm
- Web dev tools
- Frontend frameworks

### Backend

Server-side development tools.

- Database tools
- Server frameworks
- API dev utilities

### Developer

Full development setup.

- Complete IDE configuration
- Full stack dev tools
- Debugging tools

### Extras

Additional tools you might want.

- Extra dev tools
- Optional frameworks
- Useful utilities

## Configuration

### Workload Customization

Create your own workload in YAML under `workloads/`:

```yaml
workload:
  name: "custom-dev"
  packages:
    winget:
      - "Microsoft.VisualStudioCode"
    choco:
      - "git"
  scripts:
    pre: []
    post: []
```

### VS Code Configuration

- Set up your preferred VS Code settings
- Install extensions you use
- Configure debugging

## Validation

The tool checks:

- All components are installed correctly
- Everything is configured properly
- Your environment is healthy
- Packages are the right versions

## Security

- Handles Windows authentication properly
- Sets up secure account settings
- Uses least-privilege approach
- Follows security best practices

## Project Structure

```plaintext
src/
├── dev/                 # Development setup modules
├── identity/           # Identity management
├── modules/            # Core PowerShell modules
└── workloads/         # Workload definitions
    ├── backend/
    ├── developer/
    ├── essentials/
    ├── extras/
    └── frontend/
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes following the patterns in [design patterns](memory-bank/systemPatterns.md)
4. Add tests for your changes
5. Submit a pull request

## Troubleshooting

Common fixes:

- **Permission Issues**: Run PowerShell as Administrator
- **Package Install Fails**: Check your internet connection and proxy settings
- **Validation Errors**: Check the logs in `$env:TEMP\winforge-logs`

## Support

- File issues through Azure DevOps
- Check the docs in the memory-bank directory
- Review the logs if something goes wrong

## License

This project is licensed under the [MIT License](https://opensource.org/licenses/MIT).
