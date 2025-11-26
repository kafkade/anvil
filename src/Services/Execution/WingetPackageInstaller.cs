using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Service for installing packages using the Windows Package Manager (winget).
/// Provides package installation capabilities with progress reporting and error handling.
/// </summary>
public class WingetPackageInstaller : IPackageInstaller
{
    private readonly ILogger<WingetPackageInstaller> _logger;
    private const string WingetExecutable = "winget";
    private const int DefaultTimeoutSeconds = 300; // 5 minutes per package

    /// <summary>
    /// Gets the package manager identifier this installer handles.
    /// </summary>
    public string PackageManager => "winget";

    /// <summary>
    /// Initializes a new instance of the WingetPackageInstaller class.
    /// </summary>
    /// <param name="logger">The logger instance</param>
    public WingetPackageInstaller(ILogger<WingetPackageInstaller> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Installs a single package using winget.
    /// </summary>
    public async Task<PackageInstallationResult> InstallPackageAsync(
        PackageInfo package,
        IProgress<PackageInstallationProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        if (package == null)
            throw new ArgumentNullException(nameof(package));
        if (string.IsNullOrWhiteSpace(package.name))
            throw new ArgumentException("Package name cannot be empty", nameof(package));

        var result = new PackageInstallationResult
        {
            Package = package,
            StartTime = DateTime.Now,
            Status = PackageInstallationStatus.Installing
        };

        try
        {
            // _logger.LogDebug("Starting installation of package: {PackageName}", package.name);

            progress?.Report(new PackageInstallationProgress
            {
                Package = package,
                Status = PackageInstallationStatus.Installing,
                Message = $"Installing {package.name}..."
            });

            // Build the winget command
            var arguments = BuildInstallArguments(package);
            result.CommandExecuted = $"{WingetExecutable} {arguments}";

            // _logger.LogDebug("Executing command: {Command}", result.CommandExecuted);

            // Execute winget
            var (exitCode, stdout, stderr) = await ExecuteWingetAsync(
                arguments,
                cancellationToken);

            result.ExitCode = exitCode;
            result.StandardOutput = stdout;
            result.StandardError = stderr;
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;

            // Parse the result
            ParseInstallationResult(result);

            // _logger.LogDebug(
            //     "Package {PackageName} installation {Status} in {Duration}ms",
            //     package.name,
            //     result.Success ? "succeeded" : "failed",
            //     result.Duration.TotalMilliseconds);
        }
        catch (OperationCanceledException)
        {
            result.Status = PackageInstallationStatus.Cancelled;
            result.Success = false;
            result.Errors.Add("Installation was cancelled");
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;

            // _logger.LogDebug("Installation cancelled for package: {PackageName}", package.name);
        }
        catch (Exception ex)
        {
            result.Status = PackageInstallationStatus.Failed;
            result.Success = false;
            result.Errors.Add($"Exception during installation: {ex.Message}");
            result.ErrorCategory = PackageInstallationErrorCategory.Unknown;
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;

            // _logger.LogDebug(ex, "Exception during installation of package: {PackageName}", package.name);
        }

        progress?.Report(new PackageInstallationProgress
        {
            Package = package,
            Status = result.Status,
            Message = result.Success
                ? $"Successfully installed {package.name}"
                : $"Failed to install {package.name}"
        });

        return result;
    }

    /// <summary>
    /// Installs multiple packages sequentially with progress reporting.
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

        // _logger.LogDebug("Starting batch installation of {PackageCount} packages", packageList.Count);

        for (int i = 0; i < packageList.Count; i++)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                // _logger.LogDebug("Batch installation cancelled at package {Index}/{Total}",
                //     i + 1, packageList.Count);
                break;
            }

            var package = packageList[i];

