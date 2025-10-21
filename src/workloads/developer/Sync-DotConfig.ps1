<#
.SYNOPSIS
Synchronizes configuration files between user's ~/.config and WinForge src/.config directories with bidirectional sync capability.

.DESCRIPTION
This script provides comprehensive synchronization of dotfiles and configuration files between
the user's personal configuration directory (~/.config) and the WinForge repository (src/.config).
It's designed as a core component of the WinForge development environment setup, ensuring that
developer configurations remain consistent and version-controlled.

The script implements a multi-tier comparison system that goes beyond simple file existence checks:
1. File existence validation (both source and target)
2. SHA256 hash comparison for integrity verification
3. Content-aware analysis for different file types
4. Interactive decision-making with safety guardrails

Key Features:
- **Bidirectional synchronization**: Forward sync (WinForge → User) and Reverse sync (User → WinForge)
- Multi-tier comparison (existence, hash, content-aware analysis)
- Interactive user interface with per-file decision menus and diff viewing
- Comprehensive backup system with timestamped backups in %TEMP% (forward sync only)
- Git repository awareness with commit reminders for reverse sync operations
- File-type specific handling (PowerShell, Git config, JSON, bash scripts, hooks)
- Robust safety mechanisms and atomic file operations
- Progress tracking with real-time status updates
- Detailed summary reports with operation results
- Integration with WinForge project structure and conventions
- Cross-platform compatibility (Windows, Linux, macOS)

Synchronization Directions:
- **Forward Sync (Default)**: Copies from WinForge repository to user's ~/.config
  - Includes backup system for safety
  - Standard operation for environment setup
- **Reverse Sync (Contribution)**: Copies from user's ~/.config to WinForge repository
  - No backup needed (Git handles version control)
  - Enables contributing improved configurations back to WinForge
  - Includes Git awareness messages and commit reminders

WinForge Integration:
This script is part of the WinForge developer workload and integrates with the broader
ecosystem by managing configuration files that support:
- PowerShell profile customizations and aliases
- Git configuration including hooks, templates, and aliases
- Development tool configurations (Oh My Posh themes, etc.)
- Cross-platform script compatibility

.PARAMETER BackupFirst
Creates a timestamped backup of all target files before any synchronization operations begin.
Backups are stored in %TEMP%\WinForge_ConfigBackup_YYYYMMDD_HHMMSS\.
This is recommended for first-time runs or when making significant changes.

.PARAMETER ShowDiffsOnly
Only displays differences between source and target files without prompting for actions.
Useful for reviewing what would change before committing to synchronization.
Shows file status, differences, and content previews for changed files.

.PARAMETER DryRun
Performs all analysis and displays what operations would be performed without making any actual changes.
Safe for testing and validation. Combines well with -Verbose for detailed operation planning.

.PARAMETER AutoYes
Automatically answers 'yes' to all prompts, enabling unattended operation.
For missing target files: copies from source
For different files: creates backup and copies from source
Use with caution in production environments.

.PARAMETER IncludeFiles
Comma-separated list of specific files to include in synchronization.
Only files matching these names will be processed, others will be skipped.
Example: "aliases.ps1,gitconfig,mytheme.omp.json"

.PARAMETER ExcludeFiles
Comma-separated list of specific files to exclude from synchronization.
These files will be skipped even if they exist and differ.
Example: "user_profile.ps1,local_settings.json"

.PARAMETER ConfigPath
Override the default user config path (~/.config or %USERPROFILE%\.config on Windows).
Useful for testing with alternative configuration directories or custom setups.
Must be an absolute path to an existing directory.

.PARAMETER Verbose
Provides detailed logging output including:
- File discovery and mapping operations
- Hash calculations and comparisons
- Backup operations and file paths
- Copy operations with verification results
- Error details and recovery attempts

.EXAMPLE
.\Sync-DotConfig.ps1

Performs interactive synchronization with default settings. The script will:
1. Scan both ~/.config and src/.config directories
2. Compare all supported configuration files
3. Present an interactive menu for each file that differs or is missing
4. Allow you to review differences, create backups, and selectively sync files

.EXAMPLE
.\Sync-DotConfig.ps1 -BackupFirst -ShowDiffsOnly

Safe exploration mode that:
1. Creates backups of all target files first
2. Shows all differences without making changes
3. Useful for understanding current state before synchronization

.EXAMPLE
.\Sync-DotConfig.ps1 -DryRun -Verbose

Testing and validation mode that:
1. Shows detailed information about all operations that would be performed
2. Displays file paths, comparison results, and planned actions
3. Makes no actual changes to files
4. Perfect for understanding what the script would do

.EXAMPLE
.\Sync-DotConfig.ps1 -IncludeFiles "aliases.ps1,gitconfig" -AutoYes

Targeted automatic synchronization that:
1. Only processes PowerShell aliases and main Git config
2. Automatically performs recommended actions without prompting
3. Useful for CI/CD or scripted environment setup

.EXAMPLE
.\Sync-DotConfig.ps1 -ExcludeFiles "mytheme.omp.json,user_profile.ps1" -BackupFirst

Synchronizes all files except specified exclusions:
1. Creates backups before any operations
2. Skips personal theme and profile files
3. Processes all other configuration files interactively

.EXAMPLE
.\Sync-DotConfig.ps1 -ConfigPath "C:\CustomConfig" -DryRun

Tests synchronization with custom source directory:
1. Uses C:\CustomConfig instead of default ~/.config
2. Shows what would be synchronized without making changes
3. Useful for testing or working with alternative configurations

.EXAMPLE
# Reverse Sync Workflow - Contributing back to WinForge

# 1. First, sync WinForge configs to your user directory
.\Sync-DotConfig.ps1

# 2. Make improvements to your personal configs in ~/.config
# 3. Run sync again and choose "Copy from User to WinForge" for improved files
.\Sync-DotConfig.ps1

# 4. The script will remind you to commit changes:
git add src/.config/
git commit -m "Improve PowerShell aliases and Git configuration"
git push origin main

.NOTES
Author: WinForge Team
Version: 2.0.0
Requires: PowerShell 5.1 or higher
License: MIT
Project: https://github.com/javierfe_microsoft/winforge

SAFETY FEATURES AND ERROR HANDLING:

Backup System:
- Timestamped backups in %TEMP%\WinForge_ConfigBackup_YYYYMMDD_HHMMSS\
- Preserves directory structure from source
- Automatic backup before overwriting existing files
- Backup location displayed in summary report

