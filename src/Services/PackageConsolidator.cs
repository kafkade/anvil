using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Winforge.Models;

namespace Winforge.Services;

/// <summary>
/// Service responsible for consolidating packages from multiple workloads.
/// Handles extraction, deduplication, and organization of packages.
/// </summary>
public class PackageConsolidator
{
    /// <summary>
    /// Consolidates packages from the provided list of workloads.
    /// Loads configurations, extracts packages, deduplicates them, and groups by manager.
    /// </summary>
    /// <param name="selectedWorkloads">The list of workloads to process.</param>
    /// <param name="workloadManager">The workload manager service to load configurations.</param>
    /// <returns>A consolidated list of packages with statistics.</returns>
    public async Task<ConsolidatedPackageList> ConsolidateFromWorkloadsAsync(List<WorkloadMetadata> selectedWorkloads, WorkloadManager workloadManager)
    {
        var allPackages = new List<ConsolidatedPackage>();

        foreach (var workload in selectedWorkloads)
        {
            if (string.IsNullOrEmpty(workload.ConfigPath))
            {
                continue;
            }

            var config = await workloadManager.LoadWorkloadConfigAsync(workload.ConfigPath);
            if (config?.packages == null)
            {
                continue;
            }

            foreach (var pkg in config.packages)
            {
                allPackages.Add(new ConsolidatedPackage
                {
                    Name = pkg.name,
                    Manager = pkg.manager,
                    Version = pkg.version,
                    SourceWorkloads = new List<string> { workload.Name },
                    IsDuplicate = false
                });
            }
        }

        var uniquePackages = DeduplicatePackages(allPackages);

        return new ConsolidatedPackageList
        {
            PackagesByManager = uniquePackages
                .GroupBy(p => p.Manager)
                .ToDictionary(g => g.Key, g => g.ToList()),
            TotalPackages = allPackages.Count,
            UniquePackages = uniquePackages.Count,
            DuplicatesRemoved = allPackages.Count - uniquePackages.Count
        };
    }

    /// <summary>
    /// Deduplicates a list of packages based on name and manager.
    /// Merges source workloads and resolves version conflicts.
    /// </summary>
    /// <param name="packages">The raw list of packages to deduplicate.</param>
    /// <returns>A list of unique consolidated packages.</returns>
    public List<ConsolidatedPackage> DeduplicatePackages(List<ConsolidatedPackage> packages)
    {
        return packages
            .GroupBy(p => new { Name = p.Name.ToLowerInvariant(), Manager = p.Manager.ToLowerInvariant() })
            .Select(g =>
            {
                var count = g.Count();
                if (count == 1)
                {
                    return g.First();
                }

                // Merge logic for duplicates
                var sources = g.SelectMany(p => p.SourceWorkloads).Distinct().OrderBy(s => s).ToList();
                var bestVersion = ResolveVersion(g.Select(p => p.Version));
                
                // Use the name and manager from the first entry to preserve original casing
                var first = g.First();

                return new ConsolidatedPackage
                {
                    Name = first.Name,
                    Manager = first.Manager,
                    Version = bestVersion,
                    SourceWorkloads = sources,
                    IsDuplicate = true
                };
            })
            .ToList();
    }

    /// <summary>
    /// Resolves the best version from a collection of version strings.
    /// Prefers non-empty versions. If multiple exist, attempts to use SemVer comparison.
    /// </summary>
    private string ResolveVersion(IEnumerable<string> versions)
    {
        var nonEmptyVersions = versions.Where(v => !string.IsNullOrWhiteSpace(v)).Distinct().ToList();

        if (nonEmptyVersions.Count == 0)
        {
            return string.Empty;
        }

        if (nonEmptyVersions.Count == 1)
        {
            return nonEmptyVersions[0];
        }

        // Try to parse as System.Version for comparison
        // This handles standard x.y.z.w formats
        try
        {
            var parsedVersions = nonEmptyVersions
                .Select(v => new { Original = v, Parsed = Version.TryParse(v, out var pv) ? pv : null })
                .Where(x => x.Parsed != null)
                .OrderByDescending(x => x.Parsed)
                .ToList();

            if (parsedVersions.Any())
            {
                return parsedVersions.First().Original;
            }
        }
        catch
        {
            // Fallback if version parsing fails or throws
        }

        // Fallback to string comparison (lexicographical sort) if parsing fails or no valid versions found
        // This isn't perfect for all version schemes but provides a deterministic result
        return nonEmptyVersions.OrderByDescending(v => v).First();
    }
}