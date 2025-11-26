using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Winforge.Models;

namespace Winforge.Interfaces;

/// <summary>
/// Interface for file operations as part of workload execution.
/// Provides file deployment and configuration capabilities.
/// </summary>
public interface IFileOperationService
{
    /// <summary>
    /// Deploys a file from source to destination as specified in workload configuration.
    /// </summary>
    /// <param name="fileInfo">Information about the file to deploy</param>
    /// <param name="workloadDirectory">The workload directory containing source files</param>
    /// <param name="progress">Progress reporter for file operations</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The file operation result</returns>
    Task<FileOperationResult> DeployFileAsync(
        Models.FileInfo fileInfo,
        string workloadDirectory,
        IProgress<string>? progress = null,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Validates that source files exist and destinations are accessible.
    /// </summary>
    /// <param name="fileInfo">Information about the file to validate</param>
    /// <param name="workloadDirectory">The workload directory containing source files</param>
    /// <returns>Validation result for the file operation</returns>
    Task<FileValidationResult> ValidateFileOperationAsync(Models.FileInfo fileInfo, string workloadDirectory);

    /// <summary>
    /// Deploys multiple files with progress reporting.
    /// </summary>
    /// <param name="files">Collection of files to deploy</param>
    /// <param name="workloadDirectory">The workload directory containing source files</param>
    /// <param name="progress">Progress reporter for overall file operations</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>Collection of file operation results</returns>
    Task<List<FileOperationResult>> DeployFilesAsync(
        IEnumerable<Models.FileInfo> files,
        string workloadDirectory,
        IProgress<(int current, int total, string currentFile)>? progress = null,
        CancellationToken cancellationToken = default);

    /// <summary>
    /// Backs up existing files before deployment.
    /// </summary>
    /// <param name="destinationPath">The destination path to backup</param>
    /// <returns>The backup path if successful, null if backup failed</returns>
    Task<string?> BackupExistingFileAsync(string destinationPath);

    /// <summary>
    /// Expands environment variables in file paths.
    /// </summary>
    /// <param name="path">The path containing environment variables</param>
    /// <returns>The expanded path</returns>
    string ExpandEnvironmentVariables(string path);
}

/// <summary>
/// Represents the result of a file operation.
/// </summary>
public class FileOperationResult
{
    /// <summary>
    /// Gets or sets a value indicating whether the operation was successful.
    /// </summary>
    public bool Success { get; set; }

    /// <summary>
    /// Gets or sets the source file path.
    /// </summary>
    public string SourcePath { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the destination file path.
    /// </summary>
    public string DestinationPath { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the backup file path if a backup was created.
    /// </summary>
    public string? BackupPath { get; set; }

    /// <summary>
    /// Gets or sets the operation that was performed.
    /// </summary>
    public string Operation { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets any error messages.
    /// </summary>
    public List<string> Errors { get; set; } = new();

    /// <summary>
    /// Gets or sets any warning messages.
    /// </summary>
    public List<string> Warnings { get; set; } = new();

    /// <summary>
    /// Gets or sets the time taken for the operation.
    /// </summary>
    public TimeSpan Duration { get; set; }
}

/// <summary>
/// Represents the result of file operation validation.
/// </summary>
public class FileValidationResult
{
    /// <summary>
    /// Gets or sets a value indicating whether the file operation is valid.
    /// </summary>
    public bool IsValid { get; set; }

    /// <summary>
    /// Gets or sets the source file path.
    /// </summary>
    public string SourcePath { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets the destination file path.
    /// </summary>
    public string DestinationPath { get; set; } = string.Empty;

    /// <summary>
    /// Gets or sets a value indicating whether the source file exists.
    /// </summary>
    public bool SourceExists { get; set; }

    /// <summary>
    /// Gets or sets a value indicating whether the destination is accessible.
    /// </summary>
    public bool DestinationAccessible { get; set; }

    /// <summary>
    /// Gets or sets validation messages.
    /// </summary>
    public List<string> ValidationMessages { get; set; } = new();

    /// <summary>
    /// Gets or sets any warnings about the file operation.
    /// </summary>
    public List<string> Warnings { get; set; } = new();
}