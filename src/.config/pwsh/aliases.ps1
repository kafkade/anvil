### Functions
function New-Link ($target, $link) {
    gsudo New-Item -Path $link -ItemType SymbolicLink -Value $target
}

function Remove-StoppedContainers {  
    docker container rm $(docker container ls -q)
}

function Remove-AllContainers {  
    docker container rm -f $(docker container ls -aq)
}

function Get-ContainerIPAddress {  
    param (
        [string] $id
    )
    & docker inspect --format '{{ .NetworkSettings.Networks.nat.IPAddress }}' $id
}

function Get-Skopeo {
    <#
        .DESCRIPTION
        Get detail information from docker image.
        
        .EXAMPLE
        skopeo inspect --raw docker://alpine
    #>
    docker run --rm luebken/skopeo skopeo $args
}

function Get-Calendar($year=(Get-Date).Year,$month=(Get-Date).Month){
    $dtfi = New-Object System.Globalization.DateTimeFormatInfo
    $AbbreviatedDayNames=$dtfi.AbbreviatedDayNames | ForEach-Object {" {0}" -f $_.Substring(0,2)}

    $header= "$($dtfi.MonthNames[$month-1]) $year"
    $header=(" "*([math]::abs(21-$header.length) / 2))+$header
    $header+=(" "*(21-$header.length))

    Write-Host $header -BackgroundColor yellow -ForegroundColor black
    Write-Host (-join $AbbreviatedDayNames) -BackgroundColor cyan -ForegroundColor black
    $daysInMonth=[DateTime]::DaysInMonth($year,$month)

    $dayOfWeek =(New-Object DateTime $year,$month,1).dayOfWeek.value__
    $today=(Get-Date).Day

    for ($i = 0; $i -lt $dayOfWeek; $i++){Write-Host (" "*3) -NoNewline}
    for ($i = 1; $i -le $daysInMonth; $i++)
    {
        if($today -eq $i){Write-Host ("{0,3}" -f $i) -NoNewline -BackgroundColor red -ForegroundColor white}
        else {Write-Host ("{0,3}" -f $i) -NoNewline -BackgroundColor white -ForegroundColor black}

        if ($dayOfWeek -eq 6) {Write-Host}
        $dayOfWeek = ($dayOfWeek + 1) % 7
    }
    if ($dayOfWeek -ne 0) {Write-Host}
}


### Alias
Set-Alias d docker
Set-Alias drm Remove-StoppedContainers 
Set-Alias drmf Remove-AllContainers 
Set-Alias dip  Get-ContainerIPAddress  
Set-Alias skopeo Get-Skopeo
Set-Alias k kubectl
Set-Alias g git
Set-Alias ll ls
Set-Alias vim nvim
Set-Alias ln New-Link
Set-Alias which Get-Command
Set-Alias -Name kusto -Value C:\tools\kusto\net6.0\Kusto.Cli.exe
Set-Alias c code
Set-Alias ci code-insiders

function lsa() {
    eza -1l --icons=always --color=always --color-scale=all --color-scale-mode=gradient --group-directories-first --show-symlinks @args
}
Set-Alias -Name ls -Value lsa
Set-Alias e eza

## terraform
Set-Alias t terraform
function Get-TerraformStateList { t state list }
Set-Alias tl Get-TerraformStateList
function Get-TerraformStateShow { t state show $args }
Set-Alias ts Get-TerraformStateShow
function Invoke-TerraformPlan { t plan }
Set-Alias tp Invoke-TerraformPlan
function Invoke-TerraformApply { t apply $args }
Set-Alias ta Invoke-TerraformApply
function Invoke-TerraformApplyAutoApprove { t apply -auto-approve }
Set-Alias taaa Invoke-TerraformApplyAutoApprove
function Invoke-TerraformApplyRefreshOnly { t apply -refresh-only }
Set-Alias taro Invoke-TerraformApplyRefreshOnly
function Invoke-TerraformDestroy { t apply -destroy }
Set-Alias td Invoke-TerraformDestroy
function Invoke-TerraformConsole { t console }
Set-Alias tc Invoke-TerraformConsole

## GICO functions

function Invoke-SlnGen { slngen --folders:true --platform x64 }
Set-Alias sg Invoke-SlnGen

<#
.SYNOPSIS
Connects to the Microsoft AzVPN.

.DESCRIPTION
MSA-Login workflow requires a secure connection to MSA backend and other
resources. Connect-VPN establishes a connection to the Microsoft Azure VPN.

This function can be used to connect to any VPN, but for the moment is hardcoded
to this specific VPN.

