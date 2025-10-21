# Sync-DotConfig.ps1 - Configuration Synchronization Tool

## Overview

The `Sync-DotConfig.ps1` script provides comprehensive **bidirectional synchronization** of configuration files between your personal `~/.config` directory and the WinForge repository's `src/.config` directory. This tool supports both forward sync (WinForge → User) for consuming configurations and reverse sync (User → WinForge) for contributing improvements back to the repository.

The script implements advanced comparison logic, interactive decision-making, Git repository awareness, and robust safety mechanisms to ensure secure and reliable configuration management in both directions.

### Synchronization Directions

- **Forward Sync (WinForge → User)**: The default operation that copies configurations from the WinForge repository to your personal `~/.config` directory. Includes comprehensive backup system for safety.
- **Reverse Sync (User → WinForge)**: Enables contributing your improved configurations back to the WinForge repository. Includes Git awareness and commit reminders.

## Features

### Core Functionality
- **Bidirectional Synchronization**: Forward sync (WinForge → User) and reverse sync (User → WinForge)
- **Multi-tier Comparison Logic**: Existence, SHA256 hash, and content-aware analysis
- **Interactive User Interface**: Per-file decision menus with multiple action options including reverse sync
- **Comprehensive Backup System**: Timestamped backups with automatic rollback capabilities (forward sync)
- **Git Repository Awareness**: Automatic commit reminders and Git integration for reverse sync operations
- **File-type Specific Handling**: Specialized processing for PowerShell, Git config, JSON, and bash scripts
- **Robust Safety Mechanisms**: Atomic operations, verification, and error recovery for both sync directions
- **Progress Tracking**: Real-time progress updates and detailed summary reports

### Forward Sync Features (WinForge → User)
- Comprehensive backup system in `%TEMP%` with timestamped directories
- Safe overwrite protection with backup-before-copy option
- Standard environment setup and configuration consumption workflow

### Reverse Sync Features (User → WinForge)
- Git-aware operations with commit reminders
- No backup needed (Git provides version control for repository)
- Contribution workflow for sharing improved configurations
- Clear warnings and confirmations for repository modifications

### Supported Configuration Files
- **PowerShell Configuration**: `aliases.ps1`, `user_profile.ps1`, `psreadline.ps1`
- **Git Configuration**: `gitconfig`, `aliases.gitconfig`, `delta.gitconfig`, etc.
- **Git Templates**: `.gitmessage`, hooks (`pre-commit`, `pre-push`)
- **Theme Configuration**: `mytheme.omp.json` (Oh My Posh themes)
- **JSON Configuration Files**: Various `.json` config files
- **YAML Configuration Files**: `.yml` and `.yaml` files

## Command-Line Parameters

### Basic Parameters
- **`-BackupFirst`**: Creates a backup of all target files before any operations
- **`-ShowDiffsOnly`**: Only displays differences without prompting for actions
- **`-DryRun`**: Shows what would be done without making actual changes
- **`-AutoYes`**: Automatically answers 'yes' to all prompts (use with caution)
- **`-Verbose`**: Provides detailed logging output for troubleshooting

### File Filtering
- **`-IncludeFiles`**: Comma-separated list of specific files to include
- **`-ExcludeFiles`**: Comma-separated list of specific files to exclude
- **`-ConfigPath`**: Override the default user config path (`~/.config`)

## Usage Examples

### Basic Synchronization
```powershell
# Interactive synchronization with default settings
.\Sync-DotConfig.ps1

# Show all differences without making changes
.\Sync-DotConfig.ps1 -ShowDiffsOnly

# Perform a dry run to see what would be done
.\Sync-DotConfig.ps1 -DryRun -Verbose
```

### Advanced Usage
```powershell
# Create backups first, then synchronize automatically
.\Sync-DotConfig.ps1 -BackupFirst -AutoYes

# Synchronize only specific PowerShell files
.\Sync-DotConfig.ps1 -IncludeFiles "aliases.ps1,user_profile.ps1"

# Exclude theme configuration from synchronization
.\Sync-DotConfig.ps1 -ExcludeFiles "mytheme.omp.json"

# Use custom configuration path
.\Sync-DotConfig.ps1 -ConfigPath "C:\CustomConfig"
```

### Troubleshooting and Analysis
```powershell
# Detailed analysis with verbose output and backups
.\Sync-DotConfig.ps1 -BackupFirst -ShowDiffsOnly -Verbose

# Safe exploration mode
.\Sync-DotConfig.ps1 -DryRun -ShowDiffsOnly -Verbose
```

## Interactive Menu Options

When the script encounters files that require attention, it presents the following options:

1. **Show Diff/Content**: Display file differences and content preview
2. **Copy Source to Target (Replace)**: Overwrite target with source file
3. **Skip This File**: Continue without making changes
4. **Backup Target + Copy Source**: Create backup before overwriting
5. **Open Files in Editor**: Launch VS Code or Notepad for manual editing
6. **Quit Synchronization**: Exit the script safely