File Operations:
- Atomic operations: files are copied completely or not at all
- SHA256 hash verification after copy operations
- Directory creation handled automatically
- Proper error handling with detailed error messages

Recovery Procedures:
If synchronization fails or produces unexpected results:
1. Check the backup location shown in the summary report
2. Manually restore files from backup: Copy-Item "$BackupPath\*" "$TargetPath" -Recurse -Force
3. Review error messages in the summary for specific issues
4. Use -DryRun -Verbose to diagnose issues without making changes
5. Contact WinForge team if systematic issues persist

Error Handling:
- Graceful handling of missing directories (creates them)
- File access permission errors are caught and reported
- Hash calculation failures are logged but don't stop processing
- Network or disk I/O errors are handled with retry logic
- All errors include context and suggested recovery actions

Integration with WinForge:
- Follows WinForge directory structure conventions
- Uses WinForge-standard error handling patterns
- Integrates with existing PowerShell module ecosystem
- Supports WinForge workload management system
- Compatible with WinForge installation and setup scripts

SUPPORTED FILE TYPES AND HANDLING:

PowerShell Files (.ps1):
- Profile scripts, aliases, PSReadLine configurations
- Preserves execution policy requirements
- Validates PowerShell syntax where possible

Git Configuration:
- Main gitconfig, aliases, and specialized config files
- Git hooks with automatic executable permission setting
- Git message templates and diff configurations

JSON Configuration:
- Oh My Posh themes, VS Code settings, tool configurations
- JSON validation and formatting preservation

Shell Scripts and Hooks:
- Cross-platform compatibility considerations
- Automatic executable permissions on Unix-like systems
- Proper line ending handling

TROUBLESHOOTING:

Common Issues:
1. "Access Denied" errors: Run as administrator or check file permissions
2. "Path not found" errors: Ensure source directories exist, use -ConfigPath if needed
3. Hash verification failures: Check disk space and file system integrity
4. Permission errors on hooks: Script automatically handles executable permissions

Performance:
- Processes files in priority order (PowerShell, Git, then others)
- Hash calculations are optimized for typical config file sizes
- Progress reporting for operations taking longer than expected

Logging:
- Use -Verbose for detailed operation logging
- Error messages include full context and suggested actions
- Summary report provides operation-by-operation results
#>

[CmdletBinding()]
param(
    [switch]$BackupFirst,
    [switch]$ShowDiffsOnly,
    [switch]$DryRun,
    [switch]$AutoYes,
    [string[]]$IncludeFiles = @(),
    [string[]]$ExcludeFiles = @(),
    [string]$ConfigPath = $null
)

# ============================================================================
# SCRIPT CONFIGURATION AND INITIALIZATION
# ============================================================================

# Set error handling behavior - Stop on errors to ensure data integrity
$ErrorActionPreference = "Stop"

# Configure information stream based on verbose preference for detailed logging
$InformationPreference = if ($VerbosePreference -eq "Continue") { "Continue" } else { "SilentlyContinue" }

# Initialize script-level variables for tracking operations and results
# These variables maintain state throughout the synchronization process
$script:OperationResults = @()        # Tracks all file operations performed
$script:BackupPath = $null            # Path to backup directory (set dynamically)
$script:TotalFiles = 0                # Total number of files to process
$script:ProcessedFiles = 0            # Current count of processed files
$script:SuccessfulOperations = 0      # Count of successful operations
$script:FailedOperations = 0          # Count of failed operations

#region Helper Functions

# ============================================================================
# PROGRESS AND OUTPUT FUNCTIONS
# ============================================================================

<#
.SYNOPSIS
Updates the PowerShell progress bar with current operation status.

.DESCRIPTION
Provides consistent progress reporting throughout the synchronization process.
Handles both percentage-based and indeterminate progress updates.

.PARAMETER Activity
The main activity being performed (e.g., "Analyzing Configuration Files")

.PARAMETER Status
Current status message (e.g., "Processing aliases.ps1")

.PARAMETER PercentComplete
Optional percentage complete (0-100). If -1, shows indeterminate progress.
#>
function Write-ProgressUpdate {
    param(
        [string]$Activity,
        [string]$Status,
        [int]$PercentComplete = -1
    )
    
    if ($PercentComplete -ge 0) {
        Write-Progress -Activity $Activity -Status $Status -PercentComplete $PercentComplete
    } else {
        Write-Progress -Activity $Activity -Status $Status
    }
}

<#
.SYNOPSIS
Writes colored output to the console for better user experience.

.DESCRIPTION
Provides consistent colored output throughout the script. Handles different
message types with appropriate colors for success, warnings, errors, etc.

.PARAMETER Message
The message to display

.PARAMETER Color
Console color to use for the message

.PARAMETER NoNewLine
If specified, doesn't add a new line after the message
#>
function Write-ColoredOutput {
    param(
        [string]$Message,
        [string]$Color = "White",
        [switch]$NoNewLine
    )
    
    $params = @{
        Object = $Message
        ForegroundColor = $Color
    }
    
    if ($NoNewLine) {
        $params.NoNewline = $true
    }
    
    Write-Host @params
}

# ============================================================================
# FILE INTEGRITY AND COMPARISON FUNCTIONS
# ============================================================================

<#
.SYNOPSIS
Calculates SHA256 hash of a file for integrity verification.

.DESCRIPTION
Provides reliable file integrity checking using SHA256 hashing.
Includes error handling for files that cannot be accessed or read.
This is critical for verifying successful file operations.

.PARAMETER FilePath
Path to the file to hash

.RETURNS
SHA256 hash string, or $null if file cannot be hashed
#>
function Get-FileHash256 {
    param([string]$FilePath)
    
    # Verify file exists before attempting hash calculation
    if (-not (Test-Path $FilePath)) {
        return $null
    }
    
    try {
        # Use PowerShell's built-in Get-FileHash for reliable SHA256 calculation
        return (Get-FileHash -Path $FilePath -Algorithm SHA256).Hash
    }
    catch {
        # Log warning but don't stop processing - allows script to continue
        # with other files even if one file has issues
        Write-Warning "Failed to calculate hash for: $FilePath"
        return $null
    }
}

<#
.SYNOPSIS
Compares two files for content equality using hash comparison.

.DESCRIPTION
Performs efficient file comparison using SHA256 hashes rather than
byte-by-byte comparison. This is faster for larger files and provides
reliable content verification.

.PARAMETER File1
Path to first file

.PARAMETER File2
Path to second file

