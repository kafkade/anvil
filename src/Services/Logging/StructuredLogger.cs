using System;
using System.Collections.Generic;
using System.Threading;
using Microsoft.Extensions.Logging;
using Newtonsoft.Json;

namespace Winforge.Services.Logging;

/// <summary>
/// Provides structured logging with correlation IDs and standardized message formats.
/// Enhances the standard ILogger with context-aware logging capabilities.
/// </summary>
public class StructuredLogger : IStructuredLogger
{
    private readonly ILogger _logger;
    private readonly string _correlationId;
    private readonly Dictionary<string, object> _context;
    private static readonly ThreadLocal<string> _currentCorrelationId = new(() => Guid.NewGuid().ToString("N")[..8]);

    /// <summary>
    /// Gets the current correlation ID for the thread.
    /// </summary>
    public static string CurrentCorrelationId => _currentCorrelationId.Value ?? string.Empty;

    /// <summary>
    /// Sets the correlation ID for the current thread.
    /// </summary>
    public static void SetCorrelationId(string correlationId)
    {
        _currentCorrelationId.Value = correlationId;
    }

    /// <summary>
    /// Generates a new correlation ID for the current thread.
    /// </summary>
    public static string GenerateNewCorrelationId()
    {
        var correlationId = Guid.NewGuid().ToString("N")[..8];
        _currentCorrelationId.Value = correlationId;
        return correlationId;
    }

