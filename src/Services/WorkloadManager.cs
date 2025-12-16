using Spectre.Console;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;
using Microsoft.Extensions.Logging;
using Winforge.Models;
using Winforge.Interfaces;

namespace Winforge.Services;

/// <summary>
/// Workload manager for workload discovery and preview/analysis.
/// Provides YAML-based workload discovery functionality with comprehensive
/// configuration parsing and preview capabilities showing what actions would be performed.
/// </summary>
public class WorkloadManager
{
    private readonly IDeserializer _yamlDeserializer;
    private readonly ILogger<WorkloadManager> _logger;
    private readonly IPackageInstaller? _packageInstaller;

    /// <summary>
    /// Initializes a new instance of the WorkloadManager class.
    /// Sets up YAML deserializer with camelCase naming convention.
    /// </summary>
    public WorkloadManager()
    {
        _yamlDeserializer = new DeserializerBuilder()
            .WithNamingConvention(CamelCaseNamingConvention.Instance)
            .Build();
        
        // Initialize logger
        var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
        _logger = loggerFactory.CreateLogger<WorkloadManager>();
        
        _logger.LogInformation("WorkloadManager initialized in preview/analysis mode");
    }

    /// <summary>
    /// Initializes a new instance of the WorkloadManager class with dependency injection.
    /// </summary>
    /// <param name="logger">Logger instance</param>
    /// <param name="packageInstaller">Optional package installer service</param>
    public WorkloadManager(ILogger<WorkloadManager> logger, IPackageInstaller? packageInstaller = null)
    {
        _yamlDeserializer = new DeserializerBuilder()
            .WithNamingConvention(CamelCaseNamingConvention.Instance)
            .Build();
        
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
        _packageInstaller = packageInstaller;
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
        catch (IOException ex)
        {
            AnsiConsole.MarkupLine($"[red]IO error parsing {configPath}: {ex.Message}[/]");
            return null;
        }
        catch (UnauthorizedAccessException ex)
        {
            AnsiConsole.MarkupLine($"[red]Access denied parsing {configPath}: {ex.Message}[/]");
            return null;
        }
        catch (YamlDotNet.Core.YamlException ex)
        {
            AnsiConsole.MarkupLine($"[red]YAML error parsing {configPath}: {ex.Message}[/]");
            return null;
        }
        catch (Exception ex)
        {
            AnsiConsole.MarkupLine($"[red]Error parsing {configPath}: {ex.Message}[/]");
            return null;
        }
    }

    /// <summary>
    /// Generates a preview of actions that would be performed for the specified workload.
    /// </summary>
    /// <param name="workload">The workload to preview</param>
    /// <returns>A preview showing all actions that would be performed</returns>
    public async Task<WorkloadPreview> PreviewWorkloadAsync(WorkloadMetadata workload)
    {
        if (workload == null)
            throw new ArgumentNullException(nameof(workload));

        _logger.LogInformation("Generating preview for workload: {WorkloadName}", workload.Name);

        var preview = new WorkloadPreview
        {
            WorkloadName = workload.Name,
            Description = workload.Description,
            Version = workload.Version
        };

        try
        {
            // Load workload configuration
            var workloadConfig = await LoadWorkloadConfigAsync(workload.ConfigPath);
            if (workloadConfig == null)
            {
                preview.Warnings.Add($"Failed to load workload configuration from: {workload.ConfigPath}");
                return preview;
            }

            // Validate workload
            var validation = await ValidateWorkloadConfigAsync(workloadConfig, workload.DirectoryPath);
            if (!validation.IsValid)
            {
                preview.Warnings.AddRange(validation.Issues);
            }

            // Generate action previews for all components
            await GenerateActionPreviewsAsync(workloadConfig, workload, preview);

            // Add recommendations
            AddPreviewRecommendations(preview);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error generating workload preview: {WorkloadName}", workload.Name);
            preview.Warnings.Add($"Error generating preview: {ex.Message}");
        }

        return preview;
    }