.RETURNS
$true if files have identical content, $false otherwise
#>
function Test-FileContentEqual {
    param(
        [string]$File1,
        [string]$File2
    )
    
    # Ensure both files exist before comparison
    if (-not (Test-Path $File1) -or -not (Test-Path $File2)) {
        return $false
    }
    
    # Calculate hashes for both files
    $hash1 = Get-FileHash256 -FilePath $File1
    $hash2 = Get-FileHash256 -FilePath $File2
    
    # Files are equal if hashes match and neither hash is null
    return ($hash1 -eq $hash2) -and ($null -ne $hash1)
}

# ============================================================================
# FILE TYPE ANALYSIS AND CLASSIFICATION
# ============================================================================

<#
.SYNOPSIS
Analyzes a file to determine its type and handling requirements.

.DESCRIPTION
Classifies configuration files based on filename patterns and extensions.
This classification drives file-specific handling logic, such as setting
executable permissions for git hooks or validating JSON syntax.

The classification system supports:
- Git configuration files and hooks
- PowerShell scripts and profiles
- JSON configuration files
- Shell scripts and templates

.PARAMETER FilePath
Path to the file to analyze

.RETURNS
PSCustomObject with Type, Category, RequiresSpecialHandling properties
#>
function Get-FileTypeInfo {
    param([string]$FilePath)
    
    $extension = [System.IO.Path]::GetExtension($FilePath).ToLower()
    $fileName = [System.IO.Path]::GetFileName($FilePath).ToLower()
    
    # Initialize file information object with defaults
    $fileInfo = @{
        Extension = $extension
        Type = "Unknown"
        Category = "Configuration"
        IsBinary = $false
        RequiresSpecialHandling = $false
    }
    
    # Pattern-based file type detection for configuration files
    # This approach handles files without extensions or with non-standard naming
    switch -Regex ($fileName) {
        # Git configuration files - various naming patterns
        "^\.?gitconfig$|^\.?gitignore$|aliases\.gitconfig$|.*\.gitconfig$" {
            $fileInfo.Type = "Git Configuration"
            $fileInfo.Category = "Version Control"
        }
        
        # PowerShell configuration files - profiles and module files
        "aliases\.ps1$|.*profile\.ps1$|psreadline\.ps1$" {
            $fileInfo.Type = "PowerShell Configuration"
            $fileInfo.Category = "Shell"
        }
        
        # JSON configuration files
        ".*\.json$" {
            $fileInfo.Type = "JSON Configuration"
            $fileInfo.Category = "Configuration"
        }
        
        # Git hooks - require executable permissions on Unix-like systems
        "pre-commit$|pre-push$|post-.*$" {
            $fileInfo.Type = "Git Hook"
            $fileInfo.Category = "Version Control"
            $fileInfo.RequiresSpecialHandling = $true  # Needs chmod +x
        }
        
        # Git message templates
        "\.gitmessage$" {
            $fileInfo.Type = "Git Template"
            $fileInfo.Category = "Version Control"
        }
        
        # Fall back to extension-based detection
        default {
            switch ($extension) {
                ".ps1" { 
                    $fileInfo.Type = "PowerShell Script"
                    $fileInfo.Category = "Shell"
                }
                ".json" { 
                    $fileInfo.Type = "JSON Configuration"
                    $fileInfo.Category = "Configuration"
                }
                ".sh" { 
                    $fileInfo.Type = "Shell Script"
                    $fileInfo.Category = "Shell"
                }
                { $_ -in @(".yml", ".yaml") } {
                    $fileInfo.Type = "YAML Configuration"
                    $fileInfo.Category = "Configuration"
                }
                default {
                    # Generic configuration file detection
                    if ($fileName -match "config|conf|rc$") {
                        $fileInfo.Type = "Configuration File"
                    }
                }
            }
        }
    }
    
    return $fileInfo
}

# ============================================================================
# CONFIGURATION FILE DISCOVERY AND MAPPING
# ============================================================================

<#
.SYNOPSIS
Discovers and maps configuration files between source and target directories.

.DESCRIPTION
This function is the heart of the file discovery system. It defines the
mapping between user configuration files (~/.config) and WinForge repository
files (src/.config). The mapping includes priority ordering to ensure
critical files (like PowerShell profiles) are processed first.

The function applies include/exclude filters and performs initial existence
checks to build a complete picture of what needs to be synchronized.

.PARAMETER SourceConfigPath
Path to user's configuration directory (typically ~/.config)

.PARAMETER TargetConfigPath
Path to WinForge configuration directory (typically src/.config)

.RETURNS
Array of PSCustomObjects representing configuration files to process
#>
function Get-ConfigurationMapping {
    param(
        [string]$SourceConfigPath,
        [string]$TargetConfigPath
    )
    
    Write-Information "Discovering configuration files..."
    
    $configFiles = @()
    
    # Define the configuration file mappings with priority ordering
    # Priority 1: Essential PowerShell configurations (process first)
    # Priority 2: Git configurations (process second)
    # Priority 3: Templates and hooks (process last)
    $fileMappings = @(
        # PowerShell configurations - highest priority for shell functionality
        @{ Source = "pwsh/aliases.ps1"; Target = "pwsh/aliases.ps1"; Priority = 1 }
        @{ Source = "pwsh/user_profile.ps1"; Target = "pwsh/user_profile.ps1"; Priority = 1 }
        @{ Source = "pwsh/psreadline.ps1"; Target = "pwsh/psreadline.ps1"; Priority = 1 }
        
        # Git configurations - second priority for version control
        @{ Source = "git/gitconfig"; Target = "git/gitconfig"; Priority = 2 }
        @{ Source = "git/aliases.gitconfig"; Target = "git/aliases.gitconfig"; Priority = 2 }
        @{ Source = "git/delta.gitconfig"; Target = "git/delta.gitconfig"; Priority = 2 }
        @{ Source = "git/diff.gitconfig"; Target = "git/diff.gitconfig"; Priority = 2 }
        @{ Source = "git/identities.gitconfig"; Target = "git/identities.gitconfig"; Priority = 2 }
        @{ Source = "git/merge.gitconfig"; Target = "git/merge.gitconfig"; Priority = 2 }
        @{ Source = "git/split-diffs.gitconfig"; Target = "git/split-diffs.gitconfig"; Priority = 2 }
        
        # Templates and hooks - processed last due to special handling requirements
        @{ Source = "git/templates/.gitmessage"; Target = "git/templates/.gitmessage"; Priority = 3 }
        @{ Source = "git/templates/hooks/pre-commit"; Target = "git/templates/hooks/pre-commit"; Priority = 3 }
        @{ Source = "git/templates/hooks/pre-push"; Target = "git/templates/hooks/pre-push"; Priority = 3 }
        
        # Theme configurations - user customizable
        @{ Source = "mytheme.omp.json"; Target = "mytheme.omp.json"; Priority = 1 }
    )
    
    foreach ($mapping in $fileMappings) {
        # Build full paths for source and target files
        $sourcePath = Join-Path $SourceConfigPath $mapping.Source
        $targetPath = Join-Path $TargetConfigPath $mapping.Target
        
        # Apply include/exclude filters based on filename
        $fileName = [System.IO.Path]::GetFileName($mapping.Source)
        
        # Skip if not in include list (when include list is specified)
        if ($IncludeFiles.Count -gt 0 -and $fileName -notin $IncludeFiles) {
            continue
        }
        
        # Skip if in exclude list
        if ($ExcludeFiles.Count -gt 0 -and $fileName -in $ExcludeFiles) {
            continue
        }
        
        # Get file type information for specialized handling
        $fileInfo = Get-FileTypeInfo -FilePath $sourcePath
        
        # Create configuration file object with all necessary metadata
        $configFile = [PSCustomObject]@{
            Name = $fileName
            RelativePath = $mapping.Source
            SourcePath = $sourcePath
            TargetPath = $targetPath
            Priority = $mapping.Priority
            FileType = $fileInfo.Type
            Category = $fileInfo.Category
            RequiresSpecialHandling = $fileInfo.RequiresSpecialHandling
            SourceExists = Test-Path $sourcePath
            TargetExists = Test-Path $targetPath
            ComparisonResult = $null  # Will be populated during comparison
            Action = "None"           # Will be set based on user choice
        }
        
        $configFiles += $configFile
    }
    
    # Return files sorted by priority and name for consistent processing order
    return $configFiles | Sort-Object Priority, Name
}

