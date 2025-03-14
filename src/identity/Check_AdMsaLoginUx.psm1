
Import-Module $PSScriptRoot\..\modules\Execute_Check.psm1

function Check_SlnGenIsInstalled {
    Execute_Check `
        -Description "checking if slngen is installed" `
        -CheckAction { dotnet tool list --global | Where-Object { $_ -match "slngen" } } `
        -FixAction { dotnet tool install --global Microsoft.VisualStudio.SlnGen.Tool } `
        -ErrorMessage "SlnGen is required by the workload." `
        -InterruptOnError
}

Export-ModuleMember -Function Check_SlnGenIsInstalled
