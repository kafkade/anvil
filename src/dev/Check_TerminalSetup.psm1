Import-Module $PSScriptRoot\..\modules\Execute_Check.psm1
Import-Module $PSScriptRoot\..\modules\Install_Packages.psm1

function Get-RandomLetters {
    $letters = (65..90) + (97..122) | Get-Random -Count 16 | ForEach-Object { [char]$_ }    
    return -join $letters
}

function Install_Fonts {
    Param(
        [Parameter()]
        [string] $Description,
        [Parameter()]
        [string]$Url
    )
    $randomId = Get-RandomLetters
    Write-Host "::: downloading font..."
    $tempFolder = [System.IO.Path]::GetTempPath()
    $tempFile = [System.IO.Path]::Combine($tempFolder, "${randomId}.zip")
    Invoke-WebRequest -Uri $Url -OutFile $tempFile
    Write-Host "::: extracting font..."
    $extractedFolder = [System.IO.Path]::Combine($tempFolder, "${randomId}")
    if (-not (Test-Path $extractedFolder)) {
        New-Item -ItemType Directory -Path $extractedFolder
    }
    Expand-Archive -Path $tempFile -DestinationPath $extractedFolder

    try {
        $fontsDir = [System.Environment]::GetFolderPath("Fonts")
        $extractedFonts = Get-ChildItem $extractedFolder -Filter *.?tf
        $fontsNamespace = (New-Object -ComObject Shell.Application).Namespace(0x14)
        foreach ($font in $extractedFonts) {
            $fontPath = [System.IO.Path]::Combine($fontsDir, $font.Name)
            if (-not(Test-Path -Path $fontPath)) {
                Write-Host "::: installing font $($font.Name)"
                $fontsNamespace.CopyHere($font.FullName)
            }
            else {
                Write-Host "::: skipping font $($font.Name), already installed"
            }
        }

        Get-ChildItem $extractedFolder -Filter *.?tf | Select-Object -ExpandProperty FullName | Copy-Item  -Destination $fontsDir
    }
    catch {
        Write-Error $_.Exception
        exit 1
    }
    finally {
        Write-Host "::: cleaning up"
        Remove-Item $tempFile
        Remove-Item $extractedFolder -Recurse
    }
}

function Check_FontsInstalled {
    $desc = "checking if Cascaydia NF are installed"
    $check = { (New-Object System.Drawing.Text.InstalledFontCollection).Families | Where-Object { $_.Name -match "CaskaydiaCove Nerd Font" } }
    $fix = { Install_Fonts -Url "https://github.com/ryanoasis/nerd-fonts/releases/download/v2.2.2/CascadiaCode.zip" }
    Execute_Check -Description $desc -CheckAction $check -FixAction $fix -ErrorMessage "Cascaydia NF needs to be installed"
}

function Update_WindowsTerminalSettings {
    Write-Host "::: updating Windows Terminal settings"
    $settingsPath = [System.IO.Path]::Combine($env:LOCALAPPDATA, "Packages", "Microsoft.WindowsTerminal_8wekyb3d8bbwe", "LocalState", "settings.json")
    $settings = Get-Content -Path $settingsPath -Raw | ConvertFrom-Json
    $settings.profiles.list | Where-Object { $_.name -eq "Powershell" } | ForEach-Object {
        if (-not $_.font) {
            $font = [PSCustomObject]@{
                face = $null
                size = $null
            }
            $_ | Add-Member -NotePropertyName font -NotePropertyValue $font 
        }
        $_.font.face = "CaskaydiaCove NF"
        $_.font.size = 12
    }

    ConvertTo-Json $settings -Depth 32 | Out-File -FilePath $settingsPath -Encoding UTF8
}


function Update_PowershellProfile {
    write-host "::: updating PowerShell profile"
    if (-not (Test-Path -Path $profile)) {
        New-Item -ItemType File -Force -Path $profile
    }

    if (-not (Test-Path -Path $env:USERPROFILE\.config\pwsh)) {
        New-Item -ItemType Directory -Force -Path $env:USERPROFILE\.config\pwsh | Out-Null
    }

    Copy-Item -Path .\.config\user_profile.ps1 -Destination $env:USERPROFILE\.config\pwsh\user_profile.ps1 -Force
    Copy-Item -Path .\.config\mytheme.omp.json -Destination $env:USERPROFILE\.config\pwsh\mytheme.omp.json -Force
    Copy-Item -Path .\.config\psreadline.ps1 -Destination $env:USERPROFILE\.config\pwsh\psreadline.ps1 -Force
    Copy-Item -Path .\.config\aliases.ps1 -Destination $env:USERPROFILE\.config\pwsh\aliases.ps1 -Force

    $profileContent = ". $env:USERPROFILE\.config\pwsh\user_profile.ps1"
    $profileContent | Out-File -FilePath $profile -Encoding UTF8

    Execute_Check -Description "checking PS module DockerCompletion is installed" -CheckAction { Get-Module -Name DockerCompletion -ListAvailable } -FixAction { Install-Module -Name DockerCompletion -Repository PSGallery } -ErrorMessage "PS module DockerCompletion needs to be installed"
    Execute_Check -Description "checking PS module z is installed" -CheckAction { Get-Module -Name z -ListAvailable } -FixAction { Install-Module -Name z -AllowClobber } -ErrorMessage "PS module z needs to be installed"

    Write-Host "::: reloading PowerShell profile"
    . $profile
}

function Update_GitConfig {
    Write-Host "::: updating Git config"

    Copy-Item -Path .\.config\git\global.gitconfig -Destination $env:USERPROFILE\.gitconfig -Force
    Copy-Item -Path .\.config\git\aliases.gitconfig -Destination $env:USERPROFILE\.aliases.gitconfig -Force
}

function Check_TerminalSetup {
    $checkTerminal = Read-Host "Do you need to check Windows Terminal setup [y/n]"
    if ($checkTerminal -ne "y" -and $checkTerminal -ne "Y") {
        Return
    }

    Check_IsChocolateyInstalled
    Check_IsGitInstalled

    Check_FontsInstalled
    $chocoPackages = Get-Content -Path ./choco.txt
    $wingetPackages = Get-Content -Path ./winget.txt
    Install_Packages -ChocoPackages $chocoPackages -WinGetPackages $wingetPackages
    Update_WindowsTerminalSettings
    Update_PowershellProfile
    Update_GitConfig
    
    Write-Host "Done!!! (with the terminal at least) ... you may need to restart the terminal for the changes to take effect. It's a Windows Terminal after all ¯\_(ツ)_/¯"
}

Export-ModuleMember -Function Check_TerminalSetup