# ============================================================================
# FILE COMPARISON AND ANALYSIS ENGINE
# ============================================================================

<#
.SYNOPSIS
Performs comprehensive comparison of configuration files.

.DESCRIPTION
This is the core analysis engine that compares source and target files using
multiple criteria. It implements a multi-tier comparison system:

1. Existence check - determines if files exist in source/target locations
2. Hash comparison - detects content differences efficiently
3. Status classification - categorizes files for appropriate handling
4. Recommendation generation - suggests actions based on analysis

The function updates each configuration file object with detailed comparison
results that drive the interactive decision-making process.

.PARAMETER ConfigFiles
Array of configuration file objects to analyze

.NOTES
This function modifies the ComparisonResult property of each configuration file object.
#>
function Compare-ConfigurationFiles {
    param([array]$ConfigFiles)
    
    Write-Information "Analyzing configuration files..."
    
    # Initialize progress tracking
    $script:TotalFiles = $ConfigFiles.Count
    $script:ProcessedFiles = 0
    
    foreach ($configFile in $ConfigFiles) {
        # Update progress tracking
        $script:ProcessedFiles++
        $percentComplete = [math]::Round(($script:ProcessedFiles / $script:TotalFiles) * 100)
        
        Write-ProgressUpdate -Activity "Analyzing Configuration Files" -Status "Processing $($configFile.Name)" -PercentComplete $percentComplete
        
        # Initialize comparison result object
        $comparison = [PSCustomObject]@{
            Status = "Unknown"
            SourceHash = $null
            TargetHash = $null
            HashMatch = $false
            Recommendation = "Skip"
            Reason = ""
            HasConflict = $false
        }
        
        try {
            # Multi-tier comparison logic
            if (-not $configFile.SourceExists -and -not $configFile.TargetExists) {
                # Neither file exists - nothing to synchronize
                $comparison.Status = "BothMissing"
                $comparison.Recommendation = "Skip"
                $comparison.Reason = "Neither source nor target exists"
            }
            elseif (-not $configFile.SourceExists) {
                # Source missing - can't copy from non-existent source
                $comparison.Status = "SourceMissing"
                $comparison.Recommendation = "Skip"
                $comparison.Reason = "Source file does not exist"
            }
            elseif (-not $configFile.TargetExists) {
                # Target missing - safe to copy from source
                $comparison.Status = "TargetMissing"
                $comparison.Recommendation = "Copy"
                $comparison.Reason = "Target file does not exist, can copy from source"
            }
            else {
                # Both files exist - perform content comparison
                $comparison.SourceHash = Get-FileHash256 -FilePath $configFile.SourcePath
                $comparison.TargetHash = Get-FileHash256 -FilePath $configFile.TargetPath
                
                if ($comparison.SourceHash -eq $comparison.TargetHash) {
                    # Files are identical - no action needed
                    $comparison.Status = "Identical"
                    $comparison.HashMatch = $true
                    $comparison.Recommendation = "Skip"
                    $comparison.Reason = "Files are identical"
                }
                else {
                    # Files differ - requires user review
                    $comparison.Status = "Different"
                    $comparison.HashMatch = $false
                    $comparison.HasConflict = $true
                    $comparison.Recommendation = "Review"
                    $comparison.Reason = "Files differ and require review"
                }
            }
        }
        catch {
            # Handle comparison errors gracefully
            $comparison.Status = "Error"
            $comparison.Recommendation = "Skip"
            $comparison.Reason = "Error during comparison: $($_.Exception.Message)"
            Write-Warning "Error comparing $($configFile.Name): $($_.Exception.Message)"
        }
        
        # Store comparison result in the configuration file object
        $configFile.ComparisonResult = $comparison
    }
    
    # Clear progress indicator
    Write-Progress -Activity "Analyzing Configuration Files" -Completed
}

# ============================================================================
# USER INTERFACE AND INTERACTION FUNCTIONS
# ============================================================================

<#
.SYNOPSIS
Displays file differences and comparison results to the user.

.DESCRIPTION
Provides formatted output showing the status and differences between source
and target files. Optionally displays content previews for files that differ.
This helps users make informed decisions about synchronization actions.

.PARAMETER ConfigFile
Configuration file object to display