.EXAMPLE
Connect-VPN

.NOTES
There is already a MSFT-AzVPN-Auto profile that connects automatically when the machine
is started. This function is useful when the auto connection fails for some reason.
#>
function Connect-VPN {
    $vpn = "MSFT-AzVPN-Manual"
    $vpnStatus = Get-VpnConnection -Name $vpn
    if ($vpnStatus.ConnectionStatus -eq "Connected") {
        Write-Host "::: VPN is already connected."
    } else {
        Write-Host "::: Connecting to VPN..."
        rasdial $vpn
    }
}

<#
.SYNOPSIS
Sets up the environment to work on MSA-Login tasks.

.DESCRIPTION
MSA-Login workflow requires a secure connection to MSA backend and other
resources. Often developers will need to access the E directory in the
domainless VM; this command will also map that remote directory to the O: drive
in this workstation.

.EXAMPLE
Start-MsaLogin

.NOTES
This function will need refactoring since OneBox is moving to a DevBox model.
#>
function Start-MsaLogin {

    # Connect to VPN
    Connect-VPN

    # Get OneBox info from 1P
    #$item = op item get "Domainless VM OneBox" --vault "Microsoft" --format json

    # $global:onebox = [PSCustomObject]@{
    #     IPAddress   = $itemObject.fields | Where-Object { $_.label -eq "ip address" } | Select-Object -ExpandProperty value
    #     MachineName = $itemObject.fields | Where-Object { $_.label -eq "machine name" } | Select-Object -ExpandProperty value
    #     Username    = $itemObject.fields | Where-Object { $_.label -eq "username" } | Select-Object -ExpandProperty value
    #     Password    = $itemObject.fields | Where-Object { $_.label -eq "password" } | Select-Object -ExpandProperty value
    # }

    Write-Host "::: Getting OneBox info from vault"
    $global:onebox = [PSCustomObject]@{
        IPAddress   = $(op read "op://Microsoft/Domainless VM OneBox/ip address")
        MachineName = $(op read "op://Microsoft/Domainless VM OneBox/machine name")
        Username    = $(op read "op://Microsoft/Domainless VM OneBox/username")
        Password    = $(op read "op://Microsoft/Domainless VM OneBox/password")
    }

    # $User = "Domain01\User01"
    # $PWord = Read-Host -Prompt 'Enter a Password' -AsSecureString
    # $credentialParams = @{
    #     TypeName = 'System.Management.Automation.PSCredential'
    #     ArgumentList = $global:onebox.Username, $global:onebox.Password
    # }
    # $Credential = New-Object @credentialParams

    # $cred = Get-Credential -Credential $global.onebox.username

    # $parameters = @{
    #     Name = "OneBox"
    #     PSProvider = "FileSystem"
    #     Root = "\\{0}\e$" -f $global:onebox.IPAddress
    #     Description = "Maps to E drive in my OneBox."
    #     Credential = $cred
    # }
    # New-PSDrive @parameters

    # New-PSDrive -Name "O" -PSProvider "FileSystem" -Root "\\10.60.5.105\e$" -Scope Global -Credential $cred -Persist
}


function Invoke-AxisBumpVersion {
    $Local:gitStatus = git status --porcelain
    if ($Local:gitStatus) {
        Write-Host "Git repository is not clean. Either commit or stash your changes before bumping the version."
        return
    }
    Write-Host "::: setting execution policy ..."
    Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass

    Write-Host "::: bumping versions ..."
    yarn bump-versions
}
Set-Alias bump Invoke-AxisBumpVersion
function Invoke-AxisWorkspaceTest {
    Write-Host "::: running current workspace test deck ..."

    yarn test-unit signup-config.test signup-reducer.test
}
Set-Alias testws Invoke-AxisWorkspaceTest

<#
.SYNOPSIS
Formats a GUID in the clipboard.

.DESCRIPTION
Takes the content of the clipboard, and if it is a GUID, formats it and copies the formatted version onto the clipboard.

.EXAMPLE
Format-Guid

.NOTES
General notes
#>
function Format-Guid {
    $clip = Get-Clipboard

    try {
        Write-Host "Parsing guid from clipboard:" $clip
        $guid = [System.Guid]::Parse($clip)
        $guid = $guid.ToString("D")
        Set-Clipboard $guid
        Write-Host "Guid:" $guid "was copied to clipboard"
    }
    catch {
        Write-Host "There was a problem formatting the guid in the clipboard. Make sure the content in the clipboard is a valid guid."
        $guid = $null
    }
}
