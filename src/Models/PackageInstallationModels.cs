using System;
using System.Collections.Generic;
using System.Linq;

namespace Winforge.Models;

/// <summary>
/// Represents the result of a single package installation.
/// </summary>
public class PackageInstallationResult
{
    /// <summary>
    /// Gets or sets a value indicating whether the installation was successful.
    /// </summary>
    public bool Success { get; set; }

    /// <summary>
    /// Gets or sets the package that was installed.
    /// </summary>
    public PackageInfo Package { get; set; } = new();

    /// <summary>
    /// Gets or sets the installation status.
    /// </summary>
    public PackageInstallationStatus Status { get; set; }

    /// <summary>
    /// Gets or sets the error category if the installation failed.
    /// </summary>
    public PackageInstallationErrorCategory ErrorCategory { get; set; }

    /// <summary>
    /// Gets or sets the exit code from the package manager.
    /// </summary>
    public int ExitCode { get; set; }

    /// <summary>
    /// Gets or sets the standard output from the installation process.
    /// </summary>
    public List<string> StandardOutput { get; set; } = new();

    /// <summary>
    /// Gets or sets the standard error from the installation process.
    /// </summary>
    public List<string> StandardError { get; set; } = new();

    /// <summary>
    /// Gets or sets error messages.
    /// </summary>
    public List<string> Errors { get; set; } = new();

    /// <summary>
    /// Gets or sets warning messages.
    /// </summary>
    public List<string> Warnings { get; set; } = new();

    /// <summary>
    /// Gets or sets the version that was installed.
    /// </summary>
    public string? InstalledVersion { get; set; }

    /// <summary>
    /// Gets or sets the start time of the installation.
    /// </summary>
    public DateTime StartTime { get; set; }

    /// <summary>
    /// Gets or sets the end time of the installation.
    /// </summary>
    public DateTime EndTime { get; set; }

    /// <summary>
    /// Gets or sets the duration of the installation.
    /// </summary>
    public TimeSpan Duration { get; set; }

    /// <summary>
    /// Gets or sets the command that was executed.
    /// </summary>
    public string CommandExecuted { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets a value indicating whether the package was already installed.
    /// </summary>
    public bool AlreadyInstalled { get; set; }

    /// <summary>
    /// Gets or sets a value indicating whether a reboot is required.
    /// </summary>
    public bool RebootRequired { get; set; }
}

/// <summary>
/// Represents the result of installing multiple packages.
/// </summary>
public class BatchInstallationResult
{
    /// <summary>
    /// Gets or sets the individual package results.
    /// </summary>
    public List<PackageInstallationResult> Results { get; set; } = new();

    /// <summary>
    /// Gets or sets the start time of the batch installation.
    /// </summary>
    public DateTime StartTime { get; set; }

    /// <summary>
    /// Gets or sets the end time of the batch installation.
    /// </summary>
    public DateTime EndTime { get; set; }

    /// <summary>
    /// Gets or sets the total duration of all installations.
    /// </summary>
    public TimeSpan TotalDuration { get; set; }

    /// <summary>
    /// Gets the total number of packages processed.
    /// </summary>
    public int TotalPackages => Results.Count;

    /// <summary>
    /// Gets the number of successful installations.
    /// </summary>
    public int SuccessCount => Results.Count(r => r.Success);

    /// <summary>
    /// Gets the number of failed installations.
    /// </summary>
    public int FailedCount => Results.Count(r => !r.Success && r.Status == PackageInstallationStatus.Failed);

    /// <summary>
    /// Gets the number of packages that were already installed.
    /// </summary>
    public int AlreadyInstalledCount => Results.Count(r => r.AlreadyInstalled);

    /// <summary>
    /// Gets the number of packages that were skipped.
    /// </summary>
    public int SkippedCount => Results.Count(r => r.Status == PackageInstallationStatus.Skipped);