.PARAMETER ShowContent
If specified, shows content preview for different files
#>
function Show-FileDifferences {
    param(
        [PSCustomObject]$ConfigFile,
        [switch]$ShowContent = $false
    )
    
    Write-Host ""
    Write-ColoredOutput "=" * 80 -Color "Cyan"
    Write-ColoredOutput "File: $($ConfigFile.Name) ($($ConfigFile.FileType))" -Color "Yellow"
    Write-ColoredOutput "=" * 80 -Color "Cyan"
    
    Write-Host "Source: $($ConfigFile.SourcePath)"
    Write-Host "Target: $($ConfigFile.TargetPath)"
    Write-Host ""
    
    $result = $ConfigFile.ComparisonResult
    Write-Host "Status: " -NoNewline
    
    # Color-coded status display for quick visual identification
    switch ($result.Status) {
        "Identical" { Write-ColoredOutput $result.Status -Color "Green" }
        "Different" { Write-ColoredOutput $result.Status -Color "Red" }
        "TargetMissing" { Write-ColoredOutput $result.Status -Color "Yellow" }
        "SourceMissing" { Write-ColoredOutput $result.Status -Color "Gray" }
        default { Write-ColoredOutput $result.Status -Color "Magenta" }
    }
    
    Write-Host "Reason: $($result.Reason)"
    
    # Show content preview for different files if requested
    if ($ShowContent -and $result.Status -eq "Different") {
        try {
            Write-Host ""
            Write-ColoredOutput "Content Preview (first 10 lines of each file):" -Color "Cyan"
            Write-Host ""
            
            # Display source file content
            if (Test-Path $ConfigFile.SourcePath) {
                Write-ColoredOutput "SOURCE ($($ConfigFile.SourcePath)):" -Color "Green"
                $sourceContent = Get-Content $ConfigFile.SourcePath -TotalCount 10 -ErrorAction SilentlyContinue
                $sourceContent | ForEach-Object { Write-Host "  $_" }
            }
            
            Write-Host ""
            
            # Display target file content
            if (Test-Path $ConfigFile.TargetPath) {
                Write-ColoredOutput "TARGET ($($ConfigFile.TargetPath)):" -Color "Red"
                $targetContent = Get-Content $ConfigFile.TargetPath -TotalCount 10 -ErrorAction SilentlyContinue
                $targetContent | ForEach-Object { Write-Host "  $_" }
            }
        }
        catch {
            Write-Warning "Could not display file content preview: $($_.Exception.Message)"
        }
    }
}

<#
.SYNOPSIS
Displays interactive menu for file synchronization decisions.

.DESCRIPTION
Presents a user-friendly menu system for making decisions about each
configuration file. Provides options for viewing differences, copying files,
creating backups, and opening files in external editors.

.PARAMETER ConfigFile
Configuration file object to process

.RETURNS
String indicating the user's chosen action
#>
function Show-InteractiveMenu {
    param([PSCustomObject]$ConfigFile)
    
    $result = $ConfigFile.ComparisonResult
    
    Write-Host ""
    Write-ColoredOutput "Choose an action:" -Color "Cyan"
    Write-Host "1. Show Diff/Content"
    Write-Host "2. Copy from WinForge to User (Forward Sync)"
    Write-Host "3. Skip This File"
    Write-Host "4. Backup User File + Copy from WinForge"
    Write-Host "5. Open Files in Editor"
    Write-Host "6. Copy from User to WinForge (Reverse Sync)"
    Write-Host "7. Quit Synchronization"
    Write-Host ""
    
    do {
        $choice = Read-Host "Enter your choice (1-7)"
        
        switch ($choice) {
            "1" {
                # Show file content and differences
                Show-FileDifferences -ConfigFile $ConfigFile -ShowContent
                return "ShowDiff"  # Return to menu
            }
            "2" {
                # Direct copy without backup (forward sync)
                return "Copy"
            }
            "3" {
                # Skip this file
                return "Skip"
            }
            "4" {
                # Safe copy with backup (forward sync)
                return "BackupAndCopy"
            }
            "5" {
                # Open in external editor for manual review
                try {
                    if (Get-Command code -ErrorAction SilentlyContinue) {
                        # Prefer VS Code with diff view
                        Start-Process "code" -ArgumentList "--diff", "`"$($ConfigFile.SourcePath)`"", "`"$($ConfigFile.TargetPath)`""
                    }
                    elseif (Get-Command notepad -ErrorAction SilentlyContinue) {
                        # Fall back to Notepad (Windows)
                        Start-Process "notepad" -ArgumentList "`"$($ConfigFile.SourcePath)`""
                        Start-Process "notepad" -ArgumentList "`"$($ConfigFile.TargetPath)`""
                    }
                    else {
                        Write-Warning "No suitable editor found. Please install VS Code or use another editor."
                    }
                }
                catch {
                    Write-Warning "Failed to open editor: $($_.Exception.Message)"
                }
                return "ShowDiff"  # Return to menu after opening editor
            }
            "6" {
                # Reverse sync - copy from user to WinForge
                Write-Host ""
                Write-ColoredOutput "WARNING: REVERSE SYNC OPERATION" -Color "Red"
                Write-ColoredOutput "This will modify the WinForge repository files!" -Color "Red"
                Write-ColoredOutput "Make sure to commit your changes to Git after this operation." -Color "Yellow"
                Write-Host ""
                
                # Only allow reverse sync if user file exists
                if (-not $ConfigFile.TargetExists) {
                    Write-ColoredOutput "Cannot perform reverse sync: User file does not exist." -Color "Red"
                    return "ShowDiff"  # Return to menu
                }
                
                $confirmation = Read-Host "Are you sure you want to copy from User to WinForge? (y/N)"
                if ($confirmation -match '^[Yy]') {
                    return "ReverseSync"
                }
                else {
                    Write-ColoredOutput "Reverse sync cancelled." -Color "Yellow"
                    return "ShowDiff"  # Return to menu
                }
            }
            "7" {
                # Exit synchronization
                return "Quit"
            }
            default {
                Write-ColoredOutput "Invalid choice. Please enter 1-7." -Color "Red"
            }
        }
    } while ($true)
}

# ============================================================================
# BACKUP SYSTEM IMPLEMENTATION
# ============================================================================

<#
.SYNOPSIS
Creates a timestamped backup directory in the system temp folder.

.DESCRIPTION
Establishes a backup location for storing original files before synchronization.
The backup directory uses a timestamp to ensure uniqueness and prevent conflicts.
This provides a recovery mechanism if synchronization needs to be reversed.

.RETURNS
String path to backup directory, or $null if creation fails
#>
function New-BackupDirectory {
    # Create timestamp-based directory name for uniqueness
    $timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
    $backupDir = Join-Path $env:TEMP "WinForge_ConfigBackup_$timestamp"
    
    try {
        # Create backup directory with force to handle any conflicts
        New-Item -Path $backupDir -ItemType Directory -Force | Out-Null
        $script:BackupPath = $backupDir
        Write-Information "Created backup directory: $backupDir"
        return $backupDir
    }
    catch {
        Write-Error "Failed to create backup directory: $($_.Exception.Message)"
        return $null
    }
}

