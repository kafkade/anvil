using System;
using System.Collections.Generic;
using System.Linq;

namespace Winforge.Models;

/// <summary>
/// Represents information about a health check validation.
/// Enhanced version of TestInfo with severity, timeout, and remediation support.
/// </summary>
public class HealthCheckInfo
{
    /// <summary>
    /// Gets or sets the display name of the health check.
    /// </summary>
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the type of health check: command, file, or registry.
    /// </summary>
    public string Type { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the target to check - command, file path, or registry key.
    /// </summary>
    public string Target { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the expected result or pattern to match.
    /// </summary>
    public string? Expected { get; set; }

    /// <summary>
    /// Gets or sets the severity level: critical, warning, or info.
    /// Default is warning.
    /// </summary>
    public HealthCheckSeverity Severity { get; set; } = HealthCheckSeverity.Warning;

    /// <summary>
    /// Gets or sets the timeout in seconds for this health check.
    /// Default is 30 seconds.
    /// </summary>
    public int Timeout { get; set; } = 30;

    /// <summary>
    /// Gets or sets the remediation information for when the check fails.
    /// </summary>
    public RemediationInfo? Remediation { get; set; }

    /// <summary>
    /// Creates a HealthCheckInfo from a TestInfo for backward compatibility.
    /// </summary>
    public static HealthCheckInfo FromTestInfo(TestInfo testInfo)
    {
        return new HealthCheckInfo
        {
            Name = testInfo.name,
            Type = testInfo.type,
            Target = testInfo.target,
            Expected = testInfo.expected,
            Severity = HealthCheckSeverity.Warning,
            Timeout = 30
        };
    }
}

/// <summary>
/// Severity levels for health checks.
/// </summary>
public enum HealthCheckSeverity
{
    /// <summary>Essential for workload functionality - fails entire validation.</summary>
    Critical,

    /// <summary>Important but not blocking - reported as warning.</summary>
    Warning,

    /// <summary>Informational check - reported for awareness only.</summary>
    Info
}

/// <summary>
/// Represents remediation information for a failed health check.
/// </summary>
public class RemediationInfo
{
    /// <summary>
    /// Gets or sets a human-readable hint for fixing the issue.
    /// </summary>
    public string? Hint { get; set; }

    /// <summary>
    /// Gets or sets an optional auto-fix command to execute.
    /// </summary>
    public string? Command { get; set; }

    /// <summary>
    /// Gets or sets the manual steps to fix the issue.
    /// </summary>
    public List<string> ManualSteps { get; set; } = new();
}

/// <summary>
/// Represents the result of a single health check evaluation.
/// </summary>
public class HealthCheckResult
{
    /// <summary>
    /// Gets or sets the health check that was evaluated.
    /// </summary>
    public HealthCheckInfo HealthCheck { get; set; } = new();

    /// <summary>
    /// Gets or sets whether the health check passed.
    /// </summary>
    public bool Passed { get; set; }

    /// <summary>
    /// Gets or sets the status of the health check.
    /// </summary>
    public HealthCheckStatus Status { get; set; }

    /// <summary>
    /// Gets or sets the type of check that was performed.
    /// </summary>
    public string CheckType { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the result message.
    /// </summary>
    public string? Message { get; set; }

    /// <summary>
    /// Gets or sets the output from the health check execution.
    /// </summary>
    public string? Output { get; set; }

    /// <summary>
    /// Gets or sets any error output from the health check.
    /// </summary>
    public string? ErrorOutput { get; set; }

    /// <summary>
    /// Gets or sets the exit code for command-based checks.
    /// </summary>
    public int? ExitCode { get; set; }

    /// <summary>
    /// Gets or sets the start time of the evaluation.
    /// </summary>
    public DateTime StartTime { get; set; }

    /// <summary>
    /// Gets or sets the end time of the evaluation.
    /// </summary>
    public DateTime EndTime { get; set; }

    /// <summary>
    /// Gets or sets the duration of the evaluation.
    /// </summary>
    public TimeSpan Duration { get; set; }

    /// <summary>
    /// Gets or sets the remediation information if the check failed.
    /// </summary>
    public RemediationInfo? Remediation { get; set; }
}

/// <summary>
/// Status of a health check evaluation.
/// </summary>
public enum HealthCheckStatus
{
    /// <summary>Health check is pending execution.</summary>
    Pending,

    /// <summary>Health check is currently running.</summary>
    Running,

    /// <summary>Health check passed successfully.</summary>
    Passed,

    /// <summary>Health check failed.</summary>
    Failed,

    /// <summary>Health check timed out.</summary>
    Timeout,

    /// <summary>Health check was cancelled.</summary>
    Cancelled,

    /// <summary>Health check encountered an error.</summary>
    Error,

    /// <summary>Health check was skipped.</summary>
    Skipped
}

/// <summary>
/// Represents the results of evaluating multiple health checks.
/// </summary>
public class HealthCheckBatchResult
{
    /// <summary>
    /// Gets or sets the individual health check results.
    /// </summary>
    public List<HealthCheckResult> Results { get; set; } = new();

    /// <summary>
    /// Gets or sets the start time of the batch evaluation.
    /// </summary>
    public DateTime StartTime { get; set; }

    /// <summary>
    /// Gets or sets the end time of the batch evaluation.
    /// </summary>
    public DateTime EndTime { get; set; }

    /// <summary>
    /// Gets or sets the total duration of the batch evaluation.
    /// </summary>
    public TimeSpan TotalDuration { get; set; }

    /// <summary>
    /// Gets the total number of health checks.
    /// </summary>
    public int TotalCount => Results.Count;

    /// <summary>
    /// Gets the number of passed health checks.
    /// </summary>
    public int PassedCount => Results.Count(r => r.Passed);

    /// <summary>
    /// Gets the number of failed health checks.
    /// </summary>
    public int FailedCount => Results.Count(r => !r.Passed && r.Status == HealthCheckStatus.Failed);

    /// <summary>
    /// Gets the number of critical failures.
    /// </summary>
    public int CriticalFailures => Results.Count(r =>
        !r.Passed && r.HealthCheck.Severity == HealthCheckSeverity.Critical);

    /// <summary>
    /// Gets the number of warning failures.
    /// </summary>
    public int WarningFailures => Results.Count(r =>
        !r.Passed && r.HealthCheck.Severity == HealthCheckSeverity.Warning);

    /// <summary>
    /// Gets whether all health checks passed.
    /// </summary>
    public bool AllPassed => Results.All(r => r.Passed);

    /// <summary>
    /// Gets whether any critical health checks failed.
    /// </summary>
    public bool HasCriticalFailures => CriticalFailures > 0;

    /// <summary>
    /// Gets the success rate as a percentage.
    /// </summary>
    public double SuccessRate => TotalCount > 0
        ? (double)PassedCount / TotalCount * 100
        : 100;

    /// <summary>
    /// Gets all failed results with remediation information.
    /// </summary>
    public IEnumerable<HealthCheckResult> FailedWithRemediation =>
        Results.Where(r => !r.Passed && r.Remediation != null);

    /// <summary>
    /// Gets all remediation hints from failed checks.
    /// </summary>
    public IEnumerable<string> RemediationHints =>
        FailedWithRemediation
            .Where(r => !string.IsNullOrEmpty(r.Remediation?.Hint))
            .Select(r => $"[{r.HealthCheck.Name}] {r.Remediation!.Hint}");
}

/// <summary>
/// Progress information for a single health check evaluation.
/// </summary>
public class HealthCheckProgress
{
    /// <summary>
    /// Gets or sets the health check being evaluated.
    /// </summary>
    public HealthCheckInfo HealthCheck { get; set; } = new();

    /// <summary>
    /// Gets or sets the current status.
    /// </summary>
    public HealthCheckStatus Status { get; set; }

    /// <summary>
    /// Gets or sets the current status message.
    /// </summary>
    public string Message { get; set; } = string.Empty;
}

/// <summary>
/// Progress information for batch health check evaluation.
/// </summary>
public class HealthCheckBatchProgress
{
    /// <summary>
    /// Gets or sets the current check index - 1-based.
    /// </summary>
    public int CurrentIndex { get; set; }

    /// <summary>
    /// Gets or sets the total number of checks.
    /// </summary>
    public int TotalCount { get; set; }

    /// <summary>
    /// Gets or sets the current health check being evaluated.
    /// </summary>
    public HealthCheckInfo CurrentCheck { get; set; } = new();

    /// <summary>
    /// Gets or sets the current check status.
    /// </summary>
    public HealthCheckStatus CurrentStatus { get; set; }

    /// <summary>
    /// Gets or sets the current status message.
    /// </summary>
    public string Message { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the number of passed checks so far.
    /// </summary>
    public int PassedCount { get; set; }

    /// <summary>
    /// Gets or sets the number of failed checks so far.
    /// </summary>
    public int FailedCount { get; set; }

    /// <summary>
    /// Gets the overall progress percentage.
    /// </summary>
    public double OverallPercentage => TotalCount > 0
        ? (double)CurrentIndex / TotalCount * 100
        : 0;
}

/// <summary>
/// Result of attempting to remediate a failed health check.
/// </summary>
public class RemediationResult
{
    /// <summary>
    /// Gets or sets the health check that was remediated.
    /// </summary>
    public HealthCheckInfo? HealthCheck { get; set; }

    /// <summary>
    /// Gets or sets whether the remediation was successful.
    /// </summary>
    public bool Success { get; set; }

    /// <summary>
    /// Gets or sets the result message.
    /// </summary>
    public string? Message { get; set; }

    /// <summary>
    /// Gets or sets any output from the remediation command.
    /// </summary>
    public string? Output { get; set; }

    /// <summary>
    /// Gets or sets the start time.
    /// </summary>
    public DateTime StartTime { get; set; }

    /// <summary>
    /// Gets or sets the end time.
    /// </summary>
    public DateTime EndTime { get; set; }

    /// <summary>
    /// Gets or sets the duration.
    /// </summary>
    public TimeSpan Duration { get; set; }
}
