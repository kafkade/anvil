### execute this script in an elevated shell
### or via gsudo:
### sudo pwsh check-setup.ps1

Remove-Module *;

Import-Module -Name $PSScriptRoot\modules\Execute_Check.psm1
Import-Module -Name $PSScriptRoot\modules\Get_SystemInfo.psm1
Import-Module -Name $PSScriptRoot\modules\Install_Packages.psm1
Import-Module -Name $PSScriptRoot\modules\PreRequisites.psm1
Import-Module -Name $PSScriptRoot\modules\Workloads.psm1
Import-Module -Name $PSScriptRoot\identity\Check_IdentitySetup.psm1
Import-Module -Name $PSScriptRoot\dev\Check_TerminalSetup.psm1

function Check_IsChocolateyInstalled {
    $desc = "checking if chocolatey is installed"
    $check = { Get-Command choco -ErrorAction SilentlyContinue }
    $fix = { Set-ExecutionPolicy Bypass -Scope Process -Force; iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1')) }
    Execute_Check `
        -Description $desc `
        -CheckAction $check `
        -FixAction $fix `
        -ErrorMessage "Chocolatey must be installed to continue."
}

function Check_IsSpotifyInstalled {
    $desc = "checking if Spotify is installed. I 💓 music!"
    $check = { Get-Command spotify -ErrorAction SilentlyContinue }
    $fix = { choco install spotify --yes }
    Execute_Check -Description $desc -CheckAction $check -FixAction $fix -ErrorMessage "No Spotify?! What are you doing?"
}

function Check_IsGitInstalled {
    $desc = "checking if git is installed"
    $check = { Get-Command git -ErrorAction SilentlyContinue }
    $fix = { Install_ChocolateyPackages }
    Execute_Check -Description $desc -CheckAction $check -FixAction $fix -ErrorMessage "Git and other essential packages are missing. Install them now?"
}

function Check_IsGithubInstalled {
    Execute_Check `
        -Description "checking if gh is installed" `
        -CheckAction { Get-Command gh -ErrorAction SilentlyContinue } `
        -FixAction { Install_ChocolateyPackages } `
        -ErrorMessage "Github CLI and other essential packages are missing. Install them now?"
}

function Check_IsTerraformInstalled {
    Execute_Check `
        -Description "checking if terraform is installed" `
        -CheckAction { Get-Command terraform -ErrorAction SilentlyContinue } `
        -FixAction { Install_ChocolateyPackages } `
        -ErrorMessage "Terraform and other essential packages are missing. Install them now?"
}

function Check_IsAzureCliInstalled {
    Execute_Check `
        -Description "checking if Azure CLI is installed" `
        -CheckAction { Get-Command az -ErrorAction SilentlyContinue } `
        -FixAction { Install_ChocolateyPackages } `
        -ErrorMessage "Azure CLI and other essential packages are missing. Install them now?"
}


Write-Host "`n`n>>> starting bootstrapping script >>>`n"

Get_SystemInfo
Write-Host "----------------------------------------------------------`n"

Check_PreRequisites

Install-WorkloadPackages -WorkloadFolders @(
    # "essentials",
    # "developer",
    # "extras",
    # "frontend",
    "backend",
    "msa-login",
    "account-ux",
    "identity-ux")

# TODO: javierfe: Move some of these check to a workload to keep them organized
# Check_IsGitInstalled
# Check_IsGithubInstalled
# # Check_IsTerraformInstalled
# Check_IsAzureCliInstalled

# Check_TerminalSetup
# Check_IdentitySetup

Write-Host "`n<<< winforge script completed!!! <<<`n"

### TODO: Check node/npm is installed
### TODO: Check typescript is installed (create typescript/react development profile)
### TODO: Create custom shortcut for Windows Terminal in the desktop to open as admin with shortcut
### TODO: Print message to notify user to add shortcut rewrite Ctrl+Alt+T to open Windows Terminal as admin -> Win+T in PowerToys
### TODO: Install git-sim -> pip3 install git-sim (make sure pip3 is installed)
### TODO: Create personal profile for check-setup
### TODO: Add install of hledger to personal profile