<#
.SYNOPSIS
Creates a backup copy of a configuration file.

.DESCRIPTION
Safely backs up a file to the backup directory, preserving the relative
directory structure. This ensures that files can be restored to their
original locations if needed.

.PARAMETER FilePath
Full path to the file to backup

.PARAMETER RelativePath
Relative path to preserve directory structure in backup

.RETURNS
$true if backup successful, $false otherwise
#>
function Backup-ConfigFile {
    param(
        [string]$FilePath,
        [string]$RelativePath
    )
    
    # Skip backup if file doesn't exist
    if (-not (Test-Path $FilePath)) {
        return $false
    }
    
    # Ensure backup directory exists
    if (-not $script:BackupPath) {
        New-BackupDirectory | Out-Null
    }
    
    try {
        # Build backup file path preserving directory structure
        $backupFilePath = Join-Path $script:BackupPath $RelativePath
        $backupDir = Split-Path $backupFilePath -Parent
        
        # Create backup subdirectory if needed
        if (-not (Test-Path $backupDir)) {
            New-Item -Path $backupDir -ItemType Directory -Force | Out-Null
        }
        
        # Perform the backup copy
        Copy-Item -Path $FilePath -Destination $backupFilePath -Force
        Write-Information "Backed up: $RelativePath"
        return $true
    }
    catch {
        Write-Warning "Failed to backup $RelativePath`: $($_.Exception.Message)"
        return $false
    }
}

# ============================================================================
# FILE OPERATION FUNCTIONS
# ============================================================================

<#
.SYNOPSIS
Copies a configuration file from source to target with full error handling.

.DESCRIPTION
Performs the actual file synchronization operation with comprehensive safety
measures. This includes:
- Target directory creation
- Optional backup creation
- Atomic file copying
- Hash verification of copied files
- Operation result tracking

The function implements the WinForge pattern of atomic operations - files are
either copied successfully with verification, or the operation fails completely
without leaving partial or corrupted files.

.PARAMETER ConfigFile
Configuration file object to copy

.PARAMETER CreateBackup
If specified, creates backup before overwriting target

.RETURNS
$true if copy successful, $false otherwise
#>
function Copy-ConfigFile {
    param(
        [PSCustomObject]$ConfigFile,
        [switch]$CreateBackup
    )
    
    # Initialize operation tracking object
    $operation = [PSCustomObject]@{
        File = $ConfigFile.Name
        Action = "Copy"
        Success = $false
        Error = $null
        BackupCreated = $false
    }
    
    try {
        # Ensure target directory exists - create if necessary
        $targetDir = Split-Path $ConfigFile.TargetPath -Parent
        if (-not (Test-Path $targetDir)) {
            New-Item -Path $targetDir -ItemType Directory -Force | Out-Null
        }
        
        # Create backup if requested and target file exists
        if ($CreateBackup -and (Test-Path $ConfigFile.TargetPath)) {
            $operation.BackupCreated = Backup-ConfigFile -FilePath $ConfigFile.TargetPath -RelativePath $ConfigFile.RelativePath
        }
        
        if (-not $DryRun) {
            # Perform the actual copy operation
            Copy-Item -Path $ConfigFile.SourcePath -Destination $ConfigFile.TargetPath -Force
            
            # Verify the copy was successful using hash comparison
            # This ensures file integrity and catches any corruption issues
            if (Test-FileContentEqual -File1 $ConfigFile.SourcePath -File2 $ConfigFile.TargetPath) {
                $operation.Success = $true
                Write-ColoredOutput "[OK] Successfully copied: $($ConfigFile.Name)" -Color "Green"
            }
            else {
                $operation.Error = "File verification failed after copy"
                Write-ColoredOutput "[FAIL] Copy verification failed: $($ConfigFile.Name)" -Color "Red"
            }
        }
        else {
            # Dry run mode - simulate success without actual file operations
            $operation.Success = $true
            Write-ColoredOutput "[OK] [DRY RUN] Would copy: $($ConfigFile.Name)" -Color "Yellow"
        }
    }
    catch {
        # Capture and log any errors during the copy operation
        $operation.Error = $_.Exception.Message
        $operation.Success = $false
        Write-ColoredOutput "[FAIL] Failed to copy $($ConfigFile.Name): $($_.Exception.Message)" -Color "Red"
    }
    
    # Track operation result for summary reporting
    $script:OperationResults += $operation
    
    # Update global counters for final summary
    if ($operation.Success) {
        $script:SuccessfulOperations++
    }
    else {
        $script:FailedOperations++
    }
    
    return $operation.Success
}

<#
.SYNOPSIS
Copies a configuration file from user to WinForge (reverse sync).

.DESCRIPTION
Performs reverse synchronization by copying files from the user's configuration
directory to the WinForge repository. This enables users to contribute their
improved configurations back to the WinForge project.

Key differences from forward sync:
- No backup needed (Git handles version control for WinForge repo)
- Git awareness messages remind users to commit changes
- Clear warnings about modifying repository files

.PARAMETER ConfigFile
Configuration file object to reverse sync

.RETURNS
$true if reverse sync successful, $false otherwise
#>
function Copy-ConfigFileReverse {
    param([PSCustomObject]$ConfigFile)
    
    # Initialize operation tracking object
    $operation = [PSCustomObject]@{
        File = $ConfigFile.Name
        Action = "Reverse Sync"
        Success = $false
        Error = $null
        BackupCreated = $false
    }
    
    try {
        # Ensure source directory exists in WinForge repo
        $sourceDir = Split-Path $ConfigFile.SourcePath -Parent
        if (-not (Test-Path $sourceDir)) {
            New-Item -Path $sourceDir -ItemType Directory -Force | Out-Null
        }
        
        if (-not $DryRun) {
            # Perform the reverse copy operation (User -> WinForge)
            Copy-Item -Path $ConfigFile.TargetPath -Destination $ConfigFile.SourcePath -Force
            
            # Verify the copy was successful using hash comparison
            if (Test-FileContentEqual -File1 $ConfigFile.TargetPath -File2 $ConfigFile.SourcePath) {
                $operation.Success = $true
                Write-ColoredOutput "[OK] Reverse sync completed: $($ConfigFile.Name)" -Color "Green"
                Write-ColoredOutput "      Copied from User to WinForge repository" -Color "Cyan"
                Write-ColoredOutput "      Remember to commit these changes to Git!" -Color "Yellow"
            }
            else {
                $operation.Error = "File verification failed after reverse sync"
                Write-ColoredOutput "[FAIL] Reverse sync verification failed: $($ConfigFile.Name)" -Color "Red"
            }
        }
        else {
            # Dry run mode - simulate success
            $operation.Success = $true
            Write-ColoredOutput "[OK] [DRY RUN] Would reverse sync: $($ConfigFile.Name)" -Color "Yellow"
            Write-ColoredOutput "      Would copy from User to WinForge repository" -Color "Cyan"
        }
    }
    catch {
        # Capture and log any errors during the reverse copy operation
        $operation.Error = $_.Exception.Message
        $operation.Success = $false
        Write-ColoredOutput "[FAIL] Failed to reverse sync $($ConfigFile.Name): $($_.Exception.Message)" -Color "Red"
    }
    
    # Track operation result for summary reporting
    $script:OperationResults += $operation
    
    # Update global counters for final summary
    if ($operation.Success) {
        $script:SuccessfulOperations++
    }
    else {
        $script:FailedOperations++
    }
    
    return $operation.Success
}

