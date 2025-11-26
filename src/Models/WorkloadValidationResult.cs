using System.Collections.Generic;

namespace Winforge.Models;

/// <summary>
/// Represents the result of validating a workload before execution.
/// Provides validation status and detailed issues found during validation.
/// </summary>
public class WorkloadValidationResult
{
    /// <summary>
    /// Gets or sets a value indicating whether the workload validation passed.
    /// </summary>
    public bool IsValid { get; set; }

    /// <summary>
    /// Gets or sets the list of validation issues found.
    /// </summary>
    public List<string> Issues { get; set; } = new();

    /// <summary>
    /// Gets or sets the list of validation warnings (non-blocking issues).
    /// </summary>
    public List<string> Warnings { get; set; } = new();

    /// <summary>
    /// Gets or sets detailed validation information for different components.
    /// </summary>
    public List<string> ValidationDetails { get; set; } = new();

    /// <summary>
    /// Gets the total number of issues found.
    /// </summary>
    public int TotalIssues => Issues.Count;

    /// <summary>
    /// Gets the total number of warnings found.
    /// </summary>
    public int TotalWarnings => Warnings.Count;

    /// <summary>
    /// Gets a value indicating whether there are any warnings.
    /// </summary>
    public bool HasWarnings => Warnings.Count > 0;

    /// <summary>
    /// Gets a summary message of the validation result.
    /// </summary>
    public string GetSummaryMessage()
    {
        if (IsValid)
        {
            if (HasWarnings)
            {
                return $"Validation passed with {TotalWarnings} warning(s)";
            }
            else
            {
                return "Validation passed successfully";
            }
        }
        else
        {
            return $"Validation failed with {TotalIssues} issue(s)" + 
                   (HasWarnings ? $" and {TotalWarnings} warning(s)" : "");
        }
    }
}