    /// <summary>
    /// Gets a value indicating whether all packages were installed successfully.
    /// </summary>
    public bool AllSucceeded => Results.All(r => r.Success || r.AlreadyInstalled);

    /// <summary>
    /// Gets a value indicating whether any package installation failed.
    /// </summary>
    public bool HasFailures => Results.Any(r => r.Status == PackageInstallationStatus.Failed);

    /// <summary>
    /// Gets a value indicating whether any package requires a reboot.
    /// </summary>
    public bool RebootRequired => Results.Any(r => r.RebootRequired);

    /// <summary>
    /// Gets the overall success rate as a percentage.
    /// </summary>
    public double SuccessRate => TotalPackages > 0
        ? (double)(SuccessCount + AlreadyInstalledCount) / TotalPackages * 100
        : 100;
}

/// <summary>
/// Progress information for a single package installation.
/// </summary>
public class PackageInstallationProgress
{
    /// <summary>
    /// Gets or sets the package being installed.
    /// </summary>
    public PackageInfo Package { get; set; } = new();

    /// <summary>
    /// Gets or sets the current status.
    /// </summary>
    public PackageInstallationStatus Status { get; set; }

    /// <summary>
    /// Gets or sets the current status message.
    /// </summary>
    public string Message { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the percentage complete (0-100), if known.
    /// </summary>
    public int? PercentComplete { get; set; }
}

/// <summary>
/// Progress information for batch package installation.
/// </summary>
public class BatchInstallationProgress
{
    /// <summary>
    /// Gets or sets the current package index (1-based).
    /// </summary>
    public int CurrentPackageIndex { get; set; }

    /// <summary>
    /// Gets or sets the total number of packages.
    /// </summary>
    public int TotalPackages { get; set; }

    /// <summary>
    /// Gets or sets the current package being processed.
    /// </summary>
    public PackageInfo CurrentPackage { get; set; } = new();

    /// <summary>
    /// Gets or sets the current package status.
    /// </summary>
    public PackageInstallationStatus CurrentStatus { get; set; }

    /// <summary>
    /// Gets or sets the current status message.
    /// </summary>
    public string Message { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the number of successful installations so far.
    /// </summary>
    public int SuccessCount { get; set; }

    /// <summary>
    /// Gets or sets the number of failed installations so far.
    /// </summary>
    public int FailedCount { get; set; }

    /// <summary>
    /// Gets the overall progress percentage.
    /// </summary>
    public double OverallPercentage => TotalPackages > 0
        ? (double)CurrentPackageIndex / TotalPackages * 100
        : 0;
}

/// <summary>
/// Represents the status of a package installation.
/// </summary>
public enum PackageInstallationStatus
{
    /// <summary>Package installation is pending.</summary>
    Pending,

    /// <summary>Package installation is in progress.</summary>
    Installing,

    /// <summary>Package installation completed successfully.</summary>
    Succeeded,

    /// <summary>Package installation failed.</summary>
    Failed,

    /// <summary>Package was already installed.</summary>
    AlreadyInstalled,

    /// <summary>Package installation was cancelled.</summary>
    Cancelled,

    /// <summary>Package installation was skipped.</summary>
    Skipped,

    /// <summary>Package was not found in the repository.</summary>
    NotFound,

    /// <summary>Package installation requires a reboot.</summary>
    RebootRequired,
    
    /// <summary>Package installation timed out.</summary>
    TimedOut
}

/// <summary>
/// Represents the category of error that occurred during installation.
/// </summary>
public enum PackageInstallationErrorCategory
{
    /// <summary>No error occurred.</summary>
    None,

    /// <summary>The operation timed out.</summary>
    Timeout,

    /// <summary>Permission was denied.</summary>
    Permission,

    /// <summary>A network error occurred.</summary>
    Network,

    /// <summary>The package was not found.</summary>
    PackageNotFound,

    /// <summary>A dependency error occurred.</summary>
    DependencyError,

    /// <summary>An unknown error occurred.</summary>
    Unknown
}