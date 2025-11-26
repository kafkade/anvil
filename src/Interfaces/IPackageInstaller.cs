using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Winforge.Models;

namespace Winforge.Interfaces;

/// <summary>
/// Interface for package installation services.
/// Provides package installation capabilities with progress reporting and error handling.
/// </summary>
public interface IPackageInstaller
{
    /// <summary>
    /// Gets the package manager identifier this installer handles.
    /// </summary>
    string PackageManager { get; }

    /// <summary>
    /// Installs a single package.
    /// </summary>
    /// <param name="package">Information about the package to install</param>
    /// <param name="progress">Progress reporter for installation status</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The package installation result</returns>
    Task<PackageInstallationResult> InstallPackageAsync(
        PackageInfo package,
        IProgress<PackageInstallationProgress>? progress = null,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Installs multiple packages sequentially with progress reporting.
    /// </summary>
    /// <param name="packages">Collection of packages to install</param>
    /// <param name="progress">Progress reporter for overall installation status</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>Collection of package installation results</returns>
    Task<BatchInstallationResult> InstallPackagesAsync(
        IEnumerable<PackageInfo> packages,
        IProgress<BatchInstallationProgress>? progress = null,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Checks if a package is already installed.
    /// </summary>
    /// <param name="package">Information about the package to check</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>True if the package is installed, false otherwise</returns>
    Task<bool> IsPackageInstalledAsync(
        PackageInfo package,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Validates that the package manager is available on the system.
    /// </summary>
    /// <returns>True if the package manager is available, false otherwise</returns>
    Task<bool> IsAvailableAsync();

    /// <summary>
    /// Gets the version of the package manager.
    /// </summary>
    /// <returns>Version string of the package manager, or null if unavailable</returns>
    Task<string?> GetVersionAsync();
}