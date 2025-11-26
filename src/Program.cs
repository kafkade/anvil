using System;
using System.Threading.Tasks;
using Winforge.Core;
using Winforge.Interfaces;
using Winforge.Services;
using Winforge.Services.Execution;
using Winforge.UI;
using Winforge.Services.Logging;
using Spectre.Console;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Winforge;

/// <summary>
/// Main entry point for the Winforge console application.
/// Handles command line argument parsing and application initialization.
/// </summary>
class Program
{
    /// <summary>
    /// Application version information
    /// </summary>
    private static readonly Version AppVersion = new(2, 0, 0, 0);
    
    /// <summary>
    /// Main application entry point
    /// </summary>
    /// <param name="args">Command line arguments</param>
    /// <returns>Exit code (0 = success, non-zero = error)</returns>
    static async Task<int> Main(string[] args)
    {
        try
        {
            // Set console title
            Console.Title = "Winforge - Workstation Setup & Management System";
            
            // Parse command line arguments
            var action = args.Length > 0 ? args[0].ToLower() : "interactive";
            
            // Handle version and help commands without initializing full UI
            switch (action)
            {
                case "version":
                case "-version":
                case "--version":
                case "-v":
                    ShowVersion();
                    return 0;
                    
                case "help":
                case "-help":
                case "--help":
                case "-h":
                    ShowHelpSummary();
                    return 0;
                // TODO: Re-enable test-tracker command after implementing simulation
                // case "test-tracker":
                //     await TestSimplePackageTracker.RunTestAsync();
                //     return 0;
            }
            
            // Setup Dependency Injection
            var services = new ServiceCollection();
            ConfigureServices(services);
            var serviceProvider = services.BuildServiceProvider();

            // Initialize and run the full application
            using var app = serviceProvider.GetRequiredService<WinforgeApplication>();
            return await app.RunAsync(args);
        }
        catch (OperationCanceledException)
        {
            AnsiConsole.MarkupLine("[yellow]Operation cancelled by user.[/]");
            return 130; // Standard SIGINT exit code
        }
        catch (Exception ex)
        {
            AnsiConsole.MarkupLine("[red]Unhandled application error:[/]");
            AnsiConsole.WriteException(ex);
            AnsiConsole.MarkupLine("[yellow]Please report this error with the stack trace above.[/]");
            return 1;
        }
        finally
        {
            // Ensure proper cleanup
            try
            {
                GC.Collect();
                GC.WaitForPendingFinalizers();
            }
            catch
            {
                // Ignore cleanup errors
            }
        }
    }

    /// <summary>
    /// Configures the dependency injection container.
    /// </summary>
    /// <param name="services">The service collection to configure</param>
    private static void ConfigureServices(IServiceCollection services)
    {
        // Logging
        services.AddLogging(configure =>
        {
            configure.AddConsole();
            configure.SetMinimumLevel(LogLevel.Information);
        });

        // Services
        services.AddSingleton<WingetPackageInstaller>();
        services.AddSingleton<IPackageInstaller>(sp => sp.GetRequiredService<WingetPackageInstaller>());
        services.AddSingleton<WorkloadManager>();
        services.AddSingleton<PackageConsolidator>();
        services.AddSingleton<PackageInstallationOrchestrator>();
        services.AddSingleton<StructuredLogger>(sp => new StructuredLogger(sp.GetRequiredService<ILogger<StructuredLogger>>()));
        
        // UI
        services.AddSingleton<WinforgeUI>();
        
        // Core Application
        services.AddSingleton<WinforgeApplication>();
    }
    
    /// <summary>
    /// Shows version information
    /// </summary>
    private static void ShowVersion()
    {
        AnsiConsole.MarkupLine("[blue]Winforge Workstation Setup & Management System[/]");
        AnsiConsole.MarkupLine($"[grey]Version {AppVersion}[/]");
        AnsiConsole.MarkupLine("[grey]Built with .NET 8.0 for cross-platform deployment[/]");
        AnsiConsole.WriteLine();
        
        // Show build information
        var assembly = System.Reflection.Assembly.GetExecutingAssembly();
        var buildDate = System.IO.File.GetCreationTime(AppContext.BaseDirectory);
        AnsiConsole.MarkupLine($"[dim]Build Date: {buildDate:yyyy-MM-dd HH:mm:ss}[/]");
        AnsiConsole.MarkupLine($"[dim]Runtime: {System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription}[/]");
        AnsiConsole.MarkupLine($"[dim]Platform: {System.Runtime.InteropServices.RuntimeInformation.OSDescription}[/]");
    }
    
    /// <summary>
    /// Shows a brief help summary
    /// </summary>
    private static void ShowHelpSummary()
    {
        AnsiConsole.MarkupLine("[blue]Winforge Workstation Setup & Management System[/]");
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("[green]Usage:[/] winforge [[command]] [[options]]");
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("[green]Commands:[/]");
        AnsiConsole.MarkupLine("  [blue]interactive[/]       Start interactive mode (default)");
        AnsiConsole.MarkupLine("  [blue]help[/]              Show detailed help and documentation");
        AnsiConsole.MarkupLine("  [blue]version[/]           Show version information");
        // AnsiConsole.MarkupLine("  [blue]test-tracker[/]      Test the simple package tracker");
        AnsiConsole.MarkupLine("  [blue]quick-tips[/]        Show quick usage tips");
        AnsiConsole.MarkupLine("  [blue]troubleshoot[/]      Show troubleshooting information");
        AnsiConsole.MarkupLine("  [blue]workload-info[/]     Show workload structure information");
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("[green]Options:[/]");
        AnsiConsole.MarkupLine("  [blue]-h, --help[/]        Show this help message");
        AnsiConsole.MarkupLine("  [blue]-v, --version[/]     Show version information");
        AnsiConsole.WriteLine();
        AnsiConsole.MarkupLine("[yellow]Run 'winforge' without arguments to start interactive mode.[/]");
    }
}