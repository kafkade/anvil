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
    # TODO: Keep adding checks to satisfy the prerequisites page
    # https://onebranch.visualstudio.com/OneBranch/_wiki/wikis/OneBranch.wiki/3689/Install-Build-Prerequisites

    # TODO: check if IIS is installed
    # TODO: check if IISEXPRESS is installed

    ### Powershell command to enable IIS
    ### Enable-WindowsOptionalFeature -Online -FeatureName IIS-WebServerRole, IIS-WebServer, IIS-CommonHttpFeatures, IIS-ManagementConsole, IIS-HttpErrors, IIS-HttpRedirect, IIS-WindowsAuthentication, IIS-StaticContent, IIS-DefaultDocument, IIS-HttpCompressionStatic, IIS-DirectoryBrowsing
    ### Command to Disable IIS
    ### Disable-WindowsOptionalFeature -Online -FeatureName IIS-WebServerRole, IIS-WebServer

    # TODO: Create Windows terminal profiles
    # https://identitydivision.visualstudio.com/IdentityWiki/_wiki/wikis/IdentityWiki.wiki/26512/-How-to-Set-up-multiple-enlistment-windows-in-a-single-terminal

    
    Check_SlnGenIsInstalled
    Check_AccountUxIsSetup
}

Export-ModuleMember -Function Check_IdentitySetup
