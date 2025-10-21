Import-Module -Name $PSScriptRoot\Check_AdMsaLoginUx.psm1
Import-Module -Name $PSScriptRoot\Check_AdMsaAccountUx.psm1
Import-Module -Name $PSScriptRoot\..\modules\Utilities.psm1

function Check_NugetIsInstalled {
    $nugetUrl = "https://dist.nuget.org/win-x86-commandline/latest/nuget.exe"
    $nugetPath = "C:\tools"
    $nugetExe = "$nugetPath\nuget.exe"

    $fix    = {     
        try {
            if (-not (Test-Path $nugetPath)) {
                New-Item -ItemType Directory -Path $nugetPath | Out-Null
            }
            Invoke-RestMethod -Uri $nugetUrl -OutFile $nugetExe

            if (-not (Test_InPath $nugetPath))
            {
                Add_ToPath $nugetPath
            }
        }
        catch {
            Write-Host "Error: $($_.Exception.Message)"
            exit 1
        }
    }
    Execute_Check `
        -Description "checking if nuget is installed" `
        -CheckAction { Test-Path $nugetExe } `
        -FixAction $fix `
        -ErrorMessage "nuget.exe needs to be installed and accessible in the PATH sys env" `
        -InterruptOnError
}

function Check_IdentitySetup {
    $checkIdentity = Read-Host "Do you need to check Identity setup [y/n]"
    if ($checkIdentity -ne "y" -and $checkIdentity -ne "Y") {
        Return
    }

    Check_NugetIsInstalled

    
    Check_SlnGenIsInstalled
    Check_AccountUxIsSetup
}

Export-ModuleMember -Function Check_IdentitySetup
