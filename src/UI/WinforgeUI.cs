using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Text;
using Spectre.Console;
using Winforge.Models;
using Winforge.Services;
using Microsoft.Extensions.Logging;

namespace Winforge.UI;

/// <summary>
/// Main UI coordinator class that orchestrates all user interface components.
/// Provides the primary entry point for interactive mode and coordinates
/// between different UI subsystems while maintaining emoji-free presentation.
/// </summary>
public class WinforgeUI
{
    private readonly WorkloadManager _workloadManager;

    /// <summary>
    /// Initializes a new instance of the WinforgeUI class with all required components
    /// </summary>
    /// <param name="workloadManager">The workload manager service</param>
    public WinforgeUI(WorkloadManager workloadManager)
    {
        _workloadManager = workloadManager ?? throw new ArgumentNullException(nameof(workloadManager));
    }

    /// <summary>
    /// Displays the application banner
    /// </summary>
    public void ShowBanner()
    {
        var banner = new FigletText("WINFORGE")
            .LeftJustified()
            .Color(Color.Blue);

        AnsiConsole.Write(banner);
        AnsiConsole.MarkupLine("[grey]Workstation Setup & Management System v2.0[/]");
        AnsiConsole.WriteLine();
    }

    /// <summary>
    /// Displays the main menu and handles user selection
    /// </summary>
    /// <returns>The selected menu option key</returns>
    public string ShowMainMenu()
    {
        AnsiConsole.Clear();
        ShowBanner();

        var selection = AnsiConsole.Prompt(
            new SelectionPrompt<string>()
                .Title("[green]What would you like to do?[/]")
                .PageSize(10)
                .AddChoices(new[] {
                    "discover", "validate", "report", "help", "exit"
                })
                .UseConverter(choice => IconProvider.GetMenuDisplay(choice)));

        return selection;
    }

    /// <summary>
    /// Handles the workload discovery and selection workflow
    /// </summary>
    /// <returns>List of selected workloads, or empty list if none selected</returns>
    public async Task<List<WorkloadMetadata>> SelectWorkloadsAsync()
    {
        AnsiConsole.Clear();
        ShowBanner();

        // Show discovery progress
        await ShowDiscoveryProgress();

        var workloads = _workloadManager.DiscoverWorkloads();

        if (!workloads.Any())
        {
            AnsiConsole.MarkupLine("[red]No workloads found![/]");
            return new List<WorkloadMetadata>();
        }

        ShowWorkloadDiscoveryResults(workloads);

        var selectedWorkloads = ShowWorkloadSelection(workloads);
        
        if (!selectedWorkloads.Any())
        {
            AnsiConsole.MarkupLine("[yellow]No workloads selected.[/]");
            return new List<WorkloadMetadata>();
        }

        return selectedWorkloads;
    }