    /// <summary>
    /// Initializes a new instance of the StructuredLogger class.
    /// </summary>
    /// <param name="logger">The underlying logger instance</param>
    /// <param name="correlationId">Optional correlation ID</param>
    public StructuredLogger(ILogger logger, string? correlationId = null)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
        _correlationId = correlationId ?? CurrentCorrelationId;
        _context = new Dictionary<string, object>();
    }

    /// <summary>
    /// Creates a logger with additional context.
    /// </summary>
    /// <param name="context">Additional context to include in all log messages</param>
    /// <returns>A new logger instance with the specified context</returns>
    public IStructuredLogger WithContext(Dictionary<string, object> context)
    {
        var newLogger = new StructuredLogger(_logger, _correlationId);
        foreach (var kvp in _context)
        {
            newLogger._context[kvp.Key] = kvp.Value;
        }
        foreach (var kvp in context)
        {
            newLogger._context[kvp.Key] = kvp.Value;
        }
        return newLogger;
    }

    /// <summary>
    /// Creates a logger with a specific context type.
    /// </summary>
    /// <param name="contextType">The type of context (e.g., "workload", "package", "installation")</param>
    /// <param name="contextId">The identifier for this context</param>
    /// <returns>A new logger instance with the specified context</returns>
    public IStructuredLogger WithContext(string contextType, string contextId)
    {
        return WithContext(new Dictionary<string, object>
        {
            ["ContextType"] = contextType,
            ["ContextId"] = contextId
        });
    }

    /// <summary>
    /// Logs an installation start event.
    /// </summary>
    /// <param name="packageName">The package being installed</param>
    /// <param name="packageManager">The package manager being used</param>
    /// <param name="version">The package version</param>
    /// <param name="workloadContext">Optional workload context</param>
    public void LogInstallationStart(string packageName, string packageManager, string? version = null, string? workloadContext = null)
    {
        var logData = CreateLogData("InstallationStart", new Dictionary<string, object>
        {
            ["PackageName"] = packageName,
            ["PackageManager"] = packageManager,
            ["Version"] = version ?? "latest",
            ["WorkloadContext"] = workloadContext ?? string.Empty
        });

        _logger.LogInformation("Installation started: {PackageName} via {PackageManager} | {LogData}", 
            packageName, packageManager, JsonConvert.SerializeObject(logData));
    }

    /// <summary>
    /// Logs an installation completion event.
    /// </summary>
    /// <param name="packageName">The package that was installed</param>
    /// <param name="success">Whether the installation was successful</param>
    /// <param name="duration">The installation duration</param>
    /// <param name="errorMessage">Error message if unsuccessful</param>
    public void LogInstallationComplete(string packageName, bool success, TimeSpan duration, string? errorMessage = null)
    {
        var logData = CreateLogData("InstallationComplete", new Dictionary<string, object>
        {
            ["PackageName"] = packageName,
            ["Success"] = success,
            ["DurationMs"] = duration.TotalMilliseconds,
            ["ErrorMessage"] = errorMessage ?? string.Empty
        });

        var level = success ? LogLevel.Information : LogLevel.Error;
        var message = success 
            ? "Installation completed successfully: {PackageName} in {Duration}ms | {LogData}"
            : "Installation failed: {PackageName} after {Duration}ms - {Error} | {LogData}";

        _logger.Log(level, message, packageName, duration.TotalMilliseconds, errorMessage ?? string.Empty, JsonConvert.SerializeObject(logData));
    }

    /// <summary>
    /// Logs a performance measurement.
    /// </summary>
    /// <param name="operationName">The name of the operation being measured</param>
    /// <param name="duration">The duration of the operation</param>
    /// <param name="additionalData">Additional performance data</param>
    public void LogPerformance(string operationName, TimeSpan duration, Dictionary<string, object>? additionalData = null)
    {
        var perfData = new Dictionary<string, object>
        {
            ["OperationName"] = operationName,
            ["DurationMs"] = duration.TotalMilliseconds
        };

        if (additionalData != null)
        {
            foreach (var kvp in additionalData)
            {
                perfData[kvp.Key] = kvp.Value;
            }
        }

        var logData = CreateLogData("Performance", perfData);

        _logger.LogInformation("Performance: {OperationName} completed in {Duration}ms | {LogData}", 
            operationName, duration.TotalMilliseconds, JsonConvert.SerializeObject(logData));
    }

    /// <summary>
    /// Logs a system resource usage measurement.
    /// </summary>
    /// <param name="memoryUsageMB">Memory usage in MB</param>
    /// <param name="cpuUsagePercent">CPU usage percentage</param>
    /// <param name="activeOperations">Number of active operations</param>
    public void LogResourceUsage(double memoryUsageMB, double cpuUsagePercent, int activeOperations)
    {
        var logData = CreateLogData("ResourceUsage", new Dictionary<string, object>
        {
            ["MemoryUsageMB"] = memoryUsageMB,
            ["CpuUsagePercent"] = cpuUsagePercent,
            ["ActiveOperations"] = activeOperations
        });

        _logger.LogDebug("Resource usage: Memory={MemoryUsage}MB, CPU={CpuUsage}%, Operations={Operations} | {LogData}", 
            memoryUsageMB, cpuUsagePercent, activeOperations, JsonConvert.SerializeObject(logData));
    }

    /// <summary>
    /// Logs an audit event for system changes.
    /// </summary>
    /// <param name="action">The action being performed</param>
    /// <param name="target">The target of the action</param>
    /// <param name="details">Additional details about the action</param>
    public void LogAudit(string action, string target, Dictionary<string, object>? details = null)
    {
        var auditData = new Dictionary<string, object>
        {
            ["Action"] = action,
            ["Target"] = target,
            ["Timestamp"] = DateTime.UtcNow
        };

        if (details != null)
        {
            foreach (var kvp in details)
            {
                auditData[kvp.Key] = kvp.Value;
            }
        }

        var logData = CreateLogData("Audit", auditData);

        _logger.LogInformation("Audit: {Action} on {Target} | {LogData}", 
            action, target, JsonConvert.SerializeObject(logData));
    }

    /// <summary>
    /// Logs an error with structured data.
    /// </summary>
    /// <param name="exception">The exception that occurred</param>
    /// <param name="message">A descriptive message</param>
    /// <param name="additionalData">Additional structured data</param>
    public void LogError(Exception exception, string message, Dictionary<string, object>? additionalData = null)
    {
        var errorData = new Dictionary<string, object>
        {
            ["Message"] = message,
            ["ExceptionType"] = exception.GetType().Name,
            ["ExceptionMessage"] = exception.Message,
            ["StackTrace"] = exception.StackTrace ?? string.Empty
        };

        if (additionalData != null)
        {
            foreach (var kvp in additionalData)
            {
                errorData[kvp.Key] = kvp.Value;
            }
        }

        var logData = CreateLogData("Error", errorData);

        _logger.LogError(exception, "Error: {Message} | {LogData}", message, JsonConvert.SerializeObject(logData));
    }

    /// <summary>
    /// Creates standardized log data structure.
    /// </summary>
    private Dictionary<string, object> CreateLogData(string eventType, Dictionary<string, object> eventData)
    {
        var logData = new Dictionary<string, object>
        {
            ["Timestamp"] = DateTime.UtcNow.ToString("O"),
            ["CorrelationId"] = _correlationId,
            ["EventType"] = eventType
        };

        // Add context
        foreach (var kvp in _context)
        {
            logData[kvp.Key] = kvp.Value;
        }

        // Add event-specific data
        foreach (var kvp in eventData)
        {
            logData[kvp.Key] = kvp.Value;
        }

        return logData;
    }

    /// <summary>
    /// Logs a custom structured event.
    /// </summary>
    /// <param name="eventType">The type of event</param>
    /// <param name="message">The log message</param>
    /// <param name="level">The log level</param>
    /// <param name="eventData">Additional event data</param>
    public void LogEvent(string eventType, string message, LogLevel level = LogLevel.Information, Dictionary<string, object>? eventData = null)
    {
        var logData = CreateLogData(eventType, eventData ?? new Dictionary<string, object>());
        
        _logger.Log(level, "{Message} | {LogData}", message, JsonConvert.SerializeObject(logData));
    }

    /// <summary>
    /// Standard ILogger interface implementation.
    /// </summary>
    public IDisposable? BeginScope<TState>(TState state) where TState : notnull => _logger.BeginScope(state);
    public bool IsEnabled(LogLevel logLevel) => _logger.IsEnabled(logLevel);
    public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
        => _logger.Log(logLevel, eventId, state, exception, formatter);
}

