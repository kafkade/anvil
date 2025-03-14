Import-Module $PSScriptRoot\Execute_Check.psm1

function Check_IsAdmin {
    $desc = "checking if running with admin privileges"
    $check = { ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator") }
    Execute_Check `
        -Description $desc `
        -CheckAction $check `
        -ErrorMessage "This script must be run as Administrator. Open an elevated terminal and try again." `
        -InterruptOnError
}

function Check_IsPowerShell7 {
    $desc = "checking if running in PowerShell 7"
    $check = { $PSVersionTable.PSVersion.Major -eq 7 }
    $fix = { Write-Host "Install PowerShell v7: choco install pwsh" -ForegroundColor Red }
    Execute_Check `
        -Description $desc `
        -CheckAction $check `
        -FixAction $fix `
        -ErrorMessage "This script must be run in PowerShell 7 to prevent compatibility issues."
}

function Check_IsWindowsTerminal {
    $desc = "checking if running in Windows Terminal"
    $check = { $env:WT_SESSION }
    Execute_Check `
        -Description $desc `
        -CheckAction $check `
        -ErrorMessage "This script is recommended to be run in Windows Terminal"
}

function Check_PowerShellYAMLInstalled {
    Execute_Check `
        -Description "checking `powershell-yaml` is installed" `
        -CheckAction {Get-Module -ListAvailable -Name powershell-yaml} `
        -FixAction {Install-Module powershell-yaml -Scope CurrentUser -Force} `
        -ErrorMessage "powershell-yaml module needs to be installed. Install it now?" `
        -InterruptOnError
}

function Check_PreRequisites {
    Check_IsAdmin
    Check_IsPowerShell7
    Check_IsWindowsTerminal

    # The powershell-yaml module is used in the script to parse YAML files.
    Check_PowerShellYAMLInstalled
}

Export-ModuleMember -Function Check_PreRequisites