    /// <summary>
    /// Executes the selected workloads
    /// </summary>
    /// <param name="workloads">List of workloads to install</param>
    private async Task ExecuteWorkloadsAsync(List<WorkloadMetadata> workloads)
    {
        AnsiConsole.Clear();
        ShowBanner();
        AnsiConsole.MarkupLine("[bold blue]Installing Workloads...[/]\n");

        foreach (var workload in workloads)
        {
            AnsiConsole.MarkupLine($"[bold]Processing workload: {workload.Name}[/]");
            
            await AnsiConsole.Progress()
                .Columns(new ProgressColumn[]
                {
                    new TaskDescriptionColumn(),
                    new ProgressBarColumn(),
                    new PercentageColumn(),
                    new SpinnerColumn(),
                })
                .StartAsync(async ctx =>
                {
                    // Create a progress reporter that updates the Spectre.Console task
                    var progressTask = ctx.AddTask($"Installing {workload.Name} packages...");
                    var progressReporter = new Progress<BatchInstallationProgress>(p =>
                    {
                        progressTask.Description = p.Message;
                        progressTask.Value = p.OverallPercentage;
                    });

                    var results = await _workloadManager.ExecuteWorkloadAsync(workload, progressReporter);
                    
                    progressTask.Value = 100;
                    progressTask.StopTask();

                    // Show results summary for this workload
                    if (results.FailedItems > 0)
                    {
                        AnsiConsole.MarkupLine($"[red]Workload {workload.Name} completed with errors.[/]");
                        
                        // Display detailed error summary for failed packages
                        var failedPackages = results.PackageResults
                            .Where(r => !r.Success && !r.AlreadyInstalled)
                            .ToList();

                        if (failedPackages.Any())
                        {
                            AnsiConsole.WriteLine();
                            AnsiConsole.MarkupLine("[bold red]Failed Packages Summary:[/]");
                            
                            foreach (var failedPkg in failedPackages)
                            {
                                AnsiConsole.MarkupLine($"[red]Package: {failedPkg.Package.name}[/]");
                                if (failedPkg.Errors.Any())
                                {
                                    AnsiConsole.MarkupLine("[dim]Errors:[/]");
                                    foreach (var error in failedPkg.Errors)
                                    {
                                        AnsiConsole.MarkupLine($"[dim red]  - {error}[/]");
                                    }
                                }
                                AnsiConsole.WriteLine();
                            }
                        }
                        else
                        {
                            // Fallback for non-package failures
                            foreach (var failure in results.Failures)
                            {
                                AnsiConsole.MarkupLine($"[red]  - {failure}[/]");
                            }
                        }
                    }
                    else
                    {
                        AnsiConsole.MarkupLine($"[green]Workload {workload.Name} completed successfully![/]");
                    }
                    
                    if (results.Recommendations.Any())
                    {
                        AnsiConsole.MarkupLine("[yellow]Recommendations:[/]");
                        foreach (var rec in results.Recommendations)
                        {
                            AnsiConsole.MarkupLine($"[yellow]  - {rec}[/]");
                        }
                    }
                });
                
            AnsiConsole.WriteLine();
        }

        AnsiConsole.MarkupLine("[bold green]All operations completed.[/]");
        AnsiConsole.MarkupLine("Press any key to return to menu...");
        Console.ReadKey(true);
    }

    /// <summary>
    /// Shows discovery progress with spinning indicator
    /// </summary>
    /// <returns>Task representing the async operation</returns>
    private async Task ShowDiscoveryProgress()
    {
        await AnsiConsole.Status()
            .StartAsync("Discovering workloads...", async ctx =>
            {
                ctx.Spinner(Spinner.Known.Dots);
                ctx.SpinnerStyle(Style.Parse("green"));
                await Task.Delay(1000); // Simulate discovery time
            });
    }

    /// <summary>
    /// Displays the results of workload discovery in a formatted table
    /// </summary>
    /// <param name="workloads">List of discovered workloads</param>
    private void ShowWorkloadDiscoveryResults(List<WorkloadMetadata> workloads)
    {
        var table = new Table();
        table.AddColumn("Status");
        table.AddColumn("Name");
        table.AddColumn("Version");
        table.AddColumn("Packages");
        table.AddColumn("Est. Time");
        table.AddColumn("Description");

        foreach (var workload in workloads)
        {
            var status = workload.IsValid ? $"[green]{IconProvider.SUCCESS}[/]" : $"[red]{IconProvider.FAILURE}[/]";
            var name = workload.IsValid ? $"[white]{workload.Name}[/]" : $"[red]{workload.Name}[/]";
            
            table.AddRow(
                status,
                name,
                workload.Version,
                workload.PackageCount.ToString(),
                $"~{workload.EstimatedInstallTimeMinutes}m",
                workload.Description.Length > 50 ? 
                    workload.Description.Substring(0, 47) + "..." : 
                    workload.Description
            );
        }

        AnsiConsole.Write(table);
        AnsiConsole.WriteLine();
    }

