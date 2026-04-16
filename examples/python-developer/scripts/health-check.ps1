# health-check.ps1 - Python Environment Health Check
# Verifies Python and uv are properly installed
#
# Exit codes:
#   0 - All checks passed
#   1 - One or more checks failed

$ErrorActionPreference = "Continue"
$script:exitCode = 0

function Write-CheckResult {
    param(
        [string]$Name,
        [string]$Status,
        [string]$Message
    )
    switch ($Status) {
        "OK"   { Write-Host "  [PASS] $Name" -ForegroundColor Green -NoNewline }
        "FAIL" { Write-Host "  [FAIL] $Name" -ForegroundColor Red -NoNewline; $script:exitCode = 1 }
    }
    if ($Message) { Write-Host " - $Message" -ForegroundColor Gray } else { Write-Host "" }
}

Write-Host "Python Environment Health Check" -ForegroundColor Cyan
Write-Host ("=" * 40) -ForegroundColor Cyan

try {
    $version = (uv --version 2>&1)
    Write-CheckResult "uv" "OK" $version
} catch {
    Write-CheckResult "uv" "FAIL" "Not found in PATH"
}

try {
    $version = (python --version 2>&1)
    Write-CheckResult "python" "OK" $version
} catch {
    Write-CheckResult "python" "FAIL" "Not found in PATH"
}

Write-Host "`n$("-" * 40)" -ForegroundColor Cyan
if ($script:exitCode -eq 0) {
    Write-Host "All checks passed!" -ForegroundColor Green
} else {
    Write-Host "Some checks failed." -ForegroundColor Red
}

exit $script:exitCode
