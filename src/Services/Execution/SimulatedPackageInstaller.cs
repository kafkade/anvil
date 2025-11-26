using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Simulates package installation for non-winget managers.
/// </summary>
public class SimulatedPackageInstaller : IPackageInstaller
{
    private readonly string _packageManager;

    /// <summary>
    /// Gets the package manager identifier this installer handles.
    /// </summary>
    public string PackageManager => _packageManager;

    /// <summary>
    /// Initializes a new instance of the SimulatedPackageInstaller class.
    /// </summary>
    /// <param name="packageManager">The package manager to simulate (e.g., "choco", "npm")</param>
    public SimulatedPackageInstaller(string packageManager)
    {
        if (string.IsNullOrWhiteSpace(packageManager))
            throw new ArgumentException("Package manager name cannot be empty", nameof(packageManager));
            
        _packageManager = packageManager;
    }

    /// <summary>
    /// Simulates installing a single package.
    /// </summary>
    public async Task<PackageInstallationResult> InstallPackageAsync(
        PackageInfo package,
        IProgress<PackageInstallationProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        if (package == null)
            throw new ArgumentNullException(nameof(package));

        var result = new PackageInstallationResult
        {
            Package = package,
            StartTime = DateTime.Now,
            Status = PackageInstallationStatus.Installing
        };

        try
        {
            progress?.Report(new PackageInstallationProgress
            {
                Package = package,
                Status = PackageInstallationStatus.Installing,
                Message = $"Simulating installation of {package.name}..."
            });

            // Simulate work
            await Task.Delay(5000, cancellationToken);

            result.Success = true;
            result.Status = PackageInstallationStatus.Succeeded;
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
            result.CommandExecuted = $"[Simulated] {_packageManager} install {package.name}";
            result.InstalledVersion = package.version ?? "simulated-latest";
            
            progress?.Report(new PackageInstallationProgress
            {
                Package = package,
                Status = PackageInstallationStatus.Succeeded,
                Message = $"Simulated installation of {package.name}"
            });
        }
        catch (OperationCanceledException)
        {
            result.Status = PackageInstallationStatus.Cancelled;
            result.Success = false;
            result.Errors.Add("Installation was cancelled");
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
        }
        catch (Exception ex)
        {
            result.Status = PackageInstallationStatus.Failed;
            result.Success = false;
            result.Errors.Add($"Simulation error: {ex.Message}");
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
        }

        return result;
    }

    /// <summary>
    /// Simulates installing multiple packages sequentially.
    /// </summary>
    public async Task<BatchInstallationResult> InstallPackagesAsync(
        IEnumerable<PackageInfo> packages,
        IProgress<BatchInstallationProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        if (packages == null)
            throw new ArgumentNullException(nameof(packages));

        var packageList = packages.ToList();
        var batchResult = new BatchInstallationResult
        {
            StartTime = DateTime.Now
        };

        for (int i = 0; i < packageList.Count; i++)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                break;
            }

            var package = packageList[i];

            progress?.Report(new BatchInstallationProgress
            {
                CurrentPackageIndex = i + 1,
                TotalPackages = packageList.Count,
                CurrentPackage = package,
                CurrentStatus = PackageInstallationStatus.Installing,
                Message = $"Simulating installation of {package.name} ({i + 1}/{packageList.Count})...",
                SuccessCount = batchResult.SuccessCount,
                FailedCount = batchResult.FailedCount
            });

            var result = await InstallPackageAsync(package, null, cancellationToken);
            batchResult.Results.Add(result);

            progress?.Report(new BatchInstallationProgress
            {
                CurrentPackageIndex = i + 1,
                TotalPackages = packageList.Count,
                CurrentPackage = package,
                CurrentStatus = result.Status,
                Message = result.Success
                    ? $"Simulated installation of {package.name}"
                    : $"Failed simulation: {package.name}",
                SuccessCount = batchResult.SuccessCount,
                FailedCount = batchResult.FailedCount
            });
        }

        batchResult.EndTime = DateTime.Now;
        batchResult.TotalDuration = batchResult.EndTime - batchResult.StartTime;

        return batchResult;
    }

    /// <summary>
    /// Always returns false for simulated packages.
    /// </summary>
    public Task<bool> IsPackageInstalledAsync(
        PackageInfo package,
        CancellationToken cancellationToken = default)
    {
        return Task.FromResult(false);
    }

    /// <summary>
    /// Always returns true for the simulator.
    /// </summary>
    public Task<bool> IsAvailableAsync()
    {
        return Task.FromResult(true);
    }

    /// <summary>
    /// Returns a fixed version string.
    /// </summary>
    public Task<string?> GetVersionAsync()
    {
        return Task.FromResult<string?>("simulated");
    }
}