    /// <summary>
    /// Shows the workload selection interface and returns selected workloads
    /// </summary>
    /// <param name="workloads">Available workloads to select from</param>
    /// <returns>List of selected workloads</returns>
    private List<WorkloadMetadata> ShowWorkloadSelection(List<WorkloadMetadata> workloads)
    {
        var validWorkloads = workloads.Where(w => w.IsValid).ToList();
        
        if (!validWorkloads.Any())
        {
            AnsiConsole.MarkupLine("[red]No valid workloads available for selection.[/]");
            return new List<WorkloadMetadata>();
        }

        var multiSelect = new MultiSelectionPrompt<WorkloadMetadata>()
            .Title("[green]Select workloads to install:[/]")
            .NotRequired()
            .PageSize(10)
            .MoreChoicesText("[grey](Move up and down to reveal more workloads)[/]")
            .InstructionsText("[grey](Press [blue]<space>[/] to toggle, [green]<enter>[/] to accept)[/]")
            .UseConverter(workload => 
                $"{workload.Name} [grey]({workload.PackageCount} packages, ~{workload.EstimatedInstallTimeMinutes}m)[/]");

        foreach (var workload in validWorkloads)
        {
            multiSelect.AddChoice(workload);
        }

        var selectedWorkloads = AnsiConsole.Prompt(multiSelect);
        
        return selectedWorkloads.ToList();
    }

    /// <summary>
    /// Displays the preview plan showing what would be installed
    /// </summary>
    /// <param name="selectedWorkloads">List of workloads to preview</param>
    private async Task ShowPreviewPlan(List<WorkloadMetadata> selectedWorkloads)
    {
        AnsiConsole.Clear();
        ShowBanner();

        var panel = new Panel(
            new Markup($"[bold]Preview Plan[/]\n\n" +
                      $"[green]Selected Workloads:[/] {selectedWorkloads.Count}\n" +
                      $"[green]Total Packages:[/] {selectedWorkloads.Sum(w => w.PackageCount)}\n" +
                      $"[green]Total Scripts:[/] {selectedWorkloads.Sum(w => w.ScriptCount)}\n" +
                      $"[green]Estimated Time:[/] ~{selectedWorkloads.Sum(w => w.EstimatedInstallTimeMinutes)} minutes"))
            .Header(IconProvider.GetHeaderText("plan"))
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Blue);

        AnsiConsole.Write(panel);
        
        // Show selected workloads in a table
        var table = new Table();
        table.AddColumn("Workload");
        table.AddColumn("Version");
        table.AddColumn("Packages");
        table.AddColumn("Scripts");
        table.AddColumn("Tests");
        table.AddColumn("Est. Time");

        foreach (var workload in selectedWorkloads)
        {
            table.AddRow(
                workload.Name,
                workload.Version,
                workload.PackageCount.ToString(),
                workload.ScriptCount.ToString(),
                workload.TestCount.ToString(),
                $"~{workload.EstimatedInstallTimeMinutes}m"
            );
        }

        AnsiConsole.Write(table);
        AnsiConsole.WriteLine();

