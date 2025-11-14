namespace Winforge.UI;

/// <summary>
/// Centralized icon management system that provides text-based alternatives to emoji usage.
/// All UI symbols and icons used throughout the application are defined here for consistency.
/// </summary>
public static class IconProvider
{
    // Navigation and Menu Icons
    /// <summary>
    /// Icon for workload discovery and installation functionality
    /// </summary>
    public const string DISCOVER = "[[DISCOVER]]";
    
    /// <summary>
    /// Icon for validation and compliance checking functionality
    /// </summary>
    public const string VALIDATE = "[[VALIDATE]]";
    
    /// <summary>
    /// Icon for reporting and analytics functionality
    /// </summary>
    public const string REPORT = "[[REPORT]]";
    
    /// <summary>
    /// Icon for help and documentation functionality
    /// </summary>
    public const string HELP = "[[HELP]]";
    
    /// <summary>
    /// Icon for exit functionality
    /// </summary>
    public const string EXIT = "[[EXIT]]";

    // Status and Progress Icons
    /// <summary>
    /// Icon indicating success or valid status
    /// </summary>
    public const string SUCCESS = "[[✓]]";
    
    /// <summary>
    /// Icon indicating failure or invalid status
    /// </summary>
    public const string FAILURE = "[[✗]]";
    
    /// <summary>
    /// Icon for package installation operations
    /// </summary>
    public const string INSTALL = "[[INSTALL]]";
    
    /// <summary>
    /// Icon indicating both installation and validation modes
    /// </summary>
    public const string BOTH = "[[BOTH]]";
    
    /// <summary>
    /// Icon for celebration or completion
    /// </summary>
    public const string CELEBRATION = "[[SUCCESS]]";
    
    /// <summary>
    /// Icon for tips and recommendations
    /// </summary>
    public const string TIP = "[[TIP]]";

    // Execution and Display Icons
    /// <summary>
    /// Icon for execution plans and planning
    /// </summary>
    public const string PLAN = "[[PLAN]]";
    
    /// <summary>
    /// Icon for documentation and help content
    /// </summary>
    public const string DOCS = "[[DOCS]]";
    
    /// <summary>
    /// Icon for debugging and troubleshooting
    /// </summary>
    public const string DEBUG = "[[DEBUG]]";
    
    /// <summary>
    /// Icon for results and analytics
    /// </summary>
    public const string RESULTS = "[[RESULTS]]";
    
    /// <summary>
    /// Icon for complete operations
    /// </summary>
    public const string COMPLETE = "[[COMPLETE]]";

    // Menu Display Methods
    /// <summary>
    /// Gets the display text for menu items with appropriate icons
    /// </summary>
    /// <param name="menuKey">The menu item key</param>
    /// <returns>Formatted display text with icon</returns>
    public static string GetMenuDisplay(string menuKey) => menuKey switch
    {
        "discover" => $"{DISCOVER} Discover & Install Workloads",
        "validate" => $"{VALIDATE} Validate Current Installation",
        "report" => $"{REPORT} Generate Compliance Report",
        "help" => $"{HELP} Show Help & Documentation",
        "exit" => $"{EXIT} Exit Winforge",
        _ => menuKey
    };

    /// <summary>
    /// Gets the display text for execution modes with appropriate icons
    /// </summary>
    /// <param name="executionMode">The execution mode key</param>
    /// <returns>Formatted display text with icon</returns>
    public static string GetExecutionModeDisplay(string executionMode) => executionMode switch
    {
        "install" => $"{INSTALL} Installation Mode - Install packages and run setup scripts",
        "validate" => $"{VALIDATE} Validation Mode - Run validation tests only",
        "both" => $"{BOTH} Both Modes - Install then validate (Recommended)",
        _ => executionMode
    };

    /// <summary>
    /// Gets the appropriate status icon based on validation state
    /// </summary>
    /// <param name="isValid">Whether the item is valid</param>
    /// <returns>Success or failure icon</returns>
    public static string GetStatusIcon(bool isValid) => isValid ? SUCCESS : FAILURE;

    /// <summary>
    /// Gets header text with appropriate icon for different sections
    /// </summary>
    /// <param name="headerType">The type of header</param>
    /// <returns>Header text with icon</returns>
    public static string GetHeaderText(string headerType) => headerType switch
    {
        "plan" => $"{PLAN} Execution Plan",
        "results" => $"{RESULTS} Results",
        "help" => $"{DOCS} Help & Documentation",
        "discover" => $"{DISCOVER} Winforge Interactive CLI Help",
        _ => headerType
    };
}