            progress?.Report(new BatchInstallationProgress
            {
                CurrentPackageIndex = i + 1,
                TotalPackages = packageList.Count,
                CurrentPackage = package,
                CurrentStatus = PackageInstallationStatus.Installing,
                Message = GetFixedLengthMessage($"Installing {package.name} ({i + 1}/{packageList.Count})..."),
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
                Message = GetFixedLengthMessage(result.Success
                    ? $"Installed {package.name}"
                    : $"Failed: {package.name}"),
                SuccessCount = batchResult.SuccessCount,
                FailedCount = batchResult.FailedCount
            });
        }

        batchResult.EndTime = DateTime.Now;
        batchResult.TotalDuration = batchResult.EndTime - batchResult.StartTime;

        // _logger.LogDebug(
        //     "Batch installation completed: {SuccessCount}/{TotalCount} packages successful in {Duration}",
        //     batchResult.SuccessCount,
        //     batchResult.TotalPackages,
        //     batchResult.TotalDuration);

        return batchResult;
    }

    /// <summary>
    /// Checks if a package is already installed.
    /// </summary>
    public async Task<bool> IsPackageInstalledAsync(
        PackageInfo package,
        CancellationToken cancellationToken = default)
    {
        if (package == null || string.IsNullOrWhiteSpace(package.name))
            return false;

        try
        {
            // Use winget list to check for installation
            var arguments = $"list --name \"{package.name}\" --exact --accept-source-agreements";
            var (exitCode, stdout, _) = await ExecuteWingetAsync(arguments, cancellationToken);

            // If exit code is 0 and output contains the package name, it's installed
            return exitCode == 0 && stdout.Any(l => l.Contains(package.name, StringComparison.OrdinalIgnoreCase));
        }
        catch (Exception)
        {
            // _logger.LogWarning(ex, "Error checking if package is installed: {PackageName}", package.name);
            return false;
        }
    }

    /// <summary>
    /// Validates that the package manager is available on the system.
    /// </summary>
    public async Task<bool> IsAvailableAsync()
    {
        try
        {
            var (exitCode, _, _) = await ExecuteWingetAsync("--version", CancellationToken.None);
            return exitCode == 0;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Gets the version of the package manager.
    /// </summary>
    public async Task<string?> GetVersionAsync()
    {
        try
        {
            var (exitCode, stdout, _) = await ExecuteWingetAsync("--version", CancellationToken.None);
            return exitCode == 0 ? stdout.FirstOrDefault()?.Trim() : null;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Builds the winget install command arguments for a package.
    /// </summary>
    private string BuildInstallArguments(PackageInfo package)
    {
        var args = new StringBuilder();
        args.Append($"install \"{package.name}\"");

        // Add version if specified
        if (!string.IsNullOrWhiteSpace(package.version))
        {
            args.Append($" --version \"{package.version}\"");
        }

        // Standard options for non-interactive installation
        args.Append(" --accept-package-agreements");
        args.Append(" --accept-source-agreements");
        args.Append(" --silent");
        args.Append(" --disable-interactivity");

        return args.ToString();
    }

    /// <summary>
    /// Executes a winget command and captures output.
    /// </summary>
    private async Task<(int exitCode, List<string> stdout, List<string> stderr)> ExecuteWingetAsync(
        string arguments,
        CancellationToken cancellationToken)
    {
        var stdout = new List<string>();
        var stderr = new List<string>();

        using var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = WingetExecutable,
                Arguments = arguments,
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                CreateNoWindow = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8
            },
            EnableRaisingEvents = true
        };

        var outputTcs = new TaskCompletionSource<bool>();
        var errorTcs = new TaskCompletionSource<bool>();

        process.OutputDataReceived += (sender, e) =>
        {
            if (e.Data != null)
            {
                stdout.Add(e.Data);
                // _logger.LogDebug("[winget stdout] {Output}", e.Data);
            }
            else
            {
                outputTcs.TrySetResult(true);
            }
        };

        process.ErrorDataReceived += (sender, e) =>
        {
            if (e.Data != null)
            {
                stderr.Add(e.Data);
                // _logger.LogDebug("[winget stderr] {Output}", e.Data);
            }
            else
            {
                errorTcs.TrySetResult(true);
            }
        };

        process.Start();
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();

        using var timeoutCts = new CancellationTokenSource(TimeSpan.FromSeconds(DefaultTimeoutSeconds));
        using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCts.Token);

        try
        {
            await process.WaitForExitAsync(linkedCts.Token);
            await Task.WhenAll(outputTcs.Task, errorTcs.Task);
        }
        catch (OperationCanceledException) when (timeoutCts.IsCancellationRequested)
        {
            process.Kill(entireProcessTree: true);
            throw new TimeoutException($"Winget command timed out after {DefaultTimeoutSeconds} seconds");
        }
        catch (OperationCanceledException)
        {
            process.Kill(entireProcessTree: true);
            throw;
        }

        return (process.ExitCode, stdout, stderr);
    }

    /// <summary>
    /// Parses the winget installation result and sets appropriate status.
    /// </summary>
    private void ParseInstallationResult(PackageInstallationResult result)
    {
        // Check for specific output indicating already installed/no upgrade available
        // This overrides exit code checks because winget behavior can vary
        bool foundExisting = result.StandardOutput.Any(l => l.Contains("Found an existing package already installed", StringComparison.OrdinalIgnoreCase));
        bool noUpgrade = result.StandardOutput.Any(l => l.Contains("No available upgrade found", StringComparison.OrdinalIgnoreCase) ||
                                                        l.Contains("No newer package versions are available", StringComparison.OrdinalIgnoreCase));

        if (foundExisting && noUpgrade)
        {
            result.Success = true;
            result.Status = PackageInstallationStatus.AlreadyInstalled;
            result.AlreadyInstalled = true;
            result.ErrorCategory = PackageInstallationErrorCategory.None;
            result.Warnings.Add("Package is already installed and no upgrade is available");
            
            _logger.LogDebug("Package {PackageName} is already installed and up to date.", result.Package.name);
            
            ParseOutputForVersion(result);
            return;
        }

        // Check exit code
        switch (result.ExitCode)
        {
            case 0:
                result.Success = true;
                result.Status = PackageInstallationStatus.Succeeded;
                result.ErrorCategory = PackageInstallationErrorCategory.None;
                break;

            case -1978335212: // No applicable update
            case -1978335215: // Already installed
                result.Success = true;
                result.Status = PackageInstallationStatus.AlreadyInstalled;
                result.AlreadyInstalled = true;
                result.ErrorCategory = PackageInstallationErrorCategory.None;
                result.Warnings.Add("Package is already installed");
                break;

            case -1978335189: // Not found
                result.Success = false;
                result.Status = PackageInstallationStatus.NotFound;
                result.ErrorCategory = PackageInstallationErrorCategory.PackageNotFound;
                result.Errors.Add("Package not found in any configured source");
                break;

            case 3010: // Reboot required
                result.Success = true;
                result.Status = PackageInstallationStatus.RebootRequired;
                result.RebootRequired = true;
                result.ErrorCategory = PackageInstallationErrorCategory.None;
                result.Warnings.Add("A system reboot is required to complete the installation");
                break;

            default:
                result.Success = false;
                result.Status = PackageInstallationStatus.Failed;
                result.ErrorCategory = PackageInstallationErrorCategory.Unknown;
                result.Errors.Add($"Installation failed with exit code: {result.ExitCode}");
                break;
        }

        // Parse output for additional information
        ParseOutputForVersion(result);
        ParseOutputForErrors(result);
    }

    /// <summary>
    /// Parses the output to extract the installed version.
    /// </summary>
    private void ParseOutputForVersion(PackageInstallationResult result)
    {
        foreach (var line in result.StandardOutput)
        {
            // Look for version information in output
            if (line.Contains("Successfully installed") || line.Contains("Found"))
            {
                // Extract version if present (pattern varies by package)
                var versionMatch = System.Text.RegularExpressions.Regex.Match(
                    line, @"version\s+(\d+[\d\.]+)");
                if (versionMatch.Success)
                {
                    result.InstalledVersion = versionMatch.Groups[1].Value;
                }
            }
        }
    }

    /// <summary>
    /// Parses the output for additional error information.
    /// </summary>
    private void ParseOutputForErrors(PackageInstallationResult result)
    {
        foreach (var line in result.StandardError.Concat(result.StandardOutput))
        {
            if (line.Contains("error", StringComparison.OrdinalIgnoreCase) ||
                line.Contains("failed", StringComparison.OrdinalIgnoreCase))
            {
                if (!result.Errors.Any(e => e.Contains(line)))
                {
                    result.Errors.Add(line);
                }
            }
        }
    }

    /// <summary>
    /// Formats a message to a fixed length to prevent UI jitter.
    /// </summary>
    private string GetFixedLengthMessage(string message, int length = 60)
    {
        if (string.IsNullOrEmpty(message))
            return new string(' ', length);
            
        if (message.Length > length)
            return message.Substring(0, length - 3) + "...";
            
        return message.PadRight(length);
    }
}