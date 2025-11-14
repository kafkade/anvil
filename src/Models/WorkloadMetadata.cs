using System;
using System.Collections.Generic;

namespace Winforge.Models;

/// <summary>
/// Represents metadata information for a workload configuration.
/// Contains details about workload properties, validation status, and execution statistics.
/// </summary>
public class WorkloadMetadata
{
    /// <summary>
    /// Gets or sets the display name of the workload.
    /// </summary>
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the description of the workload's purpose and functionality.
    /// </summary>
    public string Description { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the version of the workload configuration.
    /// </summary>
    public string Version { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the author or maintainer of the workload.
    /// </summary>
    public string Author { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the directory name where the workload is located.
    /// </summary>
    public string DirectoryName { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the full path to the workload directory.
    /// </summary>
    public string DirectoryPath { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the path to the workload configuration file.
    /// </summary>
    public string ConfigPath { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the number of packages defined in this workload.
    /// </summary>
    public int PackageCount { get; set; }

    /// <summary>
    /// Gets or sets the number of setup scripts defined in this workload.
    /// </summary>
    public int ScriptCount { get; set; }

    /// <summary>
    /// Gets or sets the number of validation tests defined in this workload.
    /// </summary>
    public int TestCount { get; set; }

    /// <summary>
    /// Gets or sets the number of configuration files defined in this workload.
    /// </summary>
    public int FileCount { get; set; }

    /// <summary>
    /// Gets or sets a value indicating whether the workload configuration is valid.
    /// </summary>
    public bool IsValid { get; set; }

    /// <summary>
    /// Gets or sets a list of validation errors encountered while parsing the workload.
    /// </summary>
    public List<string> Errors { get; set; } = new();

    /// <summary>
    /// Gets or sets a list of validation warnings encountered while parsing the workload.
    /// </summary>
    public List<string> Warnings { get; set; } = new();

    /// <summary>
    /// Gets or sets the estimated installation time in minutes.
    /// </summary>
    public int EstimatedInstallTimeMinutes { get; set; }

    /// <summary>
    /// Gets or sets the last modification date of the workload configuration.
    /// </summary>
    public DateTime LastModified { get; set; }

    /// <summary>
    /// Gets or sets a value indicating whether this workload is selected for execution.
    /// </summary>
    public bool IsSelected { get; set; }
}