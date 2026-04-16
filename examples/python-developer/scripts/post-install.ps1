# post-install.ps1 - Python Environment Setup
# Installs Python via uv and configures the environment
#
# Exit codes:
#   0 - Success
#   1 - Failure

$ErrorActionPreference = "Stop"

Write-Host "Setting up Python development environment..." -ForegroundColor Cyan

# Install latest Python via uv
Write-Host "  Installing latest Python via uv..." -ForegroundColor Gray
uv python install

Write-Host "  Python development environment setup complete!" -ForegroundColor Green
