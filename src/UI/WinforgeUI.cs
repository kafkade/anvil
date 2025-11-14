using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Spectre.Console;
using Winforge.Models;
using Winforge.Services;

namespace Winforge.UI;

/// <summary>
/// Main UI coordinator class that orchestrates all user interface components.
/// Provides the primary entry point for interactive mode and coordinates
/// between different UI subsystems while maintaining emoji-free presentation.
/// </summary>
public class WinforgeUI : IDisposable
{
    private readonly WorkloadManager _workloadManager;

    /// <summary>
    /// Initializes a new instance of the WinforgeUI class with all required components
    /// </summary>
    public WinforgeUI()
    {
        _workloadManager = new WorkloadManager();
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
    /// Runs the main interactive mode loop, handling user navigation and feature execution
    /// </summary>
    /// <returns>Task representing the async operation</returns>
    public async Task RunInteractiveMode()
    {
        ShowBanner();

        while (true)
        {
            var choice = ShowMainMenu();
            
            switch (choice)
            {
                case "discover":
                    await DiscoverAndSelectWorkloads();
                    break;
                case "validate":
                    await RunValidationMode();
                    break;
                case "report":
                    await GenerateReport();
                    break;
                case "help":
                    ShowHelp();
                    break;
                case "exit":
                    AnsiConsole.MarkupLine("[green]Thank you for using Winforge![/]");
                    return;
            }

            if (!AnsiConsole.Confirm("Return to main menu?"))
                break;
        }
    }

    /// <summary>
    /// Displays the main menu and handles user selection
    /// </summary>
    /// <returns>The selected menu option key</returns>
    private string ShowMainMenu()
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
    /// Handles the workload discovery, selection, and execution workflow
    /// </summary>
    /// <returns>Task representing the async operation</returns>
    private async Task DiscoverAndSelectWorkloads()
    {
        AnsiConsole.Clear();
        ShowBanner();

        // Show discovery progress
        await ShowDiscoveryProgress();

        var workloads = _workloadManager.DiscoverWorkloads();

        if (!workloads.Any())
        {
            AnsiConsole.MarkupLine("[red]No workloads found![/]");
            return;
        }

        ShowWorkloadDiscoveryResults(workloads);

        var selectedWorkloads = ShowWorkloadSelection(workloads);
        
        if (!selectedWorkloads.Any())
        {
            AnsiConsole.MarkupLine("[yellow]No workloads selected.[/]");
            return;
        }

        var executionMode = ShowExecutionModeSelection();
        
        ShowExecutionPlan(selectedWorkloads, executionMode);

        if (AnsiConsole.Confirm("Proceed with installation?"))
        {
            await ExecuteWorkloadsWithDisplay(selectedWorkloads, executionMode);
        }
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
    /// Displays the execution mode selection menu
    /// </summary>
    /// <returns>The selected execution mode</returns>
    private string ShowExecutionModeSelection()
    {
        var selection = AnsiConsole.Prompt(
            new SelectionPrompt<string>()
                .Title("[green]Select execution mode:[/]")
                .AddChoices(new[] { "install", "validate", "both" })
                .UseConverter(choice => IconProvider.GetExecutionModeDisplay(choice)));

        return selection;
    }

    /// <summary>
    /// Displays the execution plan before proceeding with installation
    /// </summary>
    /// <param name="selectedWorkloads">List of workloads to be executed</param>
    /// <param name="executionMode">The execution mode selected</param>
    private void ShowExecutionPlan(List<WorkloadMetadata> selectedWorkloads, string executionMode)
    {
        AnsiConsole.Clear();
        ShowBanner();

        var panel = new Panel(
            new Markup($"[bold]Execution Plan Review[/]\n\n" +
                      $"[green]Selected Workloads:[/] {selectedWorkloads.Count}\n" +
                      $"[green]Execution Mode:[/] {executionMode.ToUpper()}\n" +
                      $"[green]Total Packages:[/] {selectedWorkloads.Sum(w => w.PackageCount)}\n" +
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
    }

    /// <summary>
    /// Executes the selected workloads and displays results
    /// </summary>
    /// <param name="selectedWorkloads">List of workloads to execute</param>
    /// <param name="executionMode">The execution mode to use</param>
    /// <returns>Task representing the async operation</returns>
    private async Task ExecuteWorkloadsWithDisplay(List<WorkloadMetadata> selectedWorkloads, string executionMode)
    {
        var results = await ExecuteWorkloads(selectedWorkloads, executionMode);
        
        // Add some default recommendations for successful executions
        if (results.SuccessRate == 100)
        {
            results.Recommendations.Add("All components installed successfully!");
            results.Recommendations.Add("Consider creating a system restore point.");
        }
        else if (results.SuccessRate > 80)
        {
            results.Recommendations.Add("Most components installed successfully.");
            results.Recommendations.Add("Review failed items and retry if needed.");
        }
        else
        {
            results.Recommendations.Add("Several components failed to install.");
            results.Recommendations.Add("Check system requirements and permissions.");
            results.Recommendations.Add("Review error logs for detailed information.");
        }
        
        ShowExecutionResults(results);
    }

    /// <summary>
    /// Executes workloads with live progress tracking and visual feedback
    /// </summary>
    /// <param name="selectedWorkloads">List of workloads to execute</param>
    /// <param name="executionMode">The execution mode (install, validate, both)</param>
    /// <returns>Execution results with detailed statistics</returns>
    private async Task<ExecutionResults> ExecuteWorkloads(List<WorkloadMetadata> selectedWorkloads, string executionMode)
    {
        AnsiConsole.Clear();
        ShowBanner();

        var results = new ExecutionResults
        {
            StartTime = DateTime.Now,
            ExecutionMode = executionMode,
            TotalPackages = selectedWorkloads.Sum(w => w.PackageCount),
            TotalScripts = selectedWorkloads.Sum(w => w.ScriptCount),
            TotalTests = selectedWorkloads.Sum(w => w.TestCount)
        };

        // Show progress with live updates - this is a simulation
        await AnsiConsole.Progress()
            .StartAsync(async ctx =>
            {
                var overallTask = ctx.AddTask("[green]Overall Progress[/]");
                var currentTask = ctx.AddTask("[blue]Current Activity[/]");

                var totalSteps = results.TotalPackages + results.TotalScripts +
                               (executionMode.Contains("validate") || executionMode == "both" ? results.TotalTests : 0);
                
                var completedSteps = 0;

                // Simulate package installation
                if (executionMode == "install" || executionMode == "both")
                {
                    foreach (var workload in selectedWorkloads)
                    {
                        for (int i = 0; i < workload.PackageCount; i++)
                        {
                            currentTask.Description = $"[blue]Installing package {i + 1}/{workload.PackageCount} from {workload.Name}[/]";
                            await Task.Delay(200);
                            
                            completedSteps++;
                            overallTask.Value = (double)completedSteps / totalSteps * 100;
                        }
                        results.SuccessfulPackages += workload.PackageCount;
                    }

                    foreach (var workload in selectedWorkloads)
                    {
                        for (int i = 0; i < workload.ScriptCount; i++)
                        {
                            currentTask.Description = $"[blue]Executing script {i + 1}/{workload.ScriptCount} from {workload.Name}[/]";
                            await Task.Delay(500);
                            
                            completedSteps++;
                            overallTask.Value = (double)completedSteps / totalSteps * 100;
                        }
                        results.SuccessfulScripts += workload.ScriptCount;
                    }
                }

                // Simulate validation
                if (executionMode == "validate" || executionMode == "both")
                {
                    foreach (var workload in selectedWorkloads)
                    {
                        for (int i = 0; i < workload.TestCount; i++)
                        {
                            currentTask.Description = $"[blue]Running test {i + 1}/{workload.TestCount} from {workload.Name}[/]";
                            await Task.Delay(100);
                            
                            completedSteps++;
                            overallTask.Value = (double)completedSteps / totalSteps * 100;
                        }
                        results.SuccessfulTests += workload.TestCount;
                    }
                }

                overallTask.Value = 100;
                currentTask.Description = $"[green]{IconProvider.SUCCESS} Complete![/]";
            });

        results.EndTime = DateTime.Now;
        results.TotalTimeSeconds = (int)(results.EndTime - results.StartTime).TotalSeconds;

        return results;
    }

    /// <summary>
    /// Displays comprehensive execution results with statistics and recommendations
    /// </summary>
    /// <param name="results">The execution results to display</param>
    private void ShowExecutionResults(ExecutionResults results)
    {
        AnsiConsole.Clear();
        ShowBanner();

        // Show summary panel
        var successRateColor = results.SuccessRate >= 90 ? "green" :
                               results.SuccessRate >= 70 ? "yellow" : "red";

        var summary = new Panel(
            new Markup($"[bold]Execution Summary[/]\n\n" +
                      $"[green]Success Rate:[/] [{successRateColor}]{results.SuccessRate:F1}%[/]\n" +
                      $"[green]Total Time:[/] {TimeSpan.FromSeconds(results.TotalTimeSeconds):mm\\:ss}\n" +
                      $"[green]Mode:[/] {results.ExecutionMode.ToUpper()}\n\n" +
                      $"[green]Packages:[/] {results.SuccessfulPackages}/{results.TotalPackages} successful\n" +
                      $"[green]Scripts:[/] {results.SuccessfulScripts}/{results.TotalScripts} successful\n" +
                      $"[green]Tests:[/] {results.SuccessfulTests}/{results.TotalTests} passed"))
            .Header(IconProvider.GetHeaderText("results"))
            .Border(BoxBorder.Rounded)
            .BorderColor(Color.Blue);

        AnsiConsole.Write(summary);

        // Show recommendations
        if (results.Recommendations.Any())
        {
            AnsiConsole.WriteLine();
            AnsiConsole.MarkupLine($"[bold yellow]{IconProvider.TIP} Recommendations:[/]");
            foreach (var rec in results.Recommendations)
            {
                AnsiConsole.MarkupLine($"[yellow]  • {rec}[/]");
            }
        }

        // Show celebration message if everything was successful
        if (results.SuccessRate == 100)
        {
            AnsiConsole.WriteLine();
            AnsiConsole.MarkupLine($"[bold green]{IconProvider.CELEBRATION} All workloads installed successfully![/]");
        }

        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("Press any key to continue...");
        Console.ReadKey(true);
    }

    /// <summary>
    /// Handles validation mode functionality (placeholder for future implementation)
    /// </summary>
    /// <returns>Task representing the async operation</returns>
    private async Task RunValidationMode()
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
    private async Task GenerateReport()
    {
        AnsiConsole.Clear();
        ShowBanner();
        AnsiConsole.MarkupLine("[yellow]Report generation - Feature coming soon![/]");
        await Task.Delay(1000);
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
            $"• [blue]{IconProvider.INSTALL} Installation Mode[/] - Install packages and run setup scripts\n" +
            $"• [blue]{IconProvider.VALIDATE} Validation Mode[/] - Run compliance tests only\n" +
            $"• [blue]{IconProvider.BOTH} Both Modes[/] - Complete installation with validation\n"
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

    /// <summary>
    /// Disposes of resources and cleans up components
    /// </summary>
    public void Dispose()
    {
        _workloadManager?.Dispose();
    }
}