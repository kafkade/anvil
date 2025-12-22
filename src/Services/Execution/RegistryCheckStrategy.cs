using System;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Microsoft.Win32;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Strategy for evaluating registry-based health checks.
/// Queries Windows registry keys and validates values.
/// </summary>
public class RegistryCheckStrategy : IHealthCheckStrategy
{
    private readonly ILogger<RegistryCheckStrategy> _logger;

    public string CheckType => "registry";

    public RegistryCheckStrategy(ILogger<RegistryCheckStrategy> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    public bool CanHandle(HealthCheckInfo healthCheck) =>
        healthCheck?.Type?.Equals(CheckType, StringComparison.OrdinalIgnoreCase) == true;

    public Task<HealthCheckResult> EvaluateAsync(
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
            _logger.LogDebug("Checking registry key: {Key}", healthCheck.Target);

            // Parse registry path
            var (rootKey, subKeyPath, valueName) = ParseRegistryPath(healthCheck.Target);

            using var key = rootKey.OpenSubKey(subKeyPath);

            if (key != null)
            {
                if (!string.IsNullOrEmpty(valueName))
                {
                    var value = key.GetValue(valueName);
                    if (value != null)
                    {
                        result.Output = value.ToString();

                        // Check expected value if specified
                        if (!string.IsNullOrEmpty(healthCheck.Expected))
                        {
                            result.Passed = result.Output?.Contains(healthCheck.Expected,
                                StringComparison.OrdinalIgnoreCase) == true;

                            if (!result.Passed)
                            {
                                result.Message = $"Registry value does not match expected: {healthCheck.Expected}";
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
                        result.Message = $"Registry value not found: {valueName}";
                    }
                }
                else
                {
                    // Just checking key existence
                    result.Passed = true;
                    result.Output = "Registry key exists";
                }
            }
            else
            {
                result.Passed = false;
                result.Message = $"Registry key not found: {healthCheck.Target}";
            }

            result.Status = result.Passed
                ? HealthCheckStatus.Passed
                : HealthCheckStatus.Failed;
        }
        catch (Exception ex)
        {
            result.Status = HealthCheckStatus.Error;
            result.Passed = false;
            result.Message = $"Error checking registry: {ex.Message}";

            _logger.LogError(ex, "Error in registry health check: {Name}", healthCheck.Name);
        }

        result.EndTime = DateTime.Now;
        result.Duration = result.EndTime - result.StartTime;

        // Attach remediation info if check failed
        if (!result.Passed && healthCheck.Remediation != null)
        {
            result.Remediation = healthCheck.Remediation;
        }

        return Task.FromResult(result);
    }

    private (RegistryKey rootKey, string subKeyPath, string? valueName) ParseRegistryPath(string path)
    {
        // Format: HKEY_CURRENT_USER\Software\Microsoft\...\ValueName
        // or: HKCU:\Software\Microsoft\...

        var normalizedPath = path
            .Replace("HKCU:", "HKEY_CURRENT_USER")
            .Replace("HKLM:", "HKEY_LOCAL_MACHINE")
            .Replace("HKCR:", "HKEY_CLASSES_ROOT")
            .Replace("HKU:", "HKEY_USERS")
            .TrimStart('\\');

        var parts = normalizedPath.Split('\\', 2);
        var rootKeyName = parts[0];
        var remainder = parts.Length > 1 ? parts[1] : "";

        // Determine root key
        RegistryKey rootKey = rootKeyName.ToUpperInvariant() switch
        {
            "HKEY_CURRENT_USER" => Registry.CurrentUser,
            "HKEY_LOCAL_MACHINE" => Registry.LocalMachine,
            "HKEY_CLASSES_ROOT" => Registry.ClassesRoot,
            "HKEY_USERS" => Registry.Users,
            _ => throw new ArgumentException($"Unknown registry root: {rootKeyName}")
        };

        // Parse value name from the path
        // Try to determine if the last component is a value name by attempting to open the full path as a key
        // If it doesn't exist as a key, assume the last component is a value name
        string? valueName = null;
        string subKeyPath = remainder;

        if (!string.IsNullOrEmpty(remainder))
        {
            // Try opening the full path as a key
            using (var testKey = rootKey.OpenSubKey(remainder))
            {
                if (testKey == null)
                {
                    // The full path doesn't exist as a key, so split off the last component as a value name
                    var lastSeparatorIndex = remainder.LastIndexOf('\\');
                    if (lastSeparatorIndex >= 0)
                    {
                        subKeyPath = remainder.Substring(0, lastSeparatorIndex);
                        valueName = remainder.Substring(lastSeparatorIndex + 1);
                    }
                    // If there's no separator, treat the whole thing as a key path (will fail at evaluation)
                }
            }
        }

        return (rootKey, subKeyPath, valueName);
    }
}
