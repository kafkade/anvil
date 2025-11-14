using System;
using System.Threading.Tasks;
using Spectre.Console;
using Winforge.UI;

namespace Winforge.Core;

/// <summary>
/// Main application coordinator for the Winforge console application.
/// Handles initialization, command routing, and cleanup.
/// </summary>
public class WinforgeApplication : IDisposable
{
    private readonly WinforgeUI _ui;
    private bool _disposed = false;
    
    /// <summary>
    /// Initializes a new instance of the WinforgeApplication class.
    /// </summary>
    public WinforgeApplication()
    {
        try
        {
            _ui = new WinforgeUI();
        }
        catch (Exception ex)
        {
            AnsiConsole.WriteException(ex);
            throw new InvalidOperationException("Failed to initialize Winforge application", ex);
        }
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
        await _ui.RunInteractiveMode();
        return 0;
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
            try
            {
                _ui?.Dispose();
            }
            catch (Exception ex)
            {
                AnsiConsole.MarkupLine($"[yellow]Warning: Error during cleanup: {ex.Message}[/]");
            }
            _disposed = true;
        }
    }
}