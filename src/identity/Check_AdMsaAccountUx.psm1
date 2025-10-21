### https://identitydivision.visualstudio.com/IdentityWiki/_wiki/wikis/IdentityWiki.wiki/34102/AD-MSA-AccountUX

Import-Module $PSScriptRoot\..\modules\Execute_Check.psm1

function AccountUxSetup {

    $response = Read-Host "Do you need to setup AD-MSA-AccountUX [y/n] "
    if ($response -ne "y" -and $response -ne "Y") { Return }
    $response = Read-Host "This will clone the repo and enlist the project. Do you want to continue [y/n] "
    if ($response -ne "y" -and $response -ne "Y") { Return }

    $nugetRoot = "C:\CxCache"
    $nugetRootResponse = Read-Host "Nuget install root (Default: $nugetRoot) "
    if ($nugetRootResponse) {
        $nugetRoot = $nugetRootResponse
    }

    Set-Variable NugetMachineInstallRoot=$nugetRoot
    Write-Host ">>> nuget root set to $nugetRoot"

    Start-Process "\\reddog\public\build\bootstrap\install.cmd" -NoNewWindow -Wait
    Set-ExecutionPolicy unrestricted

    $repo = "C:\code\"
    $oldDir = Get-Location

    try {
        Set-Location $repo
        Write-Host ">>> switch to repo location: $repo"
        git clone https://msazure.visualstudio.com/One/_git/AD-MSA-AccountUX
        Set-Location AD-MSA-AccountUX
        git pull

        $cmd = "C:\WINDOWS\system32\cmd.exe"
        $cmdArgs = "/k $repo\AD-MSA-AccountUX\init.cmd"
        $shortcutPath = "$HOME\Desktop\AD-MSA-AccountUX.lnk"
        $WScriptObj = New-Object -ComObject ("WScript.Shell")
        $shortcut = $WscriptObj.CreateShortcut($shortcutPath)
        $shortcut.TargetPath = $cmd
        $shortcut.Arguments = $cmdArgs
        $shortcut.WorkingDirectory = "$repo\AD-MSA-AccountUX"
        $shortcut.WindowStyle = 1
        $shortcut.Save()

        # Set the "Run as Administrator" flag on the shortcut
        $bytes = [System.IO.File]::ReadAllBytes($shortcutPath)
        $bytes[0x15] = $bytes[0x15] -bor 0x20 #set byte 21 (0x15) bit 6 (0x20) ON
        [System.IO.File]::WriteAllBytes($shortcutPath, $bytes)
    }
    catch {
        Write-Host "Error: $($_.Exception.Message)"
        exit 1
    }
    finally {
        Set-Location $oldDir
    }
}

function Check_AccountUxIsSetup {
    $desc   = "checking if AD-MSA-AccountUX is setup"
    $check  = { choco search BuildReq -l | Where-Object { $_ -match "BuildReq" } }
    $fix = { AccountUxSetup }
    Execute_Check -Description $desc -CheckAction $check -FixAction $fix -ErrorMessage "AD-MSA-AccountUX needs to be setup." -InterruptOnError
}

Export-ModuleMember -Function Check_AccountUxIsSetup
