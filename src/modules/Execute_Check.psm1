function Execute_Check {
	Param(
        [Parameter()]
        [string] $Description,
        [Parameter()]
        [scriptblock]$CheckAction,
        [Parameter()]
        [string]$ErrorMessage,
        [Parameter()]
        [scriptblock]$FixAction,
        [Parameter()]
        [switch] $InterruptOnError
	)
    Write-Host "::: $Description".PadRight(80, " ") -NoNewLine

    $isSuccess = & $CheckAction

    if ($isSuccess) {
        Write-Host "[ $true ]" -ForegroundColor Green
    }
    else {
        Write-Host "[ $false ]" -ForegroundColor Red
        Write-Host ">>> $ErrorMessage" -ForegroundColor Red
        if ($FixAction) {
            $fixResponse = Read-Host "Do you want to fix this? [y/n]"
            if ($fixResponse -eq "y" -or $fixResponse -eq "Y") {
                & $FixAction
                Return
            }
        }
    }

    if (-not $isSuccess -and $InterruptOnError.IsPresent) { Exit 1 }
}

Export-ModuleMember -Function Execute_Check
