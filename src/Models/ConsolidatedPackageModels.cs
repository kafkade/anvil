using System.Collections.Generic;

namespace Winforge.Models;

/// <summary>
/// Represents a package that has been consolidated from multiple workloads.
/// Contains information about the package and its source workloads.
/// </summary>
public record ConsolidatedPackage
{
    /// <summary>
    /// Gets or sets the name of the package.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// Gets or sets the package manager used for this package.
    /// </summary>
    public string Manager { get; init; } = string.Empty;

    /// <summary>
    /// Gets or sets the version of the package.
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// Gets or sets the list of workloads that include this package.
    /// </summary>
    public List<string> SourceWorkloads { get; init; } = new();

    /// <summary>
    /// Gets or sets a value indicating whether this package was found in multiple workloads and deduplicated.
    /// </summary>
    public bool IsDuplicate { get; init; }
}

/// <summary>
/// Represents a list of consolidated packages, grouped by package manager.
/// Includes statistics about the consolidation process.
/// </summary>
public record ConsolidatedPackageList
{
    /// <summary>
    /// Gets or sets the dictionary of packages grouped by their package manager.
    /// </summary>
    public Dictionary<string, List<ConsolidatedPackage>> PackagesByManager { get; init; } = new();

    /// <summary>
    /// Gets or sets the total number of packages found across all workloads before deduplication.
    /// </summary>
    public int TotalPackages { get; init; }

    /// <summary>
    /// Gets or sets the number of unique packages after deduplication.
    /// </summary>
    public int UniquePackages { get; init; }

    /// <summary>
    /// Gets or sets the number of duplicate packages that were removed/merged.
    /// </summary>
    public int DuplicatesRemoved { get; init; }
}