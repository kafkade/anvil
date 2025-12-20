using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Strategy for evaluating file-based health checks.
/// Checks for file/directory existence and optional content validation.
/// </summary>
public class FileCheckStrategy : IHealthCheckStrategy
{
    private readonly ILogger<FileCheckStrategy> _logger;

    public string CheckType => "file";

    public FileCheckStrategy(ILogger<FileCheckStrategy> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    public bool CanHandle(HealthCheckInfo healthCheck) =>
        healthCheck?.Type?.Equals(CheckType, StringComparison.OrdinalIgnoreCase) == true;

    public async Task<HealthCheckResult> EvaluateAsync(
        HealthCheckInfo healthCheck,
        CancellationToken cancellationToken = default)
    {
        var result = new HealthCheckResult
        {
            HealthCheck = healthCheck,
            StartTime = DateTime.Now,
            CheckType = CheckType
        };

        try
        {
            // Expand environment variables in path
            var expandedPath = Environment.ExpandEnvironmentVariables(healthCheck.Target);

            _logger.LogDebug("Checking file/directory existence: {Path}", expandedPath);

            // Check if it's a file or directory
            var fileExists = File.Exists(expandedPath);
            var directoryExists = Directory.Exists(expandedPath);

            if (fileExists || directoryExists)
            {
                result.Output = fileExists ? "File exists" : "Directory exists";

                // If expected content specified and it's a file, check content
                if (!string.IsNullOrEmpty(healthCheck.Expected) && fileExists)
                {
                    var content = await File.ReadAllTextAsync(expandedPath, cancellationToken);
                    result.Passed = content.Contains(healthCheck.Expected,
                        StringComparison.OrdinalIgnoreCase);

                    if (!result.Passed)
                    {
                        result.Message = $"File does not contain expected: {healthCheck.Expected}";
                    }
                }
                else
                {
                    result.Passed = true;
                }
            }
            else
            {
                result.Passed = false;
                result.Message = $"Path does not exist: {expandedPath}";
            }

            result.Status = result.Passed
                ? HealthCheckStatus.Passed
                : HealthCheckStatus.Failed;
        }
        catch (OperationCanceledException)
        {
            result.Status = HealthCheckStatus.Cancelled;
            result.Passed = false;
            result.Message = "Health check was cancelled";
        }
        catch (Exception ex)
        {
            result.Status = HealthCheckStatus.Error;
            result.Passed = false;
            result.Message = $"Error checking file: {ex.Message}";

            _logger.LogError(ex, "Error in file health check: {Name}", healthCheck.Name);
        }

        result.EndTime = DateTime.Now;
        result.Duration = result.EndTime - result.StartTime;

        // Attach remediation info if check failed
        if (!result.Passed && healthCheck.Remediation != null)
        {
            result.Remediation = healthCheck.Remediation;
        }

        return result;
    }
}
