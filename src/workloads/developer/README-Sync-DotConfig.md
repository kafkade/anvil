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

### Basic Forward Synchronization (WinForge → User)
```powershell
# Interactive synchronization with default settings
.\Sync-DotConfig.ps1

# Show all differences without making changes
.\Sync-DotConfig.ps1 -ShowDiffsOnly

# Perform a dry run to see what would be done
.\Sync-DotConfig.ps1 -DryRun -Verbose
```

### Advanced Forward Sync Usage
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

### Reverse Sync Workflows (User → WinForge)
```powershell
# Complete bidirectional workflow
# 1. First, get latest WinForge configurations
.\Sync-DotConfig.ps1 -BackupFirst

# 2. Customize your configurations in ~/.config as needed
# 3. Run sync again and use Option 6 for files you want to contribute back
.\Sync-DotConfig.ps1

# 4. Commit the changes to Git (script will remind you)
git add src/.config/
git commit -m "Improve PowerShell aliases and Git configuration"
git push origin main
```

### Development and Contribution Workflow
```powershell
# Review what you would contribute back before making changes
.\Sync-DotConfig.ps1 -ShowDiffsOnly

# Interactive session for selective reverse sync
.\Sync-DotConfig.ps1
# Choose Option 6 (Copy from User to WinForge) for improved files
# Choose Option 3 (Skip) for personal customizations

# Dry run to see what reverse sync would do
.\Sync-DotConfig.ps1 -DryRun -Verbose
# Note: Reverse sync actions are selected interactively, dry run shows analysis only
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
2. **Copy from WinForge to User (Forward Sync)**: Overwrite user file with WinForge version
3. **Skip This File**: Continue without making changes
4. **Backup User File + Copy from WinForge**: Create backup before forward sync
5. **Open Files in Editor**: Launch VS Code diff view or Notepad for manual editing
6. **Copy from User to WinForge (Reverse Sync)**: Contribute user improvements to WinForge repository
7. **Quit Synchronization**: Exit the script safely

### Menu Option Details

**Option 1 - Show Diff/Content**: Displays a comprehensive comparison showing file paths, status, and content preview (first 10 lines of each file) to help you understand the differences.

**Option 2 - Forward Sync (Direct)**: Copies the WinForge version directly to your user directory. Use when you want to adopt the WinForge configuration as-is.

**Option 3 - Skip**: Leaves both files unchanged. Use for personal customizations you want to keep separate from WinForge.

**Option 4 - Forward Sync (Safe)**: Creates a timestamped backup in `%TEMP%` before copying WinForge version to user directory. Recommended for important configurations.

**Option 5 - Open in Editor**: Launches VS Code with diff view (preferred) or opens both files in separate Notepad windows for manual comparison and editing.

**Option 6 - Reverse Sync**: **⚠️ Repository Modification Warning** - Copies your user configuration to the WinForge repository. Use when you've made improvements that should be shared. The script will:
- Show clear warnings about modifying repository files
- Require confirmation before proceeding
- Remind you to commit changes to Git after the operation

**Option 7 - Quit**: Safely exits the synchronization process and displays a summary of any operations performed up to that point.

## Reverse Sync - Contributing Back to WinForge

### Overview
Reverse sync enables you to contribute your improved configurations back to the WinForge repository. This powerful feature allows the community to benefit from your customizations and improvements.

### When to Use Reverse Sync
- **Configuration Improvements**: You've enhanced PowerShell aliases, Git configurations, or other settings
- **Bug Fixes**: You've corrected issues in existing configuration files
- **New Features**: You've added useful configurations that others would benefit from
- **Cross-platform Compatibility**: You've improved configurations to work better across different platforms

### Reverse Sync Process
1. **Start with Forward Sync**: Always begin by syncing WinForge configurations to your user directory
2. **Make Improvements**: Customize and improve configurations in your `~/.config` directory
3. **Test Thoroughly**: Ensure your changes work correctly in your environment
4. **Run Interactive Sync**: Use `.\Sync-DotConfig.ps1` and select Option 6 for files you want to contribute
5. **Commit to Git**: The script will remind you to commit your changes to the repository

### Git Integration for Reverse Sync
The script provides Git-aware features for reverse sync operations:

- **Repository Awareness**: Detects when you're modifying WinForge repository files
- **Commit Reminders**: Automatically displays Git commands to commit your changes
- **Clear Warnings**: Shows prominent warnings when modifying repository files
- **Status Integration**: Suggests using `git status` to review changes before committing

### Example Git Workflow
```powershell
# After reverse sync operations, the script will show:
git status                    # Review what files were changed
git add src/.config/          # Stage configuration changes
git commit -m "Improve PowerShell aliases and add new Git shortcuts"
git push origin main          # Share with the community
```

### Safety Considerations for Reverse Sync
- **No Backup Required**: Git provides version control for the WinForge repository
- **Confirmation Required**: Script asks for explicit confirmation before modifying repository files
- **Selective Operation**: Choose exactly which files to reverse sync
- **Repository Status**: Consider running `git status` before reverse sync to understand current state

## File Comparison Logic

### Multi-tier Analysis
1. **Existence Check**: Determines if source and/or target files exist
2. **Hash Comparison**: SHA256 hash comparison for quick difference detection
3. **Content Analysis**: Detailed content examination for supported file types

### Status Categories
- **Identical**: Files have matching SHA256 hashes
- **Different**: Files exist but have different content
- **TargetMissing**: Source exists but target doesn't (Forward: can copy; Reverse: no user file to contribute)
- **SourceMissing**: Target exists but source doesn't (Forward: no WinForge file; Reverse: can contribute new file)
- **BothMissing**: Neither file exists
- **Error**: Comparison failed due to access or other issues

## Safety Features

### Forward Sync Safety (WinForge → User)
**Backup System:**
- Automatic timestamped backup directory creation
- Individual file backups before any modifications
- Backup location: `%TEMP%\WinForge_ConfigBackup_YYYYMMDD_HHMMSS`
- Verification of backup integrity
- Preserves directory structure for easy restoration

**Protection Mechanisms:**
- Interactive confirmation for overwrites
- Option to backup before every copy operation
- Dry run mode for testing changes
- Hash verification after all copy operations

### Reverse Sync Safety (User → WinForge)
**Git Integration:**
- No backup system needed (Git provides version control)
- Clear warnings before modifying repository files
- Explicit confirmation required for each reverse sync operation
- Automatic commit reminders with suggested commands

**Repository Protection:**
- Confirmation dialogs prevent accidental changes
- Git status awareness
- Clear indication of which files will be modified in the repository
- Integration with existing Git workflows

### Universal Safety Features
**Error Handling:**
- Comprehensive try-catch blocks around all operations
- Rollback capabilities for failed operations
- File verification after copy operations (both directions)
- Detailed error logging and reporting

**Atomic Operations:**
- File operations are completed entirely or not at all
- Hash verification ensures data integrity
- Directory creation with proper error handling
- Safe permission setting for executable files (Git hooks)

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

### Forward Sync Best Practices (Consuming WinForge Configurations)

**Before Running:**
1. **Review your configurations**: Ensure your personal configs are in a good state
2. **Test with dry run**: Always run with `-DryRun` first to understand changes
3. **Create backups**: Use `-BackupFirst` for important configuration changes
4. **Check file permissions**: Ensure you have proper access to both directories

**During Forward Synchronization:**
1. **Review differences carefully**: Use "Show Diff/Content" option liberally
2. **Use backups**: Choose "Backup User File + Copy from WinForge" for important files
3. **Preserve personal customizations**: Skip files with personal settings you want to keep
4. **Test incrementally**: Sync critical files first (PowerShell, Git) before theme files

**After Forward Synchronization:**
1. **Test your environment**: Ensure PowerShell, Git, and other tools work correctly
2. **Review backup location**: Keep backups until you're confident changes are working
3. **Document any issues**: Note configurations that need further customization

### Reverse Sync Best Practices (Contributing to WinForge)

**Before Reverse Sync:**
1. **Start with forward sync**: Always begin by getting latest WinForge configurations
2. **Test your improvements**: Ensure your changes work correctly in your environment
3. **Review Git status**: Check current repository state with `git status`
4. **Consider the community**: Ensure your changes benefit others, not just your setup

**During Reverse Sync:**
1. **Be selective**: Only reverse sync improvements, not personal customizations
2. **Use descriptive commit messages**: Plan your Git commit message while selecting files
3. **Review each file**: Use "Show Diff/Content" to understand what you're contributing
4. **Confirm carefully**: Pay attention to repository modification warnings

**After Reverse Sync:**
1. **Commit promptly**: Follow the script's Git commit reminders immediately
2. **Write clear commit messages**: Describe the improvements you're contributing
3. **Test the repository**: Ensure the WinForge repository still works after your changes
4. **Consider documentation**: Update related documentation if needed

### Bidirectional Workflow Best Practices

**Establishing Your Environment:**
```powershell
# 1. Initial setup - get WinForge configurations
.\Sync-DotConfig.ps1 -BackupFirst

