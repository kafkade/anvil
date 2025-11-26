using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Winforge.Interfaces;
using Winforge.Models;
using Winforge.Services.Logging;

namespace Winforge.Services.Execution;

/// <summary>
/// Orchestrates package installation across multiple package managers.
/// </summary>
public class PackageInstallationOrchestrator
{
    private readonly WingetPackageInstaller _wingetInstaller;
    private readonly StructuredLogger _logger;
    private readonly List<PackageInstallationResult> _bufferedResults = new();

    /// <summary>
    /// Initializes a new instance of the PackageInstallationOrchestrator class.
    /// </summary>
    /// <param name="wingetInstaller">The winget installer service</param>
    /// <param name="logger">The structured logger</param>
    public PackageInstallationOrchestrator(
        WingetPackageInstaller wingetInstaller,
        StructuredLogger logger)
    {
        _wingetInstaller = wingetInstaller ?? throw new ArgumentNullException(nameof(wingetInstaller));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Installs all packages from the consolidated list, handling different package managers.
    /// </summary>
    /// <param name="packageList">The consolidated list of packages to install</param>
    /// <param name="progress">Optional progress reporter</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>A summary of the installation results</returns>
    public async Task<InstallationSummary> InstallAllPackagesAsync(
        ConsolidatedPackageList packageList,
        IProgress<InstallationProgressReport>? progress,
        CancellationToken cancellationToken)
    {
        if (packageList == null)
            throw new ArgumentNullException(nameof(packageList));

        var startTime = DateTime.Now;
        var resultsByManager = new Dictionary<string, ManagerInstallationResult>();
        
        int totalPackages = packageList.PackagesByManager.Sum(kvp => kvp.Value.Count);
        int completedPackages = 0;
        int totalSuccess = 0;
        int totalFailed = 0;
        int totalSkipped = 0;

        // _logger.LogInformation("Starting orchestration of {TotalPackages} packages across {ManagerCount} managers",
        //     totalPackages, packageList.PackagesByManager.Count);

        // Define execution order: winget, choco, npm, pip, then others alphabetically
        var orderedManagers = packageList.PackagesByManager.Keys
            .OrderBy(m => m switch
            {
                "winget" => 0,
                "choco" => 1,
                "npm" => 2,
                "pip" => 3,
                _ => 4
            })
            .ThenBy(m => m)
            .ToList();

        foreach (var managerName in orderedManagers)
        {
            if (cancellationToken.IsCancellationRequested)
                break;

            var packages = packageList.PackagesByManager[managerName]
                .OrderBy(p => p.Name)
                .ToList();

            if (!packages.Any())
                continue;

            // _logger.LogInformation("Processing {Count} packages for manager: {Manager}", packages.Count, managerName);

            // Select the appropriate installer
            IPackageInstaller installer;
            bool isSimulated = false;

            if (string.Equals(managerName, "winget", StringComparison.OrdinalIgnoreCase))
            {
                installer = _wingetInstaller;
            }
            else
            {
                installer = new SimulatedPackageInstaller(managerName);
                isSimulated = true;
            }

            var failureDetails = new List<PackageFailureDetail>();
            var managerResult = new ManagerInstallationResult(
                Manager: managerName,
                PackageCount: packages.Count,
                SuccessCount: 0,
                FailureCount: 0,
                FailedPackages: new List<string>(),
                FailureDetails: failureDetails,
                WasSimulated: isSimulated
            );

            foreach (var package in packages)
            {
                if (cancellationToken.IsCancellationRequested)
                    break;

                // Convert ConsolidatedPackage to PackageInfo
                var packageInfo = new PackageInfo
                {
                    name = package.Name,
                    version = package.Version,
                    // Map other properties if needed
                };

                progress?.Report(new InstallationProgressReport(
                    CurrentManager: managerName,
                    CurrentPackage: package.Name,
                    CompletedPackages: completedPackages,
                    TotalPackages: totalPackages,
                    SuccessCount: totalSuccess,
                    FailureCount: totalFailed,
                    StatusMessage: $"Installing {package.Name} via {managerName}..."
                ));

                try
                {
                    // _logger.LogInstallationStart(package.Name, managerName, package.Version);

                    var result = await installer.InstallPackageAsync(packageInfo, null, cancellationToken);
                    _bufferedResults.Add(result);

                    if (result.Success || result.AlreadyInstalled)
                    {
                        managerResult = managerResult with { SuccessCount = managerResult.SuccessCount + 1 };
                        totalSuccess++;
                        // _logger.LogInstallationComplete(package.Name, true, result.Duration);
                    }
                    else
                    {
                        managerResult = managerResult with { FailureCount = managerResult.FailureCount + 1 };
                        managerResult.FailedPackages.Add(package.Name);
                        
                        var failureDetail = CreateFailureDetail(package, result, managerName);
                        failureDetails.Add(failureDetail);

                        totalFailed++;
                        // _logger.LogInstallationComplete(package.Name, false, result.Duration, string.Join("; ", result.Errors));
                    }
                }
                catch (Exception ex)
                {
                    managerResult = managerResult with { FailureCount = managerResult.FailureCount + 1 };
                    managerResult.FailedPackages.Add(package.Name);
                    
                    var errorResult = new PackageInstallationResult
                    {
                        Success = false,
                        Status = PackageInstallationStatus.Failed,
                        ErrorCategory = PackageInstallationErrorCategory.Unknown,
                        Errors = new List<string> { ex.Message },
                        Package = packageInfo
                    };
                    var failureDetail = CreateFailureDetail(package, errorResult, managerName);
                    failureDetails.Add(failureDetail);

                    totalFailed++;
                    // _logger.LogError(ex, $"Failed to install {package.Name}");
                }

                completedPackages++;
                
                progress?.Report(new InstallationProgressReport(
                    CurrentManager: managerName,
                    CurrentPackage: package.Name,
                    CompletedPackages: completedPackages,
                    TotalPackages: totalPackages,
                    SuccessCount: totalSuccess,
                    FailureCount: totalFailed,
                    StatusMessage: null
                ));
            }

            resultsByManager[managerName] = managerResult;
        }

        var summary = new InstallationSummary(
            TotalPackages: totalPackages,
            SuccessfulInstalls: totalSuccess,
            FailedInstalls: totalFailed,
            SkippedInstalls: totalSkipped,
            TotalDuration: DateTime.Now - startTime,
            ResultsByManager: resultsByManager
        );

        _logger.LogDebug("Installation orchestration completed. Success: {Success}, Failed: {Failed}",
            totalSuccess, totalFailed);

        return summary;
    }

    private PackageFailureDetail CreateFailureDetail(
        ConsolidatedPackage package,
        PackageInstallationResult result,
        string managerName)
    {
        return new PackageFailureDetail(
            PackageName: package.Name,
            PackageManager: managerName,
            Version: package.Version,
            Status: result.Status,
            ErrorCategory: result.ErrorCategory,
            ExitCode: result.ExitCode,
            Errors: result.Errors,
            StandardOutput: result.StandardOutput,
            StandardError: result.StandardError,
            Duration: result.Duration,
            CommandExecuted: result.CommandExecuted
        );
    }

    private InstallationFailureReport BuildFailureReport(List<PackageFailureDetail> failureDetails)
    {
        return new InstallationFailureReport
        {
            TotalFailures = failureDetails.Count,
            Failures = failureDetails
        };
    }
}