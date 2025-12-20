using System.Threading;
using System.Threading.Tasks;
using Winforge.Models;

namespace Winforge.Interfaces;

/// <summary>
/// Strategy interface for different types of health checks.
/// Implements the Strategy pattern for extensible health check evaluation.
/// </summary>
public interface IHealthCheckStrategy
{
    /// <summary>
    /// Gets the check type this strategy handles (e.g., "command", "file", "registry").
    /// </summary>
    string CheckType { get; }

    /// <summary>
    /// Determines if this strategy can handle the given health check.
    /// </summary>
    /// <param name="healthCheck">The health check to evaluate</param>
    /// <returns>True if this strategy can handle the check type</returns>
    bool CanHandle(HealthCheckInfo healthCheck);

    /// <summary>
    /// Evaluates the health check using this strategy.
    /// </summary>
    /// <param name="healthCheck">The health check to evaluate</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The health check result</returns>
    Task<HealthCheckResult> EvaluateAsync(
        HealthCheckInfo healthCheck,
        CancellationToken cancellationToken = default);
}
