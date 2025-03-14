### Get system information

function Get_SystemInfo {
    $adminPasswordStatus = $null
    $thermalState        = $null
    $osInfo              = Get-CimInstance Win32_OperatingSystem
    $computerInfo        = Get-CimInstance Win32_ComputerSystem
    $diskInfo            = Get-CimInstance Win32_LogicalDisk
    
    Switch ($computerInfo.AdminPasswordStatus) {
        0 {$adminPasswordStatus = 'Disabled'}
        1 {$adminPasswordStatus = 'Enabled'}
        2 {$adminPasswordStatus = 'Not Implemented'} 
        3 {$adminPasswordStatus = 'Unknown'}
        Default {$adminPasswordStatus = 'Unable to determine'}
    }
    
    Switch ($computerInfo.ThermalState) {
        1 {$thermalState = 'Other'}
        2 {$thermalState = 'Unknown'}
        3 {$thermalState = 'Safe'}
        4 {$thermalState = 'Warning'} 
        5 {$thermalState = 'Critical'}
        6 {$thermalState = 'Non-recoverable'}
        Default {$thermalState = 'Unable to determine'}
    }
    
    $sysInfo = [PSCustomObject]@{
        ComputerName        = $computerInfo.Name
        OS                  = $osInfo.Caption
        'OS Version'        = $("$($osInfo.Version) Build $($osInfo.BuildNumber)")
        Domain              = $computerInfo.Domain
        Workgroup           = $computerInfo.Workgroup
        DomainJoined        = $computerInfo.PartOfDomain
        Disks               = $diskInfo
        AdminPasswordStatus = $adminPasswordStatus
        ThermalState        = $thermalState
    }

    Return $sysInfo    
}

Export-ModuleMember -Function Get_SystemInfo