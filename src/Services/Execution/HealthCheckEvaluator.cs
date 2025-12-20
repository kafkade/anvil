using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Service for evaluating health checks as part of workload execution.
/// Orchestrates strategy selection and result aggregation.
/// </summary>
public class HealthCheckEvaluator : IHealthCheckEvaluator
{
    private readonly ILogger<HealthCheckEvaluator> _logger;
    private readonly IEnumerable<IHealthCheckStrategy> _strategies;

    public HealthCheckEvaluator(
        ILogger<HealthCheckEvaluator> logger,
        IEnumerable<IHealthCheckStrategy> strategies)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
        _strategies = strategies ?? throw new ArgumentNullException(nameof(strategies));
    }

    public async Task<HealthCheckResult> EvaluateAsync(
        HealthCheckInfo healthCheck,
        CancellationToken cancellationToken = default)
    {
        if (healthCheck == null)
            throw new ArgumentNullException(nameof(healthCheck));

        _logger.LogInformation("Evaluating health check: {Name} - Type: {Type}",
            healthCheck.Name, healthCheck.Type);

        // Select appropriate strategy
        var strategy = _strategies.FirstOrDefault(s => s.CanHandle(healthCheck));

        if (strategy == null)
        {
            _logger.LogError("No strategy found for health check type: {Type}", healthCheck.Type);
            return new HealthCheckResult
            {
                HealthCheck = healthCheck,
                Status = HealthCheckStatus.Error,
                Passed = false,
                Message = $"Unsupported health check type: {healthCheck.Type}",
                StartTime = DateTime.Now,
                EndTime = DateTime.Now
            };
        }

        // Execute the strategy
        var result = await strategy.EvaluateAsync(healthCheck, cancellationToken);

        _logger.LogInformation(
            "Health check {Name} {Status}: {Message}",
            healthCheck.Name,
            result.Passed ? "passed" : "failed",
            result.Message ?? "OK");

        return result;
    }

    public async Task<HealthCheckBatchResult> EvaluateAllAsync(
        IEnumerable<HealthCheckInfo> healthChecks,
        CancellationToken cancellationToken = default)
    {
        if (healthChecks == null)
            throw new ArgumentNullException(nameof(healthChecks));

        var checkList = healthChecks.ToList();
        var batchResult = new HealthCheckBatchResult
        {
            StartTime = DateTime.Now
        };

        _logger.LogInformation("Starting evaluation of {Count} health checks", checkList.Count);

        for (int i = 0; i < checkList.Count; i++)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                _logger.LogInformation("Health check evaluation cancelled at {Index}/{Total}",
                    i + 1, checkList.Count);
                break;
            }

            var healthCheck = checkList[i];

            // Evaluate the check
            var result = await EvaluateAsync(healthCheck, cancellationToken);
            batchResult.Results.Add(result);
        }

        batchResult.EndTime = DateTime.Now;
        batchResult.TotalDuration = batchResult.EndTime - batchResult.StartTime;

        _logger.LogInformation(
            "Health check evaluation completed: {Passed}/{Total} passed in {Duration}",
            batchResult.PassedCount,
            batchResult.TotalCount,
            batchResult.TotalDuration);

        return batchResult;
    }

    public async Task<RemediationResult> AttemptRemediationAsync(
        HealthCheckInfo healthCheck,
        CancellationToken cancellationToken = default)
    {
        if (healthCheck?.Remediation?.Command == null)
        {
            return new RemediationResult
            {
                Success = false,
                Message = "No auto-fix command available"
            };
        }

        var result = new RemediationResult
        {
            HealthCheck = healthCheck,
            StartTime = DateTime.Now
        };

        try
        {
            _logger.LogInformation("Attempting auto-fix: {Command}", healthCheck.Remediation.Command);

            // Execute remediation command
            using var process = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = "cmd.exe",
                    Arguments = $"/c {healthCheck.Remediation.Command}",
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    CreateNoWindow = true
                }
            };

            process.Start();

            var timeout = TimeSpan.FromSeconds(healthCheck.Timeout * 2); // Double timeout for remediation
            var completed = await Task.Run(() =>
                process.WaitForExit((int)timeout.TotalMilliseconds), cancellationToken);

            if (!completed)
            {
                process.Kill();
                result.Success = false;
                result.Message = "Remediation command timed out";
            }
            else
            {
                result.Success = process.ExitCode == 0;
                result.Output = await process.StandardOutput.ReadToEndAsync(cancellationToken);
                result.Message = result.Success
                    ? "Remediation completed successfully"
                    : $"Remediation failed with exit code: {process.ExitCode}";
            }
        }
        catch (Exception ex)
        {
            result.Success = false;
            result.Message = $"Remediation error: {ex.Message}";
            _logger.LogError(ex, "Error during remediation for {Name}", healthCheck.Name);
        }

        result.EndTime = DateTime.Now;
        result.Duration = result.EndTime - result.StartTime;

        return result;
    }
}
