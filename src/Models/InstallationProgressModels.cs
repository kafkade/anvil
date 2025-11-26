using System;
using System.Collections.Generic;

namespace Winforge.Models;

/// <summary>
/// Progress report for package installation
/// </summary>
public record InstallationProgressReport(
    string CurrentManager,
    string CurrentPackage,
    int CompletedPackages,
    int TotalPackages,
    int SuccessCount,
    int FailureCount,
    string? StatusMessage = null
);

/// <summary>
/// Summary of installation results
/// </summary>
public record InstallationSummary(
    int TotalPackages,
    int SuccessfulInstalls,
    int FailedInstalls,
    int SkippedInstalls,
    TimeSpan TotalDuration,
    Dictionary<string, ManagerInstallationResult> ResultsByManager
);

/// <summary>
/// Installation result for a specific package manager
/// </summary>
public record ManagerInstallationResult(
    string Manager,
    int PackageCount,
    int SuccessCount,
    int FailureCount,
    List<string> FailedPackages,
    List<PackageFailureDetail> FailureDetails,
    bool WasSimulated
);

/// <summary>
/// Detailed information about a failed package installation
/// </summary>
public record PackageFailureDetail(
    string PackageName,
    string PackageManager,
    string? Version,
    PackageInstallationStatus Status,
    PackageInstallationErrorCategory ErrorCategory,
    int ExitCode,
    List<string> Errors,
    List<string> StandardOutput,
    List<string> StandardError,
    TimeSpan Duration,
    string CommandExecuted
);

/// <summary>
/// Comprehensive failure report for display after installation
/// </summary>
public class InstallationFailureReport
{
    public int TotalFailures { get; set; }
    public List<PackageFailureDetail> Failures { get; set; } = new();
    public bool HasCriticalFailures => Failures.Any(f =>
        f.ErrorCategory == PackageInstallationErrorCategory.Permission ||
        f.ErrorCategory == PackageInstallationErrorCategory.DependencyError);
    public bool RequiresReboot => Failures.Any(f =>
        f.Status == PackageInstallationStatus.RebootRequired);
}