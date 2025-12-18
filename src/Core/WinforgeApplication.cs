using Spectre.Console;
using Winforge.Services;
using Winforge.Services.Execution;
using Winforge.Services.Logging;
using Winforge.UI;

namespace Winforge.Core;

/// <summary>
/// Main application coordinator for the Winforge console application.
/// Handles initialization, command routing, and cleanup.
/// </summary>
public class WinforgeApplication : IDisposable
{
    private readonly WinforgeUI _ui;
    private readonly PackageConsolidator _packageConsolidator;
    private readonly PackageInstallationOrchestrator _installationOrchestrator;
    private readonly WorkloadManager _workloadManager;
    private readonly StructuredLogger _logger;
    private bool _disposed = false;
    
    /// <summary>
    /// Initializes a new instance of the WinforgeApplication class.
    /// </summary>
    /// <param name="ui">The UI coordinator</param>
    /// <param name="packageConsolidator">Service for consolidating packages</param>
    /// <param name="installationOrchestrator">Service for orchestrating installations</param>
    /// <param name="workloadManager">Service for managing workloads</param>
    /// <param name="logger">Structured logger</param>
    public WinforgeApplication(
        WinforgeUI ui,
        PackageConsolidator packageConsolidator,
        PackageInstallationOrchestrator installationOrchestrator,
        WorkloadManager workloadManager,
        StructuredLogger logger)
    {
        _ui = ui ?? throw new ArgumentNullException(nameof(ui));
        _packageConsolidator = packageConsolidator ?? throw new ArgumentNullException(nameof(packageConsolidator));
        _installationOrchestrator = installationOrchestrator ?? throw new ArgumentNullException(nameof(installationOrchestrator));
        _workloadManager = workloadManager ?? throw new ArgumentNullException(nameof(workloadManager));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }
    
    /// <summary>
    /// Runs the application with the specified command line arguments.
    /// </summary>
    /// <param name="args">Command line arguments</param>
    /// <returns>Exit code (0 = success, non-zero = error)</returns>
    public async Task<int> RunAsync(string[] args)
    {
        try
        {
            // Parse command line arguments
            var action = args.Length > 0 ? args[0].ToLower() : "interactive";
            
            return action switch
            {
                "" or "interactive" or "-interactive" or "--interactive" => await RunInteractiveMode(),
                "help" or "-help" or "--help" or "-h" => ShowHelp(),
                "quick-tips" or "--quick-tips" => ShowQuickTips(),
                "troubleshoot" or "--troubleshoot" => ShowTroubleshooting(),
                "workload-info" or "--workload-info" => ShowWorkloadInfo(),
                _ => ShowUnknownCommand(action)
            };
        }
        catch (OperationCanceledException)
        {
            AnsiConsole.MarkupLine("[yellow]Operation cancelled by user.[/]");
            return 130; // Standard SIGINT exit code
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Unexpected error during execution");
            AnsiConsole.WriteException(ex);
            AnsiConsole.MarkupLine("[red]An unexpected error occurred during execution.[/]");
            AnsiConsole.MarkupLine("[yellow]Please report this error with the stack trace above.[/]");
            return 1;
        }
    }
    
    /// <summary>
    /// Runs the interactive mode of the application.
    /// </summary>
    /// <returns>Exit code</returns>
    private async Task<int> RunInteractiveMode()
    {
        _ui.ShowBanner();

        while (true)
        {
            var choice = _ui.ShowMainMenu();
            
            switch (choice)
            {
                case "discover":
                    await RunDiscoveryAndInstallationWorkflow();
                    break;
                case "validate":
                    await _ui.RunValidationMode();
                    break;
                case "report":
                    await _ui.GenerateReport();
                    break;
                case "help":
                    ShowHelp();
                    break;
                case "exit":
                    AnsiConsole.MarkupLine("[green]Thank you for using Winforge![/]");
                    return 0;
            }

            if (!AnsiConsole.Confirm("Return to main menu?"))
                break;
        }
        return 0;
    }

    /// <summary>
    /// Runs the workflow for discovering workloads and installing packages.
    /// </summary>
    private async Task RunDiscoveryAndInstallationWorkflow()
    {
        try
        {
            // 1. Select Workloads
            var selectedWorkloads = await _ui.SelectWorkloadsAsync();
            if (!selectedWorkloads.Any())
            {
                return;
            }

            // 2. Consolidate Packages
            AnsiConsole.MarkupLine("[blue]Consolidating packages from selected workloads...[/]");
            var consolidatedPackages = await _packageConsolidator.ConsolidateFromWorkloadsAsync(selectedWorkloads, _workloadManager);

            // 3. Display Consolidated List
            _ui.DisplayConsolidatedPackageList(consolidatedPackages);

            // 4. Get Confirmation
            if (!_ui.ConfirmPackageInstallation(consolidatedPackages))
            {
                AnsiConsole.MarkupLine("[yellow]Installation cancelled by user.[/]");
                return;
            }

            // 5. Install Packages
            using var cts = new CancellationTokenSource();
            
            // Handle Ctrl+C to cancel installation
            Console.CancelKeyPress += (s, e) =>
            {
                e.Cancel = true;
                cts.Cancel();
                AnsiConsole.MarkupLine("[red]Cancellation requested...[/]");
            };

            try
            {
                var summary = await _ui.RunInstallationWithProgress(async (progress) =>
                {
                    return await _installationOrchestrator.InstallAllPackagesAsync(
                        consolidatedPackages,
                        progress,
                        cts.Token);
                });

                // 6. Display Summary
                _ui.DisplayInstallationSummary(summary);
            }
            catch (OperationCanceledException)
            {
                AnsiConsole.MarkupLine("[yellow]Installation was cancelled.[/]");
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error during installation workflow");
            AnsiConsole.MarkupLine($"[red]Error: {ex.Message}[/]");
        }
    }
    
    /// <summary>
    /// Shows the detailed help information.
    /// </summary>
    /// <returns>Exit code</returns>
    private int ShowHelp()
    {
        _ui.ShowHelp();
        return 0;
    }
    
    /// <summary>
    /// Shows quick tips for using the application.
    /// </summary>
    /// <returns>Exit code</returns>
    private int ShowQuickTips()
    {
        _ui.ShowQuickTips();
        return 0;
    }
    
    /// <summary>
    /// Shows troubleshooting information.
    /// </summary>
    /// <returns>Exit code</returns>
    private int ShowTroubleshooting()
    {
        _ui.ShowTroubleshooting();
        return 0;
    }
    
    /// <summary>
    /// Shows workload information and structure details.
    /// </summary>
    /// <returns>Exit code</returns>
    private int ShowWorkloadInfo()
    {
        _ui.ShowWorkloadInfo();
        return 0;
    }
    
    /// <summary>
    /// Handles unknown command errors.
    /// </summary>
    /// <param name="command">The unknown command</param>
    /// <returns>Exit code</returns>
    private static int ShowUnknownCommand(string command)
    {
        AnsiConsole.MarkupLine($"[red]Unknown command: {command}[/]");
        AnsiConsole.MarkupLine("[yellow]Use 'winforge help' to see available commands.[/]");
        return 1;
    }
    
    /// <summary>
    /// Disposes of resources used by the application.
    /// </summary>
    public void Dispose()
    {
        if (!_disposed)
        {
            // No cleanup needed - WinforgeUI no longer implements IDisposable
            _disposed = true;
        }
    }
}