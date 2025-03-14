function Install_DevelopmentWorkload {
    # Install applications via winget
    winget install --id Microsoft.VisualStudioCode -e --source winget
    winget install --id Git.Git -e --source winget
    winget install --id NodeJS.NodeJS -e --source winget

    # Set environment variables
    # [System.Environment]::SetEnvironmentVariable("NODE_ENV", "development", [System.EnvironmentVariableTarget]::Machine)

    # Create folders
    # New-Item -ItemType Directory -Path "C:\Dev\Projects"

    # Download files
    # Invoke-WebRequest -Uri "https://example.com/somefile.zip" -OutFile "C:\Dev\Projects\somefile.zip"

    Write-Host "Frontend workload installed successfully."
}