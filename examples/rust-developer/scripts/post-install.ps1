# post-install.ps1 - Rust Development Environment Setup
# Installs Rust components, toolchains, and common cargo tools
#
# Exit codes:
#   0 - Success
#   1 - Failure

$ErrorActionPreference = "Stop"

Write-Host "Setting up Rust development environment..." -ForegroundColor Cyan

# Set default toolchain
Write-Host "  Setting default toolchain to stable..." -ForegroundColor Gray
rustup default stable

# Update stable toolchain
Write-Host "  Updating stable toolchain..." -ForegroundColor Gray
rustup update stable

# Install components
Write-Host "  Installing Rust components..." -ForegroundColor Gray
rustup component add rust-src rust-analyzer clippy rustfmt llvm-tools

# Install common cargo tools
Write-Host "  Installing cargo tools (this may take a few minutes)..." -ForegroundColor Gray
cargo install cargo-watch cargo-nextest cargo-audit

Write-Host "  Rust development environment setup complete!" -ForegroundColor Green
