function Install-WorkloadPackages {
    param(
        [Parameter(Mandatory=$true)]
        [string[]]$WorkloadFolders,

        [Parameter(Mandatory=$false)]
        [string]$WorkloadsRoot = "$PSScriptRoot\..\workloads"
    )

    # Initialize hash tables to store unique packages and scripts
    $wingetPackages = @{}
    $msstorePackages = @{}
    $scripts = @{}

    foreach ($folder in $WorkloadFolders) {
        $workloadPath = Join-Path -Path $WorkloadsRoot -ChildPath $folder
        if (-not (Test-Path $workloadPath)) {
            Write-Warning "Workload folder '$workloadPath' does not exist. Ignoring for now."
            continue
        }

        $yamlFiles = Get-ChildItem -Path $workloadPath -Filter '*.yaml' -File
        foreach ($yamlFile in $yamlFiles) {
            Write-Host "::: processing workload file: $($yamlFile.FullName)"

            # Load YAML content
            $workload = Get-Content -Path $yamlFile.FullName -Raw | ConvertFrom-Yaml

            if ($null -ne $workload.packages.winget) { 
                foreach ($packageId in $workload.packages.winget) {
                    $wingetPackages[$packageId] = $true
                }
            }

            if ($null -ne $workload.packages.msstore) { 
                foreach ($packageId in $workload.packages.msstore) {
                    $msstorePackages[$packageId] = $true
                }
            }

            $scriptsPath = Join-Path -Path $yamlFile.DirectoryName -ChildPath "__scripts__"

            if ($null -ne $workload.scripts) {
                foreach ($script in $workload.scripts) {
                    Write-Host "::: processing script: $script"
                    $scripts[$script] = Join-Path -Path $scriptsPath -ChildPath $script
                    Write-Host "::: found script: $($scripts[$script])"
                }
            }
        }
    }

    # Install collected winget packages
    if ($wingetPackages.Count -gt 0) {
        Write-Host "::: Installing $($wingetPackages.Count) winget packages..."
        foreach ($packageId in $wingetPackages.Keys) {
            Write-Host "::: installing package '$packageId' via winget..."
            winget install --id $packageId --accept-source-agreements --accept-package-agreements -e --source winget
        }
    }

    # Install collected msstore packages
    if ($msstorePackages.Count -gt 0) {
        Write-Host "::: Installing $($msstorePackages.Count) Microsoft Store packages..."
        foreach ($packageId in $msstorePackages.Keys) {
            Write-Host "::: installing package '$packageId' via msstore..."
            winget install --id $packageId --accept-source-agreements --accept-package-agreements -e --source msstore
        }
    }

    # Execute collected scripts
    if ($scripts.Count -gt 0) {
        Write-Host "::: Executing $($scripts.Count) custom scripts..."
        foreach ($script in $scripts.Keys) {
            $scriptPath = $scripts[$script]
            if (Test-Path $scriptPath) {
                Write-Host "::: executing script '$scriptPath'..."
                try {
                    pwsh.exe -NoProfile -ExecutionPolicy Bypass -File $scriptPath
                }
                catch {
                    Write-Warning "Failed to execute script '$script': $scriptPath"
                }
            }
            else {
                Write-Warning "Script not found: '$scriptPath'"
            }
        }
    }    
}

Export-ModuleMember -Function Install-WorkloadPackages
