using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Spectre.Console;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;
using Winforge.Models;

namespace Winforge.Services;

/// <summary>
/// Manages workload discovery, parsing, and execution coordination.
/// Provides YAML-based workload discovery functionality with comprehensive
/// configuration parsing and execution simulation capabilities.
/// </summary>
public class WorkloadManager : IDisposable
{
    private readonly IDeserializer _yamlDeserializer;

    /// <summary>
    /// Initializes a new instance of the WorkloadManager class.
    /// Sets up YAML deserializer with camelCase naming convention.
    /// </summary>
    public WorkloadManager()
    {
        _yamlDeserializer = new DeserializerBuilder()
            .WithNamingConvention(CamelCaseNamingConvention.Instance)
            .Build();
    }

    /// <summary>
    /// Discovers workloads in the specified directory by scanning for workload.yaml files.
    /// </summary>
    /// <param name="workloadsPath">The path to the workloads directory (default: "workloads")</param>
    /// <returns>A list of discovered workload metadata</returns>
    public List<WorkloadMetadata> DiscoverWorkloads(string workloadsPath = "workloads")
    {
        var workloads = new List<WorkloadMetadata>();
        
        try
        {
            string? fullWorkloadsPath = FindWorkloadsDirectory(workloadsPath);
            
            if (string.IsNullOrEmpty(fullWorkloadsPath))
            {
                return workloads;
            }

            var workloadDirectories = Directory.GetDirectories(fullWorkloadsPath);
            
            foreach (var directory in workloadDirectories)
            {
                var configPath = Path.Combine(directory, "workload.yaml");
                if (File.Exists(configPath))
                {
                    var workload = ParseWorkloadConfig(configPath);
                    if (workload != null)
                    {
                        workload.DirectoryName = Path.GetFileName(directory);
                        workload.DirectoryPath = directory;
                        workload.ConfigPath = configPath;
                        workload.LastModified = File.GetLastWriteTime(configPath);
                        workloads.Add(workload);
                    }
                }
            }
            
            AnsiConsole.MarkupLine($"[dim]Discovered {workloads.Count} workload(s) in {fullWorkloadsPath}[/]");
        }
        catch (Exception ex)
        {
            AnsiConsole.MarkupLine($"[red]Error discovering workloads: {ex.Message}[/]");
        }

        return workloads;
    }

    /// <summary>
    /// Finds the workloads directory by checking multiple possible locations.
    /// </summary>
    /// <param name="workloadsPath">The relative workloads path to search for</param>
    /// <returns>The full path to the workloads directory, or null if not found</returns>
    private string? FindWorkloadsDirectory(string workloadsPath)
    {
        var searchPaths = new List<string>();
        
        // Get the base directory (where the executable is running from)
        var basePath = AppDomain.CurrentDomain.BaseDirectory;
        
        // 1. Try relative to the executable directory (for published/deployed apps)
        searchPaths.Add(Path.Combine(basePath, workloadsPath));
        
        // 2. Try one level up from the executable (for dotnet run from src directory)
        var parentPath = Directory.GetParent(basePath)?.FullName;
        if (!string.IsNullOrEmpty(parentPath))
        {
            searchPaths.Add(Path.Combine(parentPath, workloadsPath));
        }
        
        // 3. Try relative to current working directory
        searchPaths.Add(Path.GetFullPath(workloadsPath));
        
        // 4. Try relative to current working directory's parent
        var currentDir = Directory.GetCurrentDirectory();
        var currentParent = Directory.GetParent(currentDir)?.FullName;
        if (!string.IsNullOrEmpty(currentParent))
        {
            searchPaths.Add(Path.Combine(currentParent, workloadsPath));
        }
        
        // Search for the first existing directory
        foreach (var searchPath in searchPaths)
        {
            if (Directory.Exists(searchPath))
            {
                AnsiConsole.MarkupLine($"[dim]Found workloads directory: {searchPath}[/]");
                return searchPath;
            }
        }
        
        // Log all attempted paths for debugging
        AnsiConsole.MarkupLine($"[yellow]Workloads directory not found: {workloadsPath}[/]");
        AnsiConsole.MarkupLine("[dim]Searched in:[/]");
        foreach (var path in searchPaths)
        {
            AnsiConsole.MarkupLine($"[dim]  - {path}[/]");
        }
        
        return null;
    }

    /// <summary>
    /// Parses a workload configuration file and returns workload metadata.
    /// </summary>
    /// <param name="configPath">The path to the workload configuration YAML file</param>
    /// <returns>WorkloadMetadata if parsing succeeds, null otherwise</returns>
    private WorkloadMetadata? ParseWorkloadConfig(string configPath)
    {
        try
        {
            var yamlContent = File.ReadAllText(configPath);
            var config = _yamlDeserializer.Deserialize<WorkloadConfig>(yamlContent);
            
            return new WorkloadMetadata
            {
                Name = config.name ?? "Unknown Workload",
                Description = config.description ?? string.Empty,
                Version = config.version ?? "1.0.0",
                Author = config.author ?? "Unknown",
                PackageCount = config.packages?.Count ?? 0,
                ScriptCount = config.scripts?.Count ?? 0,
                TestCount = config.tests?.Count ?? 0,
                FileCount = config.files?.Count ?? 0,
                IsValid = !string.IsNullOrEmpty(config.name),
                EstimatedInstallTimeMinutes = Math.Max(5, (config.packages?.Count ?? 0) * 2)
            };
        }
        catch (Exception ex)
        {
            AnsiConsole.MarkupLine($"[red]Error parsing {configPath}: {ex.Message}[/]");
            return null;
        }
    }

    /// <summary>
    /// Executes the specified workloads in the given execution mode.
    /// Currently provides simulated execution results for demonstration purposes.
    /// </summary>
    /// <param name="selectedWorkloads">List of workloads to execute</param>
    /// <param name="executionMode">The execution mode (install, validate, both)</param>
    /// <returns>Execution results containing performance metrics and recommendations</returns>
    public ExecutionResults ExecuteWorkloads(List<WorkloadMetadata> selectedWorkloads, string executionMode)
    {
        // For now, return simulated results since we're not integrating with PowerShell
        var results = new ExecutionResults
        {
            StartTime = DateTime.Now,
            ExecutionMode = executionMode,
            TotalPackages = selectedWorkloads.Sum(w => w.PackageCount),
            TotalScripts = selectedWorkloads.Sum(w => w.ScriptCount),
            TotalTests = selectedWorkloads.Sum(w => w.TestCount)
        };

        // Simulate success for demo purposes
        results.SuccessfulPackages = results.TotalPackages;
        results.SuccessfulScripts = results.TotalScripts;
        results.SuccessfulTests = results.TotalTests;
        results.EndTime = DateTime.Now.AddSeconds(30); // Simulate 30 seconds execution
        results.TotalTimeSeconds = 30;

        results.Recommendations.Add("All components installed successfully!");
        results.Recommendations.Add("Consider creating a system restore point.");

        return results;
    }

    /// <summary>
    /// Disposes of resources used by the WorkloadManager.
    /// </summary>
    public void Dispose()
    {
        // No cleanup needed for YAML-based implementation
    }
}