/// <summary>
/// Interface for structured logging capabilities.
/// </summary>
public interface IStructuredLogger : ILogger
{
    /// <summary>
    /// Creates a logger with additional context.
    /// </summary>
    IStructuredLogger WithContext(Dictionary<string, object> context);

    /// <summary>
    /// Creates a logger with a specific context type.
    /// </summary>
    IStructuredLogger WithContext(string contextType, string contextId);

    /// <summary>
    /// Logs an installation start event.
    /// </summary>
    void LogInstallationStart(string packageName, string packageManager, string? version = null, string? workloadContext = null);

    /// <summary>
    /// Logs an installation completion event.
    /// </summary>
    void LogInstallationComplete(string packageName, bool success, TimeSpan duration, string? errorMessage = null);

    /// <summary>
    /// Logs a performance measurement.
    /// </summary>
    void LogPerformance(string operationName, TimeSpan duration, Dictionary<string, object>? additionalData = null);

    /// <summary>
    /// Logs a system resource usage measurement.
    /// </summary>
    void LogResourceUsage(double memoryUsageMB, double cpuUsagePercent, int activeOperations);

    /// <summary>
    /// Logs an audit event for system changes.
    /// </summary>
    void LogAudit(string action, string target, Dictionary<string, object>? details = null);

    /// <summary>
    /// Logs an error with structured data.
    /// </summary>
    void LogError(Exception exception, string message, Dictionary<string, object>? additionalData = null);

    /// <summary>
    /// Logs a custom structured event.
    /// </summary>
    void LogEvent(string eventType, string message, LogLevel level = LogLevel.Information, Dictionary<string, object>? eventData = null);
}