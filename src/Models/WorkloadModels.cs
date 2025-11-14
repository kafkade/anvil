using System;
using System.Collections.Generic;

namespace Winforge.Models;

/// <summary>
/// Represents the configuration structure for a workload as defined in YAML files.
/// Contains all the components and metadata needed to install and configure a workload.
/// </summary>
public class WorkloadConfig
{
    /// <summary>
    /// Gets or sets the name of the workload.
    /// </summary>
    public string name { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the description of what this workload provides.
    /// </summary>
    public string description { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the version of this workload configuration.
    /// </summary>
    public string version { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the author or maintainer of this workload.
    /// </summary>
    public string author { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the list of packages to be installed for this workload.
    /// </summary>
    public List<PackageInfo> packages { get; set; } = new();

    /// <summary>
    /// Gets or sets the list of setup scripts to be executed for this workload.
    /// </summary>
    public List<ScriptInfo> scripts { get; set; } = new();

    /// <summary>
    /// Gets or sets the list of validation tests to verify the workload installation.
    /// </summary>
    public List<TestInfo> tests { get; set; } = new();

    /// <summary>
    /// Gets or sets the list of configuration files to be deployed for this workload.
    /// </summary>
    public List<FileInfo> files { get; set; } = new();
}

/// <summary>
/// Represents information about a package to be installed as part of a workload.
/// Defines package installation parameters including name, manager, version, and scope.
/// </summary>
public class PackageInfo
{
    /// <summary>
    /// Gets or sets the name of the package to be installed.
    /// </summary>
    public string name { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the package manager to use for installation (e.g., "winget", "choco", "npm", "pip").
    /// </summary>
    public string manager { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the specific version of the package to install. 
    /// If empty, the latest version will be installed.
    /// </summary>
    public string version { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets a value indicating whether the package should be installed globally.
    /// Applies to package managers that support global/local installation scopes.
    /// </summary>
    public bool global { get; set; }
}

/// <summary>
/// Represents information about a setup script to be executed as part of a workload.
/// Defines script execution parameters including name, file path, and execution context.
/// </summary>
public class ScriptInfo
{
    /// <summary>
    /// Gets or sets the display name of the script for identification and logging.
    /// </summary>
    public string name { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the relative path to the script file from the workload directory.
    /// </summary>
    public string file { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the execution context for the script.
    /// Common values: "user" (current user), "admin" (elevated privileges).
    /// </summary>
    public string runAs { get; set; } = "user";
}

/// <summary>
/// Represents information about a validation test to verify workload installation.
/// Defines test execution parameters including name, type, target, and expected results.
/// </summary>
public class TestInfo
{
    /// <summary>
    /// Gets or sets the display name of the test for identification and reporting.
    /// </summary>
    public string name { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the type of test to be executed.
    /// Common values: "command", "file", "registry", "service", "process".
    /// </summary>
    public string type { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the target of the test (e.g., command to run, file to check, service name).
    /// </summary>
    public string target { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the expected result or output that indicates a successful test.
    /// </summary>
    public string expected { get; set; } = string.Empty;
}

/// <summary>
/// Represents information about a configuration file to be deployed as part of a workload.
/// Defines file deployment parameters including source, destination, and overwrite behavior.
/// </summary>
public class FileInfo
{
    /// <summary>
    /// Gets or sets the source path of the file relative to the workload directory.
    /// </summary>
    public string source { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the destination path where the file should be deployed.
    /// Can include environment variables and special folder references.
    /// </summary>
    public string destination { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets a value indicating whether existing files at the destination should be overwritten.
    /// When false, existing files will be preserved and deployment will be skipped.
    /// </summary>
    public bool overwrite { get; set; } = true;
}

/// <summary>
/// Represents the results and statistics from executing workload installations.
/// Provides comprehensive tracking of execution progress, success rates, and recommendations.
/// </summary>
public class ExecutionResults
{
    /// <summary>
    /// Gets or sets the start time of the workload execution.
    /// </summary>
    public DateTime StartTime { get; set; }

    /// <summary>
    /// Gets or sets the end time of the workload execution.
    /// </summary>
    public DateTime EndTime { get; set; }

    /// <summary>
    /// Gets or sets the total execution time in seconds.
    /// </summary>
    public int TotalTimeSeconds { get; set; }

    /// <summary>
    /// Gets or sets the execution mode that was used (e.g., "install", "validate", "both").
    /// </summary>
    public string ExecutionMode { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the total number of packages that were processed.
    /// </summary>
    public int TotalPackages { get; set; }

    /// <summary>
    /// Gets or sets the total number of scripts that were processed.
    /// </summary>
    public int TotalScripts { get; set; }

    /// <summary>
    /// Gets or sets the total number of tests that were processed.
    /// </summary>
    public int TotalTests { get; set; }

    /// <summary>
    /// Gets or sets the number of packages that were successfully installed.
    /// </summary>
    public int SuccessfulPackages { get; set; }

    /// <summary>
    /// Gets or sets the number of scripts that were successfully executed.
    /// </summary>
    public int SuccessfulScripts { get; set; }

    /// <summary>
    /// Gets or sets the number of tests that passed successfully.
    /// </summary>
    public int SuccessfulTests { get; set; }

    /// <summary>
    /// Gets or sets the number of packages that failed to install.
    /// </summary>
    public int FailedPackages { get; set; }

    /// <summary>
    /// Gets or sets the number of scripts that failed to execute.
    /// </summary>
    public int FailedScripts { get; set; }

    /// <summary>
    /// Gets or sets the number of tests that failed.
    /// </summary>
    public int FailedTests { get; set; }

    /// <summary>
    /// Gets or sets a list of specific failure messages and error details.
    /// </summary>
    public List<string> Failures { get; set; } = new();

    /// <summary>
    /// Gets or sets a list of recommendations for the user based on execution results.
    /// </summary>
    public List<string> Recommendations { get; set; } = new();

    /// <summary>
    /// Gets the overall success rate as a percentage (0-100).
    /// </summary>
    public double SuccessRate => (TotalItems > 0) ? (double)SuccessfulItems / TotalItems * 100 : 100;

    /// <summary>
    /// Gets the total number of items (packages + scripts + tests) that were processed.
    /// </summary>
    public int TotalItems => TotalPackages + TotalScripts + TotalTests;

    /// <summary>
    /// Gets the total number of items that completed successfully.
    /// </summary>
    public int SuccessfulItems => SuccessfulPackages + SuccessfulScripts + SuccessfulTests;

    /// <summary>
    /// Gets the total number of items that failed to complete.
    /// </summary>
    public int FailedItems => FailedPackages + FailedScripts + FailedTests;
}

/// <summary>
/// Represents the result of a PowerShell script execution.
/// </summary>
public class PowerShellExecutionResult
{
    /// <summary>
    /// Gets or sets the PowerShell script content that was executed.
    /// </summary>
    public string ScriptContent { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets a value indicating whether the script execution was successful.
    /// </summary>
    public bool Success { get; set; }

    /// <summary>
    /// Gets or sets the output from the script execution.
    /// </summary>
    public List<string> Output { get; set; } = new();

    /// <summary>
    /// Gets or sets the errors from the script execution.
    /// </summary>
    public List<string> Errors { get; set; } = new();

    /// <summary>
    /// Gets or sets the primary error message.
    /// </summary>
    public string ErrorMessage { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the start time of script execution.
    /// </summary>
    public DateTime StartTime { get; set; }

    /// <summary>
    /// Gets or sets the end time of script execution.
    /// </summary>
    public DateTime EndTime { get; set; }

    /// <summary>
    /// Gets or sets the duration of script execution.
    /// </summary>
    public TimeSpan Duration { get; set; }
}