# 2. Customize as needed in your ~/.config directory
# 3. Test your customizations thoroughly
```

**Contributing Improvements:**
```powershell
# 4. Review what you want to contribute
.\Sync-DotConfig.ps1 -ShowDiffsOnly

# 5. Selectively contribute improvements
.\Sync-DotConfig.ps1
# Use Option 6 for improvements, Option 3 for personal customizations

# 6. Commit your contributions
git add src/.config/
git commit -m "Improve PowerShell aliases and Git hooks"
git push origin main
```

**Regular Maintenance:**
1. **Periodic forward sync**: Stay updated with WinForge improvements
2. **Selective reverse sync**: Contribute your own improvements back
3. **Keep personal and community separate**: Maintain clear distinction between personal customizations and community contributions
4. **Document your workflow**: Keep notes about which configurations you customize vs. contribute

## Troubleshooting

### Common Forward Sync Issues

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

**Backup Directory Issues**
```powershell
# Check if temp directory is accessible
Test-Path $env:TEMP
Get-ChildItem $env:TEMP -Filter "WinForge_ConfigBackup_*" | Sort-Object CreationTime -Descending
```

### Common Reverse Sync Issues

**Git Repository Not Ready**
```powershell
# Ensure you're in a Git repository
git status

# If not in a repository or not the WinForge repository:
cd path\to\winforge
git status
```

**Uncommitted Changes Warning**
```powershell
# Check what's already modified in the repository
git status

