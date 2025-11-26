using System;
using System.Threading;
using System.Threading.Tasks;
using Winforge.Models;

namespace Winforge.Interfaces;

/// <summary>
/// Interface for executing scripts as part of workload operations.
/// Provides script execution capabilities with progress reporting and error handling.
/// </summary>
public interface IScriptExecutor
{
    /// <summary>
    /// Executes a PowerShell script file.
    /// </summary>
    /// <param name="scriptInfo">Information about the script to execute</param>
    /// <param name="workingDirectory">The working directory for script execution</param>
    /// <param name="progress">Progress reporter for script execution</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The script execution result</returns>
    Task<PowerShellExecutionResult> ExecuteScriptAsync(
        ScriptInfo scriptInfo,
        string workingDirectory,
        IProgress<string>? progress = null,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Validates that a script file exists and is accessible.
    /// </summary>
    /// <param name="scriptInfo">Information about the script to validate</param>
    /// <param name="workingDirectory">The working directory containing the script</param>
    /// <returns>True if the script is valid and accessible, false otherwise</returns>
    Task<bool> ValidateScriptAsync(ScriptInfo scriptInfo, string workingDirectory);

    /// <summary>
    /// Checks if the script executor is available (PowerShell is installed).
    /// </summary>
    /// <returns>True if PowerShell is available, false otherwise</returns>
    Task<bool> IsAvailableAsync();
}