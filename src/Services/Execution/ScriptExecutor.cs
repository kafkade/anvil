using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Service for analyzing PowerShell scripts as part of workload preview operations.
/// Provides script validation and analysis capabilities without actual execution.
/// NOTE: In preview mode, this service validates scripts but does not execute them.
/// </summary>
public class ScriptExecutor : IScriptExecutor
{
    private readonly ILogger<ScriptExecutor>? _logger;

    /// <summary>
    /// Initializes a new instance of the ScriptExecutor class.
    /// </summary>
    /// <param name="logger">The logger instance (optional)</param>
    public ScriptExecutor(ILogger<ScriptExecutor>? logger = null)
    {
        _logger = logger;
    }

    /// <summary>
    /// Analyzes a PowerShell script file without executing it (preview mode).
    /// In preview mode, this validates the script exists and reports what would be executed.
    /// </summary>
    /// <param name="scriptInfo">Information about the script to analyze</param>
    /// <param name="workingDirectory">The working directory for script analysis</param>
    /// <param name="progress">Progress reporter for script analysis</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The script analysis result</returns>
    public async Task<PowerShellExecutionResult> ExecuteScriptAsync(
        ScriptInfo scriptInfo,
        string workingDirectory,
        IProgress<string>? progress = null,
        CancellationToken cancellationToken = default)
    {
        if (scriptInfo == null)
            throw new ArgumentNullException(nameof(scriptInfo));
        if (string.IsNullOrEmpty(workingDirectory))
            throw new ArgumentException("Working directory cannot be null or empty", nameof(workingDirectory));

        var startTime = DateTime.Now;
        var result = new PowerShellExecutionResult
        {
            StartTime = startTime
        };

        try
        {
            var scriptPath = Path.Combine(workingDirectory, scriptInfo.file);
            result.ScriptContent = scriptPath;

            _logger?.LogInformation("Analyzing PowerShell script: {ScriptName} at {ScriptPath}",
                scriptInfo.name, scriptPath);

            // Validate script exists
            if (!File.Exists(scriptPath))
            {
                var error = $"Script file not found: {scriptPath}";
                _logger?.LogError(error);
                result.Success = false;
                result.ErrorMessage = error;
                result.Errors.Add(error);
                result.EndTime = DateTime.Now;
                result.Duration = result.EndTime - result.StartTime;
                return result;
            }

            progress?.Report($"Analyzing script: {scriptInfo.name}");

            // Read script content for preview
            var scriptContent = await File.ReadAllTextAsync(scriptPath, cancellationToken);
            var lineCount = scriptContent.Split('\n').Length;

            // In preview mode, we just validate and report what would be executed
            result.Success = true;
            result.Output.Add($"[PREVIEW] Would execute script: {scriptInfo.name}");
            result.Output.Add($"[PREVIEW] Script path: {scriptPath}");
            result.Output.Add($"[PREVIEW] Script lines: {lineCount}");
            result.Output.Add($"[PREVIEW] Run as: {scriptInfo.runAs ?? "user"}");
            
            if (scriptInfo.runAs == "admin")
            {
                result.Output.Add($"[PREVIEW] Note: This script requires administrator privileges");
            }

            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
            
            _logger?.LogInformation("PowerShell script analyzed successfully: {ScriptName} ({LineCount} lines)",
                scriptInfo.name, lineCount);
            
            progress?.Report($"Script analyzed: {scriptInfo.name}");
        }
        catch (OperationCanceledException)
        {
            _logger?.LogInformation("PowerShell script analysis cancelled: {ScriptName}", scriptInfo.name);
            result.Success = false;
            result.ErrorMessage = "Script analysis was cancelled";
            result.Errors.Add(result.ErrorMessage);
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
            
            progress?.Report($"Cancelled analysis of {scriptInfo.name}");
        }
        catch (Exception ex)
        {
            _logger?.LogError(ex, "Exception during PowerShell script analysis: {ScriptName}", scriptInfo.name);
            result.Success = false;
            result.ErrorMessage = $"Exception during script analysis: {ex.Message}";
            result.Errors.Add(result.ErrorMessage);
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
            
            progress?.Report($"Error analyzing {scriptInfo.name}: {ex.Message}");
        }

        return result;
    }

    /// <summary>
    /// Validates that a script file exists and is accessible.
    /// </summary>
    /// <param name="scriptInfo">Information about the script to validate</param>
    /// <param name="workingDirectory">The working directory containing the script</param>
    /// <returns>True if the script is valid and accessible, false otherwise</returns>
    public async Task<bool> ValidateScriptAsync(ScriptInfo scriptInfo, string workingDirectory)
    {
        if (scriptInfo == null || string.IsNullOrEmpty(workingDirectory))
            return false;

        try
        {
            var scriptPath = Path.Combine(workingDirectory, scriptInfo.file);
            
            if (!File.Exists(scriptPath))
            {
                _logger?.LogWarning("Script file not found: {ScriptPath}", scriptPath);
                return false;
            }

            // Check if we can read the file
            await File.ReadAllTextAsync(scriptPath);
            
            _logger?.LogDebug("Script validation successful: {ScriptPath}", scriptPath);
            return true;
        }
        catch (Exception ex)
        {
            _logger?.LogError(ex, "Error validating script: {ScriptName}", scriptInfo.name);
            return false;
        }
    }

    /// <summary>
    /// Checks if the script executor is available (always returns true in preview mode).
    /// </summary>
    /// <returns>Always returns true since preview mode doesn't require PowerShell</returns>
    public Task<bool> IsAvailableAsync()
    {
        return Task.FromResult(true);
    }
}