    /// <summary>
    /// Generates previews for multiple workloads.
    /// </summary>
    /// <param name="workloads">The workloads to preview</param>
    /// <returns>A list of previews for all workloads</returns>
    public async Task<List<WorkloadPreview>> PreviewWorkloadsAsync(List<WorkloadMetadata> workloads)
    {
        if (workloads == null || workloads.Count == 0)
            throw new ArgumentException("No workloads provided for preview", nameof(workloads));

        _logger.LogInformation("Generating previews for {WorkloadCount} workloads", workloads.Count);

        var previews = new List<WorkloadPreview>();

        foreach (var workload in workloads)
        {
            try
            {
                var preview = await PreviewWorkloadAsync(workload);
                previews.Add(preview);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error previewing workload: {WorkloadName}", workload.Name);
                previews.Add(new WorkloadPreview
                {
                    WorkloadName = workload.Name,
                    Description = workload.Description,
                    Version = workload.Version,
                    Warnings = new List<string> { $"Failed to generate preview: {ex.Message}" }
                });
            }
        }

        return previews;
    }

    /// <summary>
    /// Executes the installation of a workload.
    /// </summary>
    /// <param name="workload">The workload to install</param>
    /// <param name="progress">Optional progress reporter</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The execution results</returns>
    public async Task<ExecutionResults> ExecuteWorkloadAsync(
        WorkloadMetadata workload,
        IProgress<BatchInstallationProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        if (workload == null)
            throw new ArgumentNullException(nameof(workload));

        _logger.LogInformation("Executing workload: {WorkloadName}", workload.Name);

        var results = new ExecutionResults
        {
            StartTime = DateTime.Now,
            ExecutionMode = "install"
        };

        try
        {
            // Load workload configuration
            var workloadConfig = await LoadWorkloadConfigAsync(workload.ConfigPath);
            if (workloadConfig == null)
            {
                results.Failures.Add($"Failed to load workload configuration from: {workload.ConfigPath}");
                return results;
            }

            // Validate workload
            var validation = await ValidateWorkloadConfigAsync(workloadConfig, workload.DirectoryPath);
            if (!validation.IsValid)
            {
                results.Failures.AddRange(validation.Issues);
                return results;
            }

            // Install packages
            if (workloadConfig.packages?.Count > 0)
            {
                results.TotalPackages = workloadConfig.packages.Count;
                
                if (_packageInstaller != null)
                {
                    // Filter for packages managed by the registered installer
                    var supportedPackages = workloadConfig.packages
                        .Where(p => string.Equals(p.manager, _packageInstaller.PackageManager, StringComparison.OrdinalIgnoreCase))
                        .ToList();
                    
                    var unsupportedPackages = workloadConfig.packages
                        .Where(p => !string.Equals(p.manager, _packageInstaller.PackageManager, StringComparison.OrdinalIgnoreCase))
                        .ToList();

                    if (unsupportedPackages.Any())
                    {
                        var msg = $"Skipping {unsupportedPackages.Count} packages with unsupported managers: {string.Join(", ", unsupportedPackages.Select(p => p.manager).Distinct())}";
                        _logger.LogWarning(msg);
                        results.Recommendations.Add(msg);
                        results.FailedPackages += unsupportedPackages.Count; // Count as failed/skipped for now
                    }

                    if (supportedPackages.Any())
                    {
                        var installResults = await _packageInstaller.InstallPackagesAsync(supportedPackages, progress, cancellationToken);
                        
                        results.SuccessfulPackages += installResults.SuccessCount + installResults.AlreadyInstalledCount;
                        results.FailedPackages += installResults.FailedCount;
                        
                        // Store detailed package results
                        results.PackageResults.AddRange(installResults.Results);

                        // Add failures to results
                        foreach (var result in installResults.Results.Where(r => !r.Success && !r.AlreadyInstalled))
                        {
                            results.Failures.AddRange(result.Errors);
                        }

                        if (installResults.RebootRequired)
                        {
                            results.Recommendations.Add("A system reboot is required to complete package installation.");
                        }
                    }
                }
                else
                {
                    var msg = "No package installer service registered. Skipping package installation.";
                    _logger.LogWarning(msg);
                    results.Failures.Add(msg);
                    results.FailedPackages = workloadConfig.packages.Count;
                }
            }

            // TODO: Implement script execution and file operations
            if (workloadConfig.scripts?.Count > 0)
            {
                results.TotalScripts = workloadConfig.scripts.Count;
                results.Recommendations.Add($"Skipped {workloadConfig.scripts.Count} scripts (script execution not yet implemented)");
            }

            if (workloadConfig.tests?.Count > 0)
            {
                results.TotalTests = workloadConfig.tests.Count;
                results.Recommendations.Add($"Skipped {workloadConfig.tests.Count} tests (test execution not yet implemented)");
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error executing workload: {WorkloadName}", workload.Name);
            results.Failures.Add($"Error executing workload: {ex.Message}");
        }
        finally
        {
            results.EndTime = DateTime.Now;
            results.TotalTimeSeconds = (int)(results.EndTime - results.StartTime).TotalSeconds;
        }

        return results;
    }

    /// <summary>
    /// Loads a workload configuration from a YAML file.
    /// </summary>
    public async Task<WorkloadConfig?> LoadWorkloadConfigAsync(string configPath)
    {
        try
        {
            var yamlContent = await File.ReadAllTextAsync(configPath);
            return _yamlDeserializer.Deserialize<WorkloadConfig>(yamlContent);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to load workload configuration from {ConfigPath}", configPath);
            return null;
        }
    }

    /// <summary>
    /// Validates a workload configuration for preview.
    /// </summary>
    private async Task<WorkloadValidationResult> ValidateWorkloadConfigAsync(WorkloadConfig workloadConfig, string workloadDirectory)
    {
        var result = new WorkloadValidationResult { IsValid = true };

        try
        {
            // Validate package definitions
            if (workloadConfig.packages?.Count > 0)
            {
                foreach (var package in workloadConfig.packages)
                {
                    if (string.IsNullOrWhiteSpace(package.name))
                    {
                        result.IsValid = false;
                        result.Issues.Add("Package with empty name found");
                    }
                    if (string.IsNullOrWhiteSpace(package.manager))
                    {
                        result.IsValid = false;
                        result.Issues.Add($"Package '{package.name}' has no package manager specified");
                    }
                }
            }

            // Validate scripts
            if (workloadConfig.scripts?.Count > 0)
            {
                foreach (var script in workloadConfig.scripts)
                {
                    if (string.IsNullOrWhiteSpace(script.name))
                    {
                        result.Warnings.Add("Script with empty name found");
                    }
                    if (string.IsNullOrWhiteSpace(script.file))
                    {
                        result.IsValid = false;
                        result.Issues.Add($"Script '{script.name}' has no file path specified");
                    }
                    else
                    {
                        var scriptPath = Path.Combine(workloadDirectory, script.file);
                        if (!File.Exists(scriptPath))
                        {
                            result.Warnings.Add($"Script file not found: {script.file}");
                        }
                    }
                }
            }

            // Validate file operations
            if (workloadConfig.files?.Count > 0)
            {
                foreach (var file in workloadConfig.files)
                {
                    if (string.IsNullOrWhiteSpace(file.source))
                    {
                        result.IsValid = false;
                        result.Issues.Add("File operation with empty source path found");
                    }
                    if (string.IsNullOrWhiteSpace(file.destination))
                    {
                        result.IsValid = false;
                        result.Issues.Add($"File '{file.source}' has no destination specified");
                    }
                }
            }

            // Validate tests
            if (workloadConfig.tests?.Count > 0)
            {
                foreach (var test in workloadConfig.tests)
                {
                    if (string.IsNullOrWhiteSpace(test.name))
                    {
                        result.Warnings.Add("Test with empty name found");
                    }
                    if (string.IsNullOrWhiteSpace(test.type))
                    {
                        result.IsValid = false;
                        result.Issues.Add($"Test '{test.name}' has no type specified");
                    }
                    if (string.IsNullOrWhiteSpace(test.target))
                    {
                        result.IsValid = false;
                        result.Issues.Add($"Test '{test.name}' has no target specified");
                    }
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error during workload validation");
            result.IsValid = false;
            result.Issues.Add($"Validation error: {ex.Message}");
        }

        return await Task.FromResult(result);
    }

    /// <summary>
    /// Generates action previews for all workload components.
    /// </summary>
    private async Task GenerateActionPreviewsAsync(WorkloadConfig workloadConfig, WorkloadMetadata workloadMetadata, WorkloadPreview preview)
    {
        int estimatedSeconds = 0;

        // Generate package installation previews
        if (workloadConfig.packages?.Count > 0)
        {
            foreach (var package in workloadConfig.packages)
            {
                var action = new WorkloadAction
                {
                    ActionType = "Package Install",
                    Name = package.name,
                    Description = $"Install package '{package.name}' using {package.manager}",
                    Details = new Dictionary<string, string>
                    {
                        { "Manager", package.manager },
                        { "Version", string.IsNullOrWhiteSpace(package.version) ? "latest" : package.version }
                    },
                    EstimatedSeconds = 30 // Estimated time per package
                };
                preview.Actions.Add(action);
                estimatedSeconds += action.EstimatedSeconds;
            }
        }

        // Generate script execution previews
        if (workloadConfig.scripts?.Count > 0)
        {
            foreach (var script in workloadConfig.scripts)
            {
                var scriptPath = Path.Combine(workloadMetadata.DirectoryPath, script.file);
                var action = new WorkloadAction
                {
                    ActionType = "Script Execution",
                    Name = script.name,
                    Description = $"Execute PowerShell script: {script.file}",
                    Details = new Dictionary<string, string>
                    {
                        { "File", script.file },
                        { "RunAs", script.runAs },
                        { "Exists", File.Exists(scriptPath) ? "Yes" : "No" }
                    },
                    EstimatedSeconds = 15 // Estimated time per script
                };
                preview.Actions.Add(action);
                estimatedSeconds += action.EstimatedSeconds;
            }
        }

        // Generate file operation previews
        if (workloadConfig.files?.Count > 0)
        {
            foreach (var file in workloadConfig.files)
            {
                var action = new WorkloadAction
                {
                    ActionType = "File Operation",
                    Name = Path.GetFileName(file.source),
                    Description = $"Copy file from '{file.source}' to '{file.destination}'",
                    Details = new Dictionary<string, string>
                    {
                        { "Source", file.source },
                        { "Destination", file.destination },
                        { "Overwrite", file.overwrite ? "Yes" : "No" }
                    },
                    EstimatedSeconds = 2 // Estimated time per file operation
                };
                preview.Actions.Add(action);
                estimatedSeconds += action.EstimatedSeconds;
            }
        }

        // Generate validation test previews
        if (workloadConfig.tests?.Count > 0)
        {
            foreach (var test in workloadConfig.tests)
            {
                var action = new WorkloadAction
                {
                    ActionType = "Validation Test",
                    Name = test.name,
                    Description = $"Run {test.type} test: {test.target}",
                    Details = new Dictionary<string, string>
                    {
                        { "Type", test.type },
                        { "Target", test.target },
                        { "Expected", test.expected }
                    },
                    EstimatedSeconds = 5 // Estimated time per test
                };
                preview.Actions.Add(action);
                estimatedSeconds += action.EstimatedSeconds;
            }
        }

        preview.TotalEstimatedSeconds = estimatedSeconds;

        await Task.CompletedTask;
    }

    /// <summary>
    /// Adds recommendations to the preview based on the actions that would be performed.
    /// </summary>
    private void AddPreviewRecommendations(WorkloadPreview preview)
    {
        // Add time estimate recommendation
        if (preview.TotalEstimatedSeconds > 0)
        {
            var minutes = preview.TotalEstimatedSeconds / 60;
            var seconds = preview.TotalEstimatedSeconds % 60;
            
            if (minutes > 0)
            {
                preview.Recommendations.Add($"Estimated execution time: {minutes} minute(s) and {seconds} second(s)");
            }
            else
            {
                preview.Recommendations.Add($"Estimated execution time: {seconds} second(s)");
            }
        }

        // Add component-specific recommendations
        if (preview.TotalPackages > 0)
        {
            preview.Recommendations.Add($"Will install {preview.TotalPackages} package(s). Ensure you have administrator privileges and a stable internet connection.");
        }

        if (preview.TotalScripts > 0)
        {
            preview.Recommendations.Add($"Will execute {preview.TotalScripts} script(s). Review script contents before proceeding if you're concerned about security.");
        }

        if (preview.TotalFiles > 0)
        {
            preview.Recommendations.Add($"Will perform {preview.TotalFiles} file operation(s). Existing files may be overwritten depending on configuration.");
        }

        if (preview.TotalTests > 0)
        {
            preview.Recommendations.Add($"Will run {preview.TotalTests} validation test(s) to verify the installation.");
        }

        // Add warnings-based recommendations
        if (preview.Warnings.Count > 0)
        {
            preview.Recommendations.Add($"Found {preview.Warnings.Count} warning(s). Review warnings before executing this workload.");
        }

        // General recommendations
        if (preview.TotalActions > 10)
        {
            preview.Recommendations.Add("This is a complex workload with many actions. Consider creating a system restore point before proceeding.");
        }

        preview.Recommendations.Add("This is a preview only. No actual changes have been made to your system.");
    }
}