## File Comparison Logic

### Multi-tier Analysis
1. **Existence Check**: Determines if source and/or target files exist
2. **Hash Comparison**: SHA256 hash comparison for quick difference detection  
3. **Content Analysis**: Detailed content examination for supported file types

### Status Categories
- **Identical**: Files have matching SHA256 hashes
- **Different**: Files exist but have different content
- **TargetMissing**: Source exists but target doesn't
- **SourceMissing**: Target exists but source doesn't
- **BothMissing**: Neither file exists
- **Error**: Comparison failed due to access or other issues

## Safety Features

### Backup System
- Automatic timestamped backup directory creation
- Individual file backups before any modifications
- Backup location: `%TEMP%\WinForge_ConfigBackup_YYYYMMDD_HHMMSS`
- Verification of backup integrity

### Error Handling
- Comprehensive try-catch blocks around all operations
- Rollback capabilities for failed operations
- File verification after copy operations
- Detailed error logging and reporting

### Atomic Operations
- File operations are completed entirely or not at all
- Hash verification ensures data integrity
- Directory creation with proper error handling
- Safe permission setting for executable files

## File Type Handling

### PowerShell Files (`.ps1`)
- Syntax validation support
- Profile-specific handling
- Module import verification

### Git Configuration Files
- Config section parsing
- Alias validation
- Identity management support
- Hook executable permission setting

### JSON Configuration Files
- Structure validation
- Pretty-printing preservation
- Schema compliance checking

### Git Hooks and Scripts
- Executable permission management
- Cross-platform compatibility
- Shebang line preservation

## Output and Reporting

### Progress Tracking
- Real-time progress bars during analysis
- File-by-file processing status
- Percentage completion indicators

### Summary Reports
- Total files processed count
- Success/failure operation counts
- File status distribution
- Backup location information
- Detailed operation results

### Color-coded Output
- **Green**: Successful operations and identical files
- **Red**: Errors and conflicts
- **Yellow**: Warnings and dry-run indicators
- **Cyan**: Headers and informational messages
- **Gray**: Skipped or missing files

## Best Practices

### Before Running
1. **Review your configurations**: Ensure your personal configs are in a good state
2. **Test with dry run**: Always run with `-DryRun` first to understand changes
3. **Create backups**: Use `-BackupFirst` for important configuration changes
4. **Check file permissions**: Ensure you have proper access to both directories

### During Synchronization
1. **Review differences carefully**: Use "Show Diff/Content" option liberally
2. **Use backups**: Choose "Backup Target + Copy Source" for important files
3. **Test configurations**: Verify configs work after synchronization
4. **Document changes**: Keep notes of customizations and modifications

### After Synchronization  
1. **Test your environment**: Ensure PowerShell, Git, and other tools work correctly
2. **Review backup location**: Keep backups until you're confident changes are working
3. **Update documentation**: Document any custom configurations or processes
4. **Regular maintenance**: Run periodic synchronizations to keep configs updated

## Troubleshooting

### Common Issues

**Permission Errors**
```powershell
# Run with elevated permissions if needed
gsudo .\Sync-DotConfig.ps1
```

**Path Issues**
```powershell
# Verify paths exist and are accessible
Test-Path "~/.config"
Test-Path "src/.config"
```

**File Lock Errors**
```powershell
# Close applications that might be using config files
# Restart PowerShell session if needed
```

### Error Recovery
- Check the backup directory for original files
- Review the summary report for specific error details
- Use `-Verbose` flag for detailed troubleshooting information
- Manually restore from backups if needed

### Logging and Diagnostics
```powershell
# Enable detailed logging
.\Sync-DotConfig.ps1 -Verbose -DryRun > sync-log.txt 2>&1

# Review operation results
Get-Content sync-log.txt | Select-String "Error|Warning|Failed"
```

## Integration with WinForge

### Workload Integration
The script is designed to integrate seamlessly with the WinForge workload system:
- Located in `src/workloads/developer/` directory
- Follows WinForge PowerShell coding standards
- Uses WinForge error handling patterns
- Compatible with existing module structure

### Development Workflow
1. Modify personal configurations in `~/.config`
2. Test configurations thoroughly
3. Run `Sync-DotConfig.ps1` to synchronize with repository
4. Commit changes to WinForge repository
5. Share configurations with team members

## Version History

- **v1.0.0**: Initial implementation with full feature set
  - Multi-tier comparison logic
  - Interactive user interface
  - Comprehensive backup system
  - File-type specific handling
  - Safety mechanisms and error handling
  - Progress tracking and reporting

## Contributing

When contributing to this script:
1. Follow PowerShell best practices and style guidelines
2. Add comprehensive error handling for new features
3. Update documentation for any new parameters or functionality
4. Test thoroughly with various file types and scenarios
5. Ensure backward compatibility with existing WinForge patterns

## License

This script is part of the WinForge project and follows the same licensing terms.