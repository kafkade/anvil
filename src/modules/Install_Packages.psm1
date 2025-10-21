
# Function to check if a package is installed via winget
function Check_IsPackageInstalled {
    param (
        [string]$PackageId,
        [string[]]$InstalledPackages = $null
    )

    $result = winget list --id $PackageId -e
    if ($result -match $PackageId) {
        return $true
    }
    
    return $false
}

function Install_ChocoPackages {
    Param (
        [Parameter()]
        [string[]] $Packages
    )

    $installed = choco list -l
    
    $Packages | ForEach-Object {
        $p = $_.Trim()
        if ($p.StartsWith("#")) {
            Write-Host "::: skipping $p"
            continue
        }
        $found = $null -ne ($installed | Where-Object { $_ -match $p })
        Write-Host "::: checking $p".PadRight(50, " ") $found
        if (-not $found)
        {
            Write-Host "::: installing $p"
            choco install $p --yes
        }
    }
}

function Install_WinGetPackages {
    Param (
        [Parameter()]
        [string[]] $Packages
    )

    $Packages | ForEach-Object {
        $p = $_.Trim()
        if ($p.StartsWith("#")) {
            Write-Host "::: skipping $p"
            continue
        }
        $found = 0 -eq ((winget list -q $p) -match "No installed package found").Length
        Write-Host "::: checking $p".PadRight(50, " ") $found
        if (-not $found)
        {
            Write-Host "::: installing $p"
            winget install $p
        }
    }
}

function Install_Packages {
	Param(
        [Parameter()]
        [string[]] $ChocoPackages,
        [Parameter()]
        [string[]] $WinGetPackages
    )

    if ($ChocoPackages) {
        Write-Host "::: installing choco packages"
        Install_ChocoPackages -Packages $ChocoPackages
    }

    if ($WinGetPackages) {
        Write-Host "::: installing winget packages"
        Install_WinGetPackages -Packages $WinGetPackages
    }
}

Export-ModuleMember -Function Install_Packages

