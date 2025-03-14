# Script paths
$vsConfigPath = Join-Path $PSScriptRoot "..\__config__\vs-enterprise.vsconfig"
$vsInstallerPath = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vs_installer.exe"
$vsInstallPath = "${env:ProgramFiles}\Microsoft Visual Studio\2022\Enterprise"
$tempConfigPath = Join-Path $env:TEMP "current-vs-config.vsconfig"

Write-Host "`n>>> Checking Visual Studio Enterprise configuration >>>`n"

if (-not (Test-Path $vsConfigPath)) {
    Write-Error "vsconfig file not found at: $vsConfigPath"
    exit 1
}

if (-not (Test-Path $vsInstallerPath)) {
    Write-Error "Visual Studio Installer not found at: $vsInstallerPath"
    exit 1
}

# Check if VS is installed
if (-not (Test-Path $vsInstallPath)) {
    Write-Warning "Visual Studio Enterprise is not installed. Proceeding with installation..."
} else {
    Write-Host "Exporting current Visual Studio configuration..."
    
    try {
        # Export current configuration
        Write-Verbose $vsInstallerPath
        Write-Verbose $vsInstallPath
        Write-Verbose $tempConfigPath

        $exportProcess = Start-Process -FilePath $vsInstallerPath -ArgumentList "export", "--installPath", "`"$vsInstallPath`"", "--config", "`"$tempConfigPath`"" -Wait -PassThru -NoNewWindow
        
        if ($exportProcess.ExitCode -eq 0) {
            Write-Host "Configuration exported successfully."
            
            # Compare configurations
            $currentConfig = Get-Content $tempConfigPath | ConvertFrom-Json
            $newConfig = Get-Content $vsConfigPath | ConvertFrom-Json
            
            # Compare the configurations (excluding metadata fields)
            $currentComponents = $currentConfig.components | Sort-Object -Property id
            $newComponents = $newConfig.components | Sort-Object -Property id
            
            $isDifferent = $false
            
            if ($currentComponents.Count -ne $newComponents.Count) {
                $isDifferent = $true
                Write-Host "Different number of components detected."
            } else {
                for ($i = 0; $i -lt $currentComponents.Count; $i++) {
                    if ($currentComponents[$i].id -ne $newComponents[$i].id) {
                        $isDifferent = $true
                        Write-Host "Configuration difference detected in component: $($newComponents[$i].id)"
                        break
                    }
                }
            }
            
            if (-not $isDifferent) {
                Write-Host "Current configuration matches desired configuration. No update needed."
                exit 0
            }
            
            Write-Host "Configuration differences found. Proceeding with update..."
        } else {
            Write-Warning "Failed to export current configuration. Proceeding with update as precaution..."
        }
    } catch {
        Write-Warning "Error during configuration comparison: $_"
        Write-Warning "Proceeding with update as precaution..."
    } finally {
        # Cleanup temp config file if it exists
        if (Test-Path $tempConfigPath) {
            Remove-Item $tempConfigPath -Force
        }
    }
}

try {
    # Update Visual Studio Enterprise with the specified configuration
    $process = Start-Process -FilePath $vsInstallerPath -ArgumentList "modify", "--installPath", "`"${env:ProgramFiles}\Microsoft Visual Studio\2022\Enterprise`"", "--config", "`"$vsConfigPath`"", "--passive", "--norestart" -Wait -PassThru
    switch ($process.ExitCode) {
        0 { 
            Write-Host "Visual Studio Enterprise update completed successfully."
        }
        3010 {
            Write-Host "Visual Studio Enterprise update completed successfully."
            Write-Warning "A system restart is required to complete the installation."
        }
        default {
            Write-Error "Visual Studio Enterprise update failed with exit code: $($process.ExitCode)"
        }
    }
} catch {
    Write-Error "An error occurred during Visual Studio Enterprise update: $_"
}
