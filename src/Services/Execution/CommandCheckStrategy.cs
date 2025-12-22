using System;
using System.Diagnostics;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Strategy for evaluating command-based health checks.
/// Executes a command and validates the output against expected patterns.
/// </summary>
public class CommandCheckStrategy : IHealthCheckStrategy
{
    private readonly ILogger<CommandCheckStrategy> _logger;

    public string CheckType => "command";

    public CommandCheckStrategy(ILogger<CommandCheckStrategy> logger)
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
            _logger.LogDebug("Executing command health check: {Name} - {Target}",
                healthCheck.Name, healthCheck.Target);

            // Parse command and arguments
            var (command, arguments) = ParseCommand(healthCheck.Target);

            // Execute with timeout
            var timeout = TimeSpan.FromSeconds(healthCheck.Timeout);
            using var timeoutCts = new CancellationTokenSource(timeout);
            using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(
                cancellationToken, timeoutCts.Token);

            var (exitCode, stdout, stderr) = await ExecuteCommandAsync(
                command, arguments, linkedCts.Token);

            result.ExitCode = exitCode;
            result.Output = stdout;
            result.ErrorOutput = stderr;
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;

            // Evaluate success
            if (exitCode == 0)
            {
                // Check expected pattern if specified
                if (!string.IsNullOrEmpty(healthCheck.Expected))
                {
                    result.Passed = stdout.Contains(healthCheck.Expected,
                        StringComparison.OrdinalIgnoreCase);
                    if (!result.Passed)
                    {
                        result.Message = $"Output does not contain expected: {healthCheck.Expected}";
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
                result.Message = $"Command exited with code: {exitCode}";
            }

            result.Status = result.Passed
                ? HealthCheckStatus.Passed
                : HealthCheckStatus.Failed;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            result.Status = HealthCheckStatus.Cancelled;
            result.Passed = false;
            result.Message = "Health check was cancelled";
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
        }
        catch (OperationCanceledException)
        {
            result.Status = HealthCheckStatus.Timeout;
            result.Passed = false;
            result.Message = $"Health check timed out after {healthCheck.Timeout} seconds";
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;
        }
        catch (Exception ex)
        {
            result.Status = HealthCheckStatus.Error;
            result.Passed = false;
            result.Message = $"Error executing command: {ex.Message}";
            result.ErrorOutput = ex.ToString();
            result.EndTime = DateTime.Now;
            result.Duration = result.EndTime - result.StartTime;

            _logger.LogError(ex, "Error executing command health check: {Name}", healthCheck.Name);
        }

        // Attach remediation info if check failed
        if (!result.Passed && healthCheck.Remediation != null)
        {
            result.Remediation = healthCheck.Remediation;
        }

        return result;
    }

    private (string command, string arguments) ParseCommand(string target)
    {
        if (string.IsNullOrWhiteSpace(target))
        {
            return (string.Empty, string.Empty);
        }

        // Trim leading/trailing whitespace to avoid empty command segments.
        target = target.Trim();

        bool inQuotes = false;
        int splitIndex = -1;

        for (int i = 0; i < target.Length; i++)
        {
            char c = target[i];

            if (c == '"')
            {
                // Toggle quoted section state. This is a simple model that assumes
                // double quotes are used to group arguments.
                inQuotes = !inQuotes;
                continue;
            }

            if (!inQuotes && char.IsWhiteSpace(c))
            {
                splitIndex = i;
                break;
            }
        }

        if (splitIndex == -1)
        {
            // No unquoted whitespace found; entire target is the command.
            return (target, string.Empty);
        }

        string command = target.Substring(0, splitIndex);
        string arguments = target.Substring(splitIndex + 1).TrimStart();

        return (command, arguments);
    }

    private async Task<(int exitCode, string stdout, string stderr)> ExecuteCommandAsync(
        string command, string arguments, CancellationToken cancellationToken)
    {
        var stdout = new StringBuilder();
        var stderr = new StringBuilder();

        using var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = command,
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

        process.OutputDataReceived += (s, e) =>
        {
            if (e.Data != null) stdout.AppendLine(e.Data);
        };

        process.ErrorDataReceived += (s, e) =>
        {
            if (e.Data != null) stderr.AppendLine(e.Data);
        };

        process.Start();
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();

        await process.WaitForExitAsync(cancellationToken);

        return (process.ExitCode, stdout.ToString(), stderr.ToString());
    }
}