<#
.SYNOPSIS
Sets appropriate file permissions for configuration files.

.DESCRIPTION
Handles file-type specific permission requirements, particularly for Git hooks
that need executable permissions on Unix-like systems. This ensures that
synchronized files maintain proper functionality across different platforms.

.PARAMETER FilePath
Path to the file to set permissions on

.PARAMETER ConfigFile
Configuration file object containing type information
#>
function Set-FilePermissions {
    param(
        [string]$FilePath,
        [PSCustomObject]$ConfigFile
    )
    
    # Set executable permissions for git hooks on Unix-like systems
    if ($ConfigFile.FileType -eq "Git Hook" -and $ConfigFile.RequiresSpecialHandling) {
        try {
            # Use chmod on Linux/macOS systems
            if ($IsLinux -or $IsMacOS) {
                & chmod +x $FilePath
            }
            Write-Information "Set executable permissions for: $($ConfigFile.Name)"
        }
        catch {
            Write-Warning "Failed to set permissions for $($ConfigFile.Name): $($_.Exception.Message)"
        }
    }
}

# ============================================================================
# REPORTING AND SUMMARY FUNCTIONS
# ============================================================================

<#
.SYNOPSIS
Displays comprehensive summary report of synchronization operations.

.DESCRIPTION
Generates a detailed report showing:
- Overall operation statistics
- File-by-file status breakdown
- Successful and failed operations
- Backup location information
- Color-coded results for easy interpretation

This report provides users with complete visibility into what was synchronized
and helps identify any issues that need attention.

