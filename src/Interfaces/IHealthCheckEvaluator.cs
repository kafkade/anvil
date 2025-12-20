using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Winforge.Models;

namespace Winforge.Interfaces;

/// <summary>
/// Interface for health check evaluation services.
/// Provides health check execution with progress reporting and remediation hints.
/// </summary>
public interface IHealthCheckEvaluator
{
    /// <summary>
    /// Evaluates a single health check.
    /// </summary>
    /// <param name="healthCheck">The health check to evaluate</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The health check result</returns>
    Task<HealthCheckResult> EvaluateAsync(
        HealthCheckInfo healthCheck,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Evaluates multiple health checks sequentially.
    /// </summary>
    /// <param name="healthChecks">Collection of health checks to evaluate</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>Batch result containing all health check results</returns>
    Task<HealthCheckBatchResult> EvaluateAllAsync(
        IEnumerable<HealthCheckInfo> healthChecks,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Attempts to auto-remediate a failed health check.
    /// </summary>
    /// <param name="healthCheck">The failed health check with remediation info</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The remediation result</returns>
    Task<RemediationResult> AttemptRemediationAsync(
        HealthCheckInfo healthCheck,
        CancellationToken cancellationToken = default);
}
