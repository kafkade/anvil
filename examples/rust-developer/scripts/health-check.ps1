# health-check.ps1 - Rust Toolchain Health Check
# Verifies the Rust development environment is correctly configured
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

Write-Host "Rust Toolchain Health Check" -ForegroundColor Cyan
Write-Host ("=" * 40) -ForegroundColor Cyan

# Check core tools
Write-Host "`nChecking core tools..." -ForegroundColor Yellow

try {
    $version = (rustc --version 2>&1) -replace 'rustc\s+', ''
    Write-CheckResult "rustc" "OK" $version
} catch {
    Write-CheckResult "rustc" "FAIL" "Not found in PATH"
}

try {
    $version = (cargo --version 2>&1) -replace 'cargo\s+', ''
    Write-CheckResult "cargo" "OK" $version
} catch {
    Write-CheckResult "cargo" "FAIL" "Not found in PATH"
}

try {
    $version = (rustup --version 2>&1) -replace 'rustup\s+', ''
    Write-CheckResult "rustup" "OK" $version
} catch {
    Write-CheckResult "rustup" "FAIL" "Not found in PATH"
}

# Check components
Write-Host "`nChecking components..." -ForegroundColor Yellow

$components = rustup component list --installed 2>&1
foreach ($comp in @("rust-src", "rust-analyzer", "clippy", "rustfmt")) {
    if ($components -match $comp) {
        Write-CheckResult "$comp" "OK"
    } else {
        Write-CheckResult "$comp" "FAIL" "Component not installed"
    }
}

# Check environment
Write-Host "`nChecking environment..." -ForegroundColor Yellow

$cargoPath = Join-Path $env:USERPROFILE ".cargo\bin"
if ($env:PATH -like "*$cargoPath*") {
    Write-CheckResult "PATH contains .cargo/bin" "OK"
} else {
    Write-CheckResult "PATH contains .cargo/bin" "FAIL"
}

Write-Host "`n$("-" * 40)" -ForegroundColor Cyan
if ($script:exitCode -eq 0) {
    Write-Host "All checks passed!" -ForegroundColor Green
} else {
    Write-Host "Some checks failed." -ForegroundColor Red
}

exit $script:exitCode