# Commit or stash existing changes before reverse sync
git add .
git commit -m "Previous changes"
# OR
git stash
```

**Repository Permission Issues**
```powershell
# Ensure you have write access to the WinForge repository
# Check if files are read-only
Get-ChildItem src\.config -Recurse | Where-Object { $_.IsReadOnly }
```

**Reverse Sync Confirmation Issues**
- The script requires explicit confirmation before modifying repository files
- Make sure to type 'y' or 'Y' when prompted for reverse sync operations
- If you're unsure, use Option 1 to review differences first

### Git Integration Troubleshooting

**Git Commands Not Found**
```powershell
# Verify Git is installed and in PATH
git --version

# If Git is not found, ensure it's installed and added to PATH
```

**Repository State Issues**
```powershell
# Check repository status before reverse sync
git status
git log --oneline -5

# Ensure you're on the correct branch
git branch -a
```

**Post-Reverse Sync Git Issues**
```powershell
# Review what files were changed
git status
git diff --name-only

# If you need to undo reverse sync changes:
git checkout -- src/.config/
```

### Error Recovery

**Forward Sync Recovery**
- Check the backup directory shown in the summary report
- Restore files from backup: `Copy-Item "$BackupPath\*" "~/.config" -Recurse -Force`
- Review the summary report for specific error details

**Reverse Sync Recovery**
```powershell
# Use Git to recover from reverse sync issues
git status                    # See what was changed
git diff src/.config/         # Review the changes
git checkout -- src/.config/ # Undo changes if needed
git reset HEAD src/.config/   # Unstage changes if needed
```

### Advanced Troubleshooting

**Logging and Diagnostics**
```powershell
# Enable detailed logging for forward sync
.\Sync-DotConfig.ps1 -Verbose -DryRun > sync-log.txt 2>&1

# Review operation results
Get-Content sync-log.txt | Select-String "Error|Warning|Failed"
```

**Hash Verification Issues**
```powershell
# Manually verify file integrity
$sourceHash = (Get-FileHash "src/.config/pwsh/aliases.ps1").Hash
$targetHash = (Get-FileHash "~/.config/pwsh/aliases.ps1").Hash
$sourceHash -eq $targetHash
```

**File System Issues**
```powershell
# Check disk space
Get-WmiObject -Class Win32_LogicalDisk | Select-Object DeviceID, FreeSpace, Size