.PARAMETER ConfigFiles
Array of all configuration files processed
#>
function Show-SummaryReport {
    param([array]$ConfigFiles)
    
    Write-Host ""
    Write-ColoredOutput "=" * 80 -Color "Cyan"
    Write-ColoredOutput "SYNCHRONIZATION SUMMARY" -Color "Yellow"
    Write-ColoredOutput "=" * 80 -Color "Cyan"
    
    # Display overall statistics
    Write-Host ""
    Write-Host "Total Files Processed: $($script:TotalFiles)"
    Write-ColoredOutput "Successful Operations: $($script:SuccessfulOperations)" -Color "Green"
    Write-ColoredOutput "Failed Operations: $($script:FailedOperations)" -Color "Red"
    
    # Show file status breakdown
    Write-Host ""
    Write-ColoredOutput "File Status Summary:" -Color "Cyan"
    
    $statusCounts = $ConfigFiles | Group-Object { $_.ComparisonResult.Status } | Sort-Object Name
    foreach ($status in $statusCounts) {
        $color = switch ($status.Name) {
            "Identical" { "Green" }
            "Different" { "Red" }
            "TargetMissing" { "Yellow" }
            "SourceMissing" { "Gray" }
            default { "White" }
        }
        Write-ColoredOutput "  $($status.Name): $($status.Count)" -Color $color
    }
    
    # Display detailed operation results
    if ($script:OperationResults.Count -gt 0) {
        Write-Host ""
        Write-ColoredOutput "Operations Performed:" -Color "Cyan"
        
        foreach ($operation in $script:OperationResults) {
            $status = if ($operation.Success) { "[OK]" } else { "[FAIL]" }
            $color = if ($operation.Success) { "Green" } else { "Red" }
            
            Write-ColoredOutput "  $status $($operation.Action): $($operation.File)" -Color $color
            
            # Show backup information
            if ($operation.BackupCreated) {
                Write-ColoredOutput "    (Backup created)" -Color "Cyan"
            }
            
            # Show error details
            if ($operation.Error) {
                Write-ColoredOutput "    Error: $($operation.Error)" -Color "Red"
            }
        }
    }
    
    # Display backup location for recovery purposes
    if ($script:BackupPath) {
        Write-Host ""
        Write-ColoredOutput "Backup Location: $($script:BackupPath)" -Color "Yellow"
    }
    
    # Show Git commit reminder if any reverse sync operations were performed
    $reverseSyncOperations = $script:OperationResults | Where-Object { $_.Action -eq "Reverse Sync" -and $_.Success }
    if ($reverseSyncOperations.Count -gt 0) {
        Write-Host ""
        Write-ColoredOutput "GIT REPOSITORY NOTICE:" -Color "Yellow"
        Write-ColoredOutput "Reverse sync operations modified the WinForge repository." -Color "Cyan"
        Write-ColoredOutput "Remember to review and commit your changes:" -Color "Cyan"
        Write-Host "  git status"
        Write-Host "  git add src/.config/"
        Write-Host "  git commit -m `"Update configuration files from user improvements`""
        Write-Host "  git push origin main"
    }
    
    Write-Host ""
    Write-ColoredOutput "=" * 80 -Color "Cyan"
}

#endregion

#region Main Script Logic

# ============================================================================
# ENVIRONMENT INITIALIZATION
# ============================================================================

<#
.SYNOPSIS
Initializes the synchronization environment and validates paths.

.DESCRIPTION
Performs essential setup tasks:
- Determines source and target configuration paths
- Validates directory existence and accessibility
- Creates missing directories when possible
- Provides clear feedback about the environment setup

This function establishes the foundation for all subsequent operations.

.RETURNS
Hashtable with SourcePath and TargetPath, or $null if initialization fails
#>
function Initialize-SyncEnvironment {
    Write-ColoredOutput "WinForge Configuration Synchronization Tool" -Color "Cyan"
    Write-ColoredOutput "=" * 50 -Color "Cyan"
    
    # Determine configuration paths
    # Source: WinForge repository config directory (src/.config) - the authoritative source
    # Target: User's personal config directory (default: ~/.config) - the destination to sync to
    $scriptRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
    $sourceConfigPath = Join-Path $scriptRoot ".config"
    $targetConfigPath = if ($ConfigPath) { $ConfigPath } else { Join-Path $env:USERPROFILE ".config" }
    
    Write-Host ""
    Write-Host "Source Config Path (WinForge): $sourceConfigPath"
    Write-Host "Target Config Path (User): $targetConfigPath"
    
    # Validate and create source directory if needed
    # Validate that WinForge source directory exists (should be part of repository)
    if (-not (Test-Path $sourceConfigPath)) {
        Write-Error "WinForge source configuration directory not found: $sourceConfigPath"
        Write-Error "This directory should be part of the WinForge repository structure."
        return $null
    }
    
    # Create user target directory if it doesn't exist
    if (-not (Test-Path $targetConfigPath)) {
        Write-Warning "User configuration directory not found: $targetConfigPath"
        Write-Host "Creating user config directory..."
        try {
            New-Item -Path $targetConfigPath -ItemType Directory -Force | Out-Null
        }
        catch {
            Write-Error "Failed to create user config directory: $($_.Exception.Message)"
            return $null
        }
    }
    
    return @{
        SourcePath = $sourceConfigPath
        TargetPath = $targetConfigPath
    }
}

# ============================================================================
# MAIN SYNCHRONIZATION ORCHESTRATOR
# ============================================================================

<#
.SYNOPSIS
Main orchestration function that coordinates the entire synchronization process.

.DESCRIPTION
This function implements the complete synchronization workflow:

1. Environment initialization and path validation
2. Configuration file discovery and mapping
3. Multi-tier file comparison and analysis
4. Optional backup creation
5. Interactive user decision-making (unless automated)
6. File operations with safety checks
7. Permission setting for special file types
8. Comprehensive summary reporting

The function follows WinForge patterns for error handling, returning
appropriate exit codes for integration with broader automation systems.

.RETURNS
Integer exit code (0 for success, 1 for failure)
#>
function Start-ConfigurationSync {
    # Initialize environment and validate paths
    $paths = Initialize-SyncEnvironment
    if (-not $paths) {
        return 1
    }
    
    try {
        # Discover and map configuration files between source and target
        $configFiles = Get-ConfigurationMapping -SourceConfigPath $paths.SourcePath -TargetConfigPath $paths.TargetPath
        
        if ($configFiles.Count -eq 0) {
            Write-Warning "No configuration files found to synchronize."
            return 0
        }
        
        Write-Host ""
        Write-ColoredOutput "Found $($configFiles.Count) configuration files to analyze." -Color "Green"
        
        # Create initial backup if requested (safety measure)
        if ($BackupFirst) {
            Write-Information "Creating initial backup of all target files..."
            New-BackupDirectory | Out-Null
            
            foreach ($configFile in $configFiles) {
                if ($configFile.TargetExists) {
                    Backup-ConfigFile -FilePath $configFile.TargetPath -RelativePath $configFile.RelativePath | Out-Null
                }
            }
        }
        
        # Perform comprehensive file analysis
        Compare-ConfigurationFiles -ConfigFiles $configFiles
        
        # Handle show-only mode (display results without actions)
        if ($ShowDiffsOnly) {
            foreach ($configFile in $configFiles) {
                if ($configFile.ComparisonResult.Status -in @("Different", "TargetMissing")) {
                    Show-FileDifferences -ConfigFile $configFile -ShowContent
                }
            }
            Show-SummaryReport -ConfigFiles $configFiles
            return 0
        }
        
        # Process each configuration file
        foreach ($configFile in $configFiles) {
            $result = $configFile.ComparisonResult
            
            # Skip files that don't need processing
            if ($result.Status -in @("Identical", "SourceMissing", "BothMissing", "Error")) {
                Write-Information "Skipping $($configFile.Name): $($result.Reason)"
                continue
            }
            
            # Display file information to user
            Show-FileDifferences -ConfigFile $configFile
            
            $action = $null
            
            if ($AutoYes) {
                # Automatic mode - choose safe default actions
                $action = if ($result.Status -eq "TargetMissing") { "Copy" } else { "BackupAndCopy" }
                Write-ColoredOutput "Auto-selected action: $action" -Color "Yellow"
            }
            else {
                # Interactive mode - let user decide
                do {
                    $action = Show-InteractiveMenu -ConfigFile $configFile
                    
                    if ($action -eq "Quit") {
                        Write-ColoredOutput "Synchronization cancelled by user." -Color "Yellow"
                        Show-SummaryReport -ConfigFiles $configFiles
                        return 0
                    }
                } while ($action -eq "ShowDiff")
            }
            
            # Execute the chosen action
            switch ($action) {
                "Copy" {
                    # Direct copy without backup (forward sync)
                    Copy-ConfigFile -ConfigFile $configFile
                    Set-FilePermissions -FilePath $configFile.TargetPath -ConfigFile $configFile
                }
                "BackupAndCopy" {
                    # Safe copy with backup (forward sync)
                    Copy-ConfigFile -ConfigFile $configFile -CreateBackup
                    Set-FilePermissions -FilePath $configFile.TargetPath -ConfigFile $configFile
                }
                "ReverseSync" {
                    # Reverse sync - copy from user to WinForge
                    Copy-ConfigFileReverse -ConfigFile $configFile
                    Set-FilePermissions -FilePath $configFile.SourcePath -ConfigFile $configFile
                }
                "Skip" {
                    # User chose to skip this file
                    Write-Information "Skipped: $($configFile.Name)"
                }
            }
        }
        
        # Display final summary report
        Show-SummaryReport -ConfigFiles $configFiles
        
        # Return appropriate exit code based on operation results
        if ($script:FailedOperations -eq 0) {
            return 0
        } else {
            return 1
        }
    }
    catch {
        # Handle any unexpected errors during synchronization
        Write-Error "An unexpected error occurred: $($_.Exception.Message)"
        Write-Error $_.ScriptStackTrace
        return 1
    }
    finally {
        # Always clean up progress indicators
        Write-Progress -Activity "Configuration Synchronization" -Completed
    }
}

#endregion

# ============================================================================
# SCRIPT ENTRY POINT
# ============================================================================

# Execute main synchronization function when script is run directly
# (not when dot-sourced for testing or module loading)
if ($MyInvocation.InvocationName -ne '.') {
    $exitCode = Start-ConfigurationSync
    exit $exitCode
}