        // Generate and display detailed package listing
        await ShowPackageListingAsync(selectedWorkloads);
    }

    /// <summary>
    /// Displays a detailed listing of all packages that will be installed, organized by workload
    /// </summary>
    /// <param name="selectedWorkloads">List of workloads to show packages for</param>
    private async Task ShowPackageListingAsync(List<WorkloadMetadata> selectedWorkloads)
    {
        List<WorkloadPreview> previews = null!;
        
        // Show progress while loading workload configurations
        await AnsiConsole.Status()
            .StartAsync("Loading package details...", async ctx =>
            {
                ctx.Spinner(Spinner.Known.Dots);
                ctx.SpinnerStyle(Style.Parse("green"));

                // Get previews for all selected workloads
                previews = await _workloadManager.PreviewWorkloadsAsync(selectedWorkloads);
            });

        // Create package listing table after status is complete
        var packageTable = new Table();
        packageTable.Border(TableBorder.Rounded);
        packageTable.AddColumn(new TableColumn("[bold]Package Name[/]").LeftAligned());
        packageTable.AddColumn(new TableColumn("[bold]Manager[/]").Centered());
        packageTable.AddColumn(new TableColumn("[bold]Version[/]").Centered());
        packageTable.AddColumn(new TableColumn("[bold]Workload[/]").LeftAligned());

        // Extract and display all package install actions
        int totalPackages = 0;
        foreach (var preview in previews)
        {
            // Filter for package install actions
            var packageActions = preview.Actions
                .Where(a => a.ActionType == "Package Install")
                .ToList();

            foreach (var action in packageActions)
            {
                var manager = action.Details.ContainsKey("Manager") ? action.Details["Manager"] : "unknown";
                var version = action.Details.ContainsKey("Version") ? action.Details["Version"] : "latest";
                
                // Color code package managers
                var managerDisplay = manager.ToLower() switch
                {
                    "winget" => "[blue]winget[/]",
                    "choco" => "[yellow]choco[/]",
                    "npm" => "[red]npm[/]",
                    "pip" => "[green]pip[/]",
                    _ => $"[grey]{manager}[/]"
                };

                packageTable.AddRow(
                    action.Name,
                    managerDisplay,
                    version,
                    $"[dim]{preview.WorkloadName}[/]"
                );
                totalPackages++;
            }
        }

        // Display the package listing table
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine($"[bold blue]Package Installation Details[/] [dim]({totalPackages} packages)[/]");
        AnsiConsole.Write(packageTable);
        AnsiConsole.WriteLine();
    }

    /// <summary>
    /// Displays the consolidated packages grouped by package manager
    /// </summary>
    /// <param name="packageList">The consolidated package list to display</param>
    public void DisplayConsolidatedPackageList(ConsolidatedPackageList packageList)
    {
        AnsiConsole.Clear();
        ShowBanner();

        // Summary Header
        AnsiConsole.MarkupLine($"[bold]Found {packageList.UniquePackages} unique packages ({packageList.DuplicatesRemoved} duplicates removed from {packageList.TotalPackages} total)[/]");
        AnsiConsole.WriteLine();

        foreach (var managerGroup in packageList.PackagesByManager)
        {
            var manager = managerGroup.Key;
            var packages = managerGroup.Value.OrderBy(p => p.Name).ToList();
            var icon = GetManagerIcon(manager);
            
            // Manager Header
            var managerColor = manager.ToLower() switch
            {
                "winget" => "blue",
                "choco" => "yellow",
                "npm" => "red",
                "pip" => "green",
                _ => "grey"
            };

            AnsiConsole.MarkupLine($"[{managerColor}]{icon} {manager}[/]");

            // Table
            var table = new Table();
            table.Border(TableBorder.Rounded);
            table.AddColumn("Package Name");
            table.AddColumn("Version");
            table.AddColumn("Source Workloads");

            foreach (var pkg in packages)
            {
                var version = string.IsNullOrEmpty(pkg.Version) ? "[dim]latest[/]" : pkg.Version;
                var sources = string.Join(", ", pkg.SourceWorkloads);
                var name = pkg.IsDuplicate ? $"{pkg.Name} [dim](merged)[/]" : pkg.Name;

                table.AddRow(name, version, sources);
            }

            AnsiConsole.Write(table);
            AnsiConsole.WriteLine();
        }
    }

    /// <summary>
    /// Asks the user to confirm package installation
    /// </summary>
    /// <param name="packageList">The consolidated package list</param>
    /// <returns>True if the user confirms, false otherwise</returns>
    public bool ConfirmPackageInstallation(ConsolidatedPackageList packageList)
    {
        AnsiConsole.MarkupLine($"[bold]Ready to install {packageList.UniquePackages} packages using {packageList.PackagesByManager.Count} package managers[/]");
        
        foreach (var manager in packageList.PackagesByManager)
        {
            AnsiConsole.MarkupLine($"  • {manager.Key} ({manager.Value.Count} packages)");
        }
        
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("[dim]Packages will be installed in order: winget, choco, npm, pip, others[/]");
        
        // Check if there are non-winget packages
        var hasNonWinget = packageList.PackagesByManager.Keys.Any(k => !string.Equals(k, "winget", StringComparison.OrdinalIgnoreCase));
        if (hasNonWinget)
        {
            AnsiConsole.MarkupLine("[yellow]Note: Non-winget packages will be simulated for this session[/]");
        }

        AnsiConsole.WriteLine();
        return AnsiConsole.Confirm("Do you want to proceed with installation?", true);
    }

    /// <summary>
    /// Gets the icon for a specific package manager
    /// </summary>
    /// <param name="manager">The package manager name</param>
    /// <returns>The icon string</returns>
    public string GetManagerIcon(string manager)
    {
        return manager.ToLower() switch
        {
            "winget" => "📦",
            "choco" => "🍫",
            "npm" => "📗",
            "pip" => "🐍",
            _ => "📥"
        };
    }

    /// <summary>
    /// Handles validation mode functionality (placeholder for future implementation)
    /// </summary>
    /// <returns>Task representing the async operation</returns>
    public async Task RunValidationMode()
    {
        AnsiConsole.Clear();
        ShowBanner();
        AnsiConsole.MarkupLine("[yellow]Validation mode - Feature coming soon![/]");
        await Task.Delay(1000);
    }

    /// <summary>
    /// Handles report generation functionality (placeholder for future implementation)
    /// </summary>
    /// <returns>Task representing the async operation</returns>
    public async Task GenerateReport()
    {
        AnsiConsole.Clear();
        ShowBanner();
        AnsiConsole.MarkupLine("[yellow]Report generation - Feature coming soon![/]");
        await Task.Delay(1000);
    }

    /// <summary>
    /// Displays the installation summary
    /// </summary>
    /// <param name="summary">The installation summary to display</param>
    public void DisplayInstallationSummary(InstallationSummary summary)
    {
        AnsiConsole.WriteLine();
        AnsiConsole.Write(new Rule("[blue]Installation Summary[/]"));
        AnsiConsole.WriteLine();

        // Overall stats
        var grid = new Grid();
        grid.AddColumn();
        grid.AddColumn();
        
        grid.AddRow(new Markup("[bold]Total Packages:[/][dim].......[/]"), new Markup(summary.TotalPackages.ToString()));
        grid.AddRow(new Markup("[green]Successful:[/][dim]...........[/]"), new Markup($"[green]{summary.SuccessfulInstalls}[/]"));
        grid.AddRow(new Markup("[red]Failed:[/][dim]...............[/]"), new Markup($"[red]{summary.FailedInstalls}[/]"));
        grid.AddRow(new Markup("[yellow]Skipped:[/][dim]..............[/]"), new Markup($"[yellow]{summary.SkippedInstalls}[/]"));
        grid.AddRow(new Markup("[bold]Total Time:[/][dim]..........[/]"), new Markup($"{summary.TotalDuration.TotalSeconds:F1}s"));
        
        AnsiConsole.Write(new Panel(grid)
            .Header("Results")
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Blue));

        AnsiConsole.WriteLine();

        // Per-manager breakdown
        var table = new Table();
        table.Border(TableBorder.Rounded);
        table.AddColumn("Manager");
        table.AddColumn("Packages");
        table.AddColumn("Success");
        table.AddColumn("Failed");
        table.AddColumn("Mode");

        foreach (var result in summary.ResultsByManager.Values)
        {
            var mode = result.WasSimulated ? "[yellow]Simulated[/]" : "[green]Real[/]";
            table.AddRow(
                result.Manager,
                result.PackageCount.ToString(),
                $"[green]{result.SuccessCount}[/]",
                result.FailureCount > 0 ? $"[red]{result.FailureCount}[/]" : "0",
                mode
            );
        }

        AnsiConsole.Write(table);

        // Failed packages details
        var allFailures = summary.ResultsByManager.Values
            .Where(r => r.FailureDetails != null)
            .SelectMany(r => r.FailureDetails)
            .ToList();

        if (allFailures.Any())
        {
            var report = new InstallationFailureReport
            {
                TotalFailures = allFailures.Count,
                Failures = allFailures
            };
            DisplayFailureReport(report);
        }
        else if (summary.FailedInstalls > 0)
        {
            AnsiConsole.WriteLine();
            AnsiConsole.MarkupLine("[red bold]Failed Packages:[/]");
            foreach (var result in summary.ResultsByManager.Values.Where(r => r.FailureCount > 0))
            {
                foreach (var failedPkg in result.FailedPackages)
                {
                    AnsiConsole.MarkupLine($"[red]  • {failedPkg} ({result.Manager})[/]");
                }
            }
        }

        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("Press any key to continue...");
        Console.ReadKey(true);
    }

    /// <summary>
    /// Runs the installation process with a progress display
    /// </summary>
    /// <param name="installationAction">The installation action to run</param>
    /// <returns>The installation summary</returns>
    public async Task<InstallationSummary> RunInstallationWithProgress(
        Func<IProgress<InstallationProgressReport>, Task<InstallationSummary>> installationAction)
    {
        InstallationSummary summary = null!;
        string lastMessage = string.Empty;

        await AnsiConsole.Progress()
            .Columns(new ProgressColumn[]
            {
                new ProgressBarColumn(),
                new PercentageColumn(),
                new SpinnerColumn(),
            })
            .StartAsync(async ctx =>
            {
                var progressTask = ctx.AddTask("[green]Starting installation...[/]");
                
                var progress = new Progress<InstallationProgressReport>(report =>
                {
                    // Update status message
                    if (!string.IsNullOrEmpty(report.StatusMessage) && report.StatusMessage != lastMessage)
                    {
                        AnsiConsole.WriteLine(report.StatusMessage);
                        lastMessage = report.StatusMessage;
                    }

                    // Update progress value
                    if (report.TotalPackages > 0)
                    {
                        progressTask.Value = (double)report.CompletedPackages / report.TotalPackages * 100;
                    }
                });

                summary = await installationAction(progress);
                
                progressTask.Value = 100;
                AnsiConsole.MarkupLine("[green]Installation complete![/]");
                progressTask.StopTask();
            });

        return summary;
    }

    /// <summary>
    /// Displays help and documentation
    /// </summary>
    public void ShowHelp()
    {
        AnsiConsole.Clear();
        ShowBanner();

        var helpContent = new Markup(
            $"[bold blue]{IconProvider.GetHeaderText("discover")}[/]\n\n" +
            "[green]Navigation:[/]\n" +
            "• Use arrow keys to navigate menus\n" +
            "• Press SPACE to select/deselect items\n" +
            "• Press ENTER to confirm selections\n" +
            "• Press ESC to go back\n\n" +
            "[green]Features:[/]\n" +
            $"• [blue]{IconProvider.DISCOVER} Discover & Install Workloads[/] - Find and install workstation configurations\n" +
            $"• [blue]{IconProvider.VALIDATE} Validate Installation[/] - Check current system compliance\n" +
            $"• [blue]{IconProvider.REPORT} Generate Reports[/] - Create compliance and audit reports\n\n" +
            "[green]Workload Selection:[/]\n" +
            "• Multi-select interface with real-time feedback\n" +
            "• Package consolidation and conflict detection\n" +
            "• Time estimation and dependency resolution\n" +
            "• Live progress tracking during execution\n\n" +
            "[green]Execution Modes:[/]\n" +
            $"• [blue]{IconProvider.INSTALL} Preview Mode[/] - View what would be installed without making changes\n"
        );

        var panel = new Panel(helpContent)
            .Header(IconProvider.GetHeaderText("help"))
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Blue);

        AnsiConsole.Write(panel);
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("Press any key to continue...");
        Console.ReadKey(true);
    }

    /// <summary>
    /// Shows quick tips to the user
    /// </summary>
    public void ShowQuickTips()
    {
        var tips = new string[]
        {
            "Use the multi-select interface to choose multiple workloads at once",
            "Check estimated installation times before proceeding",
            "Validation mode is useful for checking system compliance without making changes",
            "Both mode (install + validate) is recommended for complete setup verification",
            "Review the execution plan carefully before confirming installation"
        };

        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine($"[bold yellow]{IconProvider.TIP} Quick Tips:[/]");
        
        foreach (var tip in tips)
        {
            AnsiConsole.MarkupLine($"[yellow]  • {tip}[/]");
        }
        AnsiConsole.WriteLine();
    }

    /// <summary>
    /// Shows troubleshooting information
    /// </summary>
    public void ShowTroubleshooting()
    {
        var troubleshootingContent =
            "[green]Common Issues:[/]\n" +
            "• If workloads don't appear, check the workloads directory exists\n" +
            "• Ensure YAML files are properly formatted\n" +
            "• Run as administrator if package installations fail\n" +
            "• Check network connectivity for package downloads\n\n" +
            "[green]Getting Help:[/]\n" +
            "• Review workload YAML files for configuration details\n" +
            "• Check execution logs for detailed error information\n" +
            "• Verify system requirements are met\n";

        var panel = new Panel(new Markup(troubleshootingContent))
            .Header($"{IconProvider.HELP} Troubleshooting")
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Blue);

        AnsiConsole.Write(panel);
    }

    /// <summary>
    /// Shows workload information and structure details
    /// </summary>
    public void ShowWorkloadInfo()
    {
        var workloadInfo =
            "[green]Workload Structure:[/]\n" +
            "• Each workload is defined by a workload.yaml file\n" +
            "• Workloads contain packages, scripts, tests, and files\n" +
            "• Packages are installed using appropriate package managers\n" +
            "• Scripts are executed to configure the environment\n" +
            "• Tests validate the installation was successful\n\n" +
            "[green]Directory Layout:[/]\n" +
            "• workloads/[name]/workload.yaml - Main configuration\n" +
            "• workloads/[name]/scripts/ - Setup scripts\n" +
            "• workloads/[name]/README.md - Documentation\n";

        var panel = new Panel(new Markup(workloadInfo))
            .Header($"{IconProvider.HELP} Workload Information")
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Blue);

        AnsiConsole.Write(panel);
    }

    public void DisplayFailureReport(InstallationFailureReport report)
    {
        if (report.TotalFailures == 0) return;
        
        AnsiConsole.WriteLine();
        AnsiConsole.Write(new Rule("[red]Installation Failures[/]").RuleStyle("red"));
        AnsiConsole.WriteLine();
        
        foreach (var failure in report.Failures)
        {
            var panel = new Panel(FormatFailureDetails(failure))
                .Header($"[red]{failure.PackageName}[/] ({failure.PackageManager})")
                .Border(BoxBorder.Rounded)
                .BorderColor(Color.Red)
                .Padding(1, 0);
            
            AnsiConsole.Write(panel);
            AnsiConsole.WriteLine();
        }
    }

    private string FormatFailureDetails(PackageFailureDetail failure)
    {
        var sb = new StringBuilder();
        sb.AppendLine($"[yellow]Status:[/] {failure.Status}");
        sb.AppendLine($"[yellow]Exit Code:[/] {failure.ExitCode}");
        sb.AppendLine($"[yellow]Duration:[/] {failure.Duration.TotalSeconds:F1}s");
        
        if (failure.Errors.Count > 0)
        {
            sb.AppendLine();
            sb.AppendLine("[yellow]Errors:[/]");
            foreach (var error in failure.Errors.Take(5))
            {
                sb.AppendLine($"  [red]• {Markup.Escape(error)}[/]");
            }
        }
        
        if (failure.StandardError.Count > 0)
        {
            sb.AppendLine();
            sb.AppendLine("[yellow]Output (last 5 lines):[/]");
            foreach (var line in failure.StandardError.TakeLast(5))
            {
                sb.AppendLine($"  [dim]{Markup.Escape(line)}[/]");
            }
        }
        
        sb.AppendLine();
        sb.AppendLine($"[yellow]Command:[/]");
        sb.AppendLine($"  [dim]{Markup.Escape(failure.CommandExecuted)}[/]");
        
        return sb.ToString();
    }

}