# Check file system errors
chkdsk C: /f
```

### Getting Help

**Script-Specific Issues**
1. Use `-Verbose -DryRun` to understand what the script would do
2. Check the summary report for detailed error information
3. Review backup locations for recovery options
4. Use Git commands to understand repository state for reverse sync issues

**Community Support**
- Review WinForge documentation for configuration file formats
- Check Git logs to see recent changes: `git log --oneline src/.config/`
- Consult PowerShell or Git documentation for tool-specific issues

## Integration with WinForge

### Workload Integration
The script is designed to integrate seamlessly with the WinForge workload system:
- Located in `src/workloads/developer/` directory
- Follows WinForge PowerShell coding standards
- Uses WinForge error handling patterns
- Compatible with existing module structure
- Supports both consumption and contribution workflows

### Bidirectional Development Workflow

**Forward Sync (Consuming WinForge Configurations):**
1. Run `Sync-DotConfig.ps1` to get latest WinForge configurations
2. Test configurations in your development environment
3. Customize as needed for your personal workflow
4. Keep personal customizations separate from community configurations

**Reverse Sync (Contributing to WinForge):**
1. Start with latest WinForge configurations (forward sync)
2. Make improvements to configurations in `~/.config`
3. Test improvements thoroughly in your environment
4. Run `Sync-DotConfig.ps1` and use Option 6 to contribute improvements
5. Commit changes to WinForge repository with descriptive messages
6. Share improvements with the community

### Community Collaboration
The bidirectional sync capability enables:
- **Knowledge Sharing**: Community members can share their configuration improvements
- **Iterative Improvement**: Configurations evolve through community contributions
- **Best Practice Distribution**: Proven configurations spread throughout the community
- **Cross-Platform Benefits**: Improvements from different platforms benefit everyone

## Version History

- **v2.0.0**: Bidirectional Sync Release
  - **Bidirectional synchronization**: Forward sync (WinForge → User) and reverse sync (User → WinForge)
  - **Git repository awareness**: Automatic commit reminders and Git integration
  - **Enhanced interactive menu**: New Option 6 for reverse sync with confirmation dialogs
  - **Community contribution workflow**: Streamlined process for sharing configuration improvements
  - **Improved safety mechanisms**: Different safety approaches for forward vs reverse sync
  - **Git-aware operations**: Repository modification warnings and commit guidance

- **v1.0.0**: Initial Forward Sync Implementation
  - Multi-tier comparison logic
  - Interactive user interface
  - Comprehensive backup system (forward sync only)
  - File-type specific handling
  - Safety mechanisms and error handling
  - Progress tracking and reporting

## Contributing

### Contributing to the Script
When contributing to this script:
1. Follow PowerShell best practices and style guidelines
2. Add comprehensive error handling for new features
3. Update documentation for any new parameters or functionality
4. Test thoroughly with various file types and scenarios, including both sync directions
5. Ensure backward compatibility with existing WinForge patterns
6. Consider both forward and reverse sync implications for new features

### Contributing Configurations Using Reverse Sync
The script now makes it easy to contribute improved configurations:

1. **Start with WinForge baseline**: Use forward sync to get the latest configurations
2. **Make thoughtful improvements**: Enhance configurations that benefit the community
3. **Test thoroughly**: Ensure your improvements work across different scenarios
4. **Use reverse sync**: Run the script and choose Option 6 for files you want to contribute
5. **Write clear commit messages**: Describe the improvements you're contributing
6. **Follow Git workflow**: Use the script's commit reminders to share your improvements

### Types of Configuration Contributions Welcome
- **PowerShell alias improvements**: More efficient or useful command shortcuts
- **Git configuration enhancements**: Better diff tools, merge strategies, or aliases
- **Cross-platform compatibility fixes**: Configurations that work better across Windows, Linux, and macOS
- **Performance optimizations**: Configurations that improve tool performance
- **Security improvements**: Enhanced security configurations
- **New useful configurations**: Additional configuration files that benefit developers

### Contribution Guidelines
- **Community benefit**: Ensure changes benefit others, not just personal preferences
- **Documentation**: Update relevant documentation for significant configuration changes
- **Testing**: Test changes in multiple environments when possible
- **Incremental improvements**: Make focused improvements rather than massive overhauls
- **Respect existing patterns**: Follow established WinForge configuration conventions

## License

This script is part of the WinForge project and follows the same licensing terms.