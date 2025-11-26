using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Winforge.Interfaces;
using Winforge.Models;

namespace Winforge.Services.Execution;

/// <summary>
/// Service for file operations as part of workload execution.
/// Provides file deployment and configuration capabilities.
/// </summary>
public class FileOperationService : IFileOperationService
{
    private readonly ILogger<FileOperationService> _logger;

    /// <summary>
    /// Initializes a new instance of the FileOperationService class.
    /// </summary>
    /// <param name="logger">The logger instance</param>
    public FileOperationService(ILogger<FileOperationService> logger)
    {
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
    }

    /// <summary>
    /// Deploys a file from source to destination as specified in workload configuration.
    /// </summary>
    /// <param name="fileInfo">Information about the file to deploy</param>
    /// <param name="workloadDirectory">The workload directory containing source files</param>
    /// <param name="progress">Progress reporter for file operations</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>The file operation result</returns>
    public async Task<FileOperationResult> DeployFileAsync(
        Models.FileInfo fileInfo,
        string workloadDirectory,
        IProgress<string>? progress = null,
        CancellationToken cancellationToken = default)
    {
        if (fileInfo == null)
            throw new ArgumentNullException(nameof(fileInfo));
        if (string.IsNullOrEmpty(workloadDirectory))
            throw new ArgumentException("Workload directory cannot be null or empty", nameof(workloadDirectory));

        var startTime = DateTime.Now;
        var result = new FileOperationResult
        {
            SourcePath = fileInfo.source,
            DestinationPath = fileInfo.destination,
            Operation = "Deploy"
        };

        try
        {
            // Resolve paths
            var sourcePath = Path.Combine(workloadDirectory, fileInfo.source);
            var destinationPath = ExpandEnvironmentVariables(fileInfo.destination);

            result.SourcePath = sourcePath;
            result.DestinationPath = destinationPath;

            _logger.LogInformation("Deploying file from {SourcePath} to {DestinationPath}", 
                sourcePath, destinationPath);

            progress?.Report($"Deploying {Path.GetFileName(sourcePath)} to {Path.GetFileName(destinationPath)}");

            // Validate source file exists
            if (!File.Exists(sourcePath))
            {
                var error = $"Source file not found: {sourcePath}";
                _logger.LogError(error);
                result.Success = false;
                result.Errors.Add(error);
                result.Duration = DateTime.Now - startTime;
                return result;
            }

            // Ensure destination directory exists
            var destinationDir = Path.GetDirectoryName(destinationPath);
            if (!string.IsNullOrEmpty(destinationDir) && !Directory.Exists(destinationDir))
            {
                try
                {
                    Directory.CreateDirectory(destinationDir);
                    _logger.LogDebug("Created destination directory: {DestinationDir}", destinationDir);
                }
                catch (Exception ex)
                {
                    var error = $"Failed to create destination directory {destinationDir}: {ex.Message}";
                    _logger.LogError(ex, error);
                    result.Success = false;
                    result.Errors.Add(error);
                    result.Duration = DateTime.Now - startTime;
                    return result;
                }
            }

            // Check if destination file already exists and handle overwrite
            if (File.Exists(destinationPath))
            {
                if (!fileInfo.overwrite)
                {
                    var message = $"Destination file exists and overwrite is disabled: {destinationPath}";
                    _logger.LogInformation(message);
                    result.Success = true;
                    result.Warnings.Add(message);
                    result.Operation = "Skipped";
                    result.Duration = DateTime.Now - startTime;
                    return result;
                }

                // Create backup if overwriting
                var backupPath = await BackupExistingFileAsync(destinationPath);
                if (!string.IsNullOrEmpty(backupPath))
                {
                    result.BackupPath = backupPath;
                    _logger.LogDebug("Created backup at: {BackupPath}", backupPath);
                }
                else
                {
                    result.Warnings.Add("Could not create backup of existing file");
                }
            }

            // Copy the file
            progress?.Report($"Copying {Path.GetFileName(sourcePath)}...");
            
            await Task.Run(() =>
            {
                File.Copy(sourcePath, destinationPath, overwrite: true);
            }, cancellationToken);

            result.Success = true;
            result.Duration = DateTime.Now - startTime;

            _logger.LogInformation("File deployed successfully: {SourcePath} -> {DestinationPath} in {Duration}ms", 
                sourcePath, destinationPath, result.Duration.TotalMilliseconds);
            
            progress?.Report($"Successfully deployed {Path.GetFileName(sourcePath)}");
        }
        catch (OperationCanceledException)
        {
            _logger.LogInformation("File deployment cancelled: {SourcePath}", fileInfo.source);
            result.Success = false;
            result.Errors.Add("File deployment was cancelled");
            result.Duration = DateTime.Now - startTime;
            
            progress?.Report($"Cancelled deployment of {Path.GetFileName(fileInfo.source)}");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Exception during file deployment: {SourcePath} -> {DestinationPath}", 
                result.SourcePath, result.DestinationPath);
            
            result.Success = false;
            result.Errors.Add($"Exception during file deployment: {ex.Message}");
            result.Duration = DateTime.Now - startTime;
            
            progress?.Report($"Error deploying {Path.GetFileName(fileInfo.source)}: {ex.Message}");
        }

        return result;
    }

    /// <summary>
    /// Validates that source files exist and destinations are accessible.
    /// </summary>
    /// <param name="fileInfo">Information about the file to validate</param>
    /// <param name="workloadDirectory">The workload directory containing source files</param>
    /// <returns>Validation result for the file operation</returns>
    public async Task<FileValidationResult> ValidateFileOperationAsync(Models.FileInfo fileInfo, string workloadDirectory)
    {
        if (fileInfo == null || string.IsNullOrEmpty(workloadDirectory))
        {
            return new FileValidationResult
            {
                IsValid = false,
                ValidationMessages = { "Invalid file info or workload directory" }
            };
        }

        var result = new FileValidationResult();

        try
        {
            // Resolve paths
            var sourcePath = Path.Combine(workloadDirectory, fileInfo.source);
            var destinationPath = ExpandEnvironmentVariables(fileInfo.destination);

            result.SourcePath = sourcePath;
            result.DestinationPath = destinationPath;

            // Check source file exists
            result.SourceExists = File.Exists(sourcePath);
            if (!result.SourceExists)
            {
                result.ValidationMessages.Add($"Source file not found: {sourcePath}");
            }

            // Check destination accessibility
            var destinationDir = Path.GetDirectoryName(destinationPath);
            if (!string.IsNullOrEmpty(destinationDir))
            {
                try
                {
                    // Test if we can create the directory
                    if (!Directory.Exists(destinationDir))
                    {
                        var testDir = Path.Combine(destinationDir, "test_access_" + Guid.NewGuid());
                        Directory.CreateDirectory(testDir);
                        Directory.Delete(testDir);
                    }

                    result.DestinationAccessible = true;
                }
                catch (Exception ex)
                {
                    result.DestinationAccessible = false;
                    result.ValidationMessages.Add($"Destination directory not accessible: {ex.Message}");
                }
            }
            else
            {
                result.DestinationAccessible = false;
                result.ValidationMessages.Add("Invalid destination path");
            }

            // Check if destination file exists and warn about overwrite
            if (File.Exists(destinationPath))
            {
                if (fileInfo.overwrite)
                {
                    result.Warnings.Add($"Destination file exists and will be overwritten: {destinationPath}");
                }
                else
                {
                    result.Warnings.Add($"Destination file exists and overwrite is disabled: {destinationPath}");
                }
            }

            result.IsValid = result.SourceExists && result.DestinationAccessible;

            _logger.LogDebug("File operation validation: {IsValid} for {SourcePath} -> {DestinationPath}", 
                result.IsValid, sourcePath, destinationPath);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error validating file operation: {Source} -> {Destination}", 
                fileInfo.source, fileInfo.destination);
            
            result.IsValid = false;
            result.ValidationMessages.Add($"Validation error: {ex.Message}");
        }

        return result;
    }

    /// <summary>
    /// Deploys multiple files with progress reporting.
    /// </summary>
    /// <param name="files">Collection of files to deploy</param>
    /// <param name="workloadDirectory">The workload directory containing source files</param>
    /// <param name="progress">Progress reporter for overall file operations</param>
    /// <param name="cancellationToken">Cancellation token</param>
    /// <returns>Collection of file operation results</returns>
    public async Task<List<FileOperationResult>> DeployFilesAsync(
        IEnumerable<Models.FileInfo> files,
        string workloadDirectory,
        IProgress<(int current, int total, string currentFile)>? progress = null,
        CancellationToken cancellationToken = default)
    {
        if (files == null)
            throw new ArgumentNullException(nameof(files));

        var fileList = files.ToList();
        var results = new List<FileOperationResult>();

        _logger.LogInformation("Starting deployment of {FileCount} files", fileList.Count);

        for (int i = 0; i < fileList.Count; i++)
        {
            if (cancellationToken.IsCancellationRequested)
                break;

            var file = fileList[i];
            progress?.Report((i + 1, fileList.Count, file.source));

            var result = await DeployFileAsync(file, workloadDirectory, cancellationToken: cancellationToken);
            results.Add(result);

            if (!result.Success)
            {
                _logger.LogWarning("File deployment failed: {SourcePath} -> {DestinationPath}", 
                    result.SourcePath, result.DestinationPath);
            }
        }

        var successCount = results.Count(r => r.Success);
        _logger.LogInformation("File deployment completed: {SuccessCount}/{TotalCount} files deployed successfully",
            successCount, results.Count);

        return results;
    }

    /// <summary>
    /// Backs up existing files before deployment.
    /// </summary>
    /// <param name="destinationPath">The destination path to backup</param>
    /// <returns>The backup path if successful, null if backup failed</returns>
    public async Task<string?> BackupExistingFileAsync(string destinationPath)
    {
        try
        {
            if (!File.Exists(destinationPath))
                return null;

            var backupPath = $"{destinationPath}.backup.{DateTime.Now:yyyyMMdd_HHmmss}";
            
            await Task.Run(() =>
            {
                File.Copy(destinationPath, backupPath, overwrite: false);
            });

            _logger.LogDebug("Created backup: {DestinationPath} -> {BackupPath}", destinationPath, backupPath);
            return backupPath;
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to create backup for: {DestinationPath}", destinationPath);
            return null;
        }
    }

    /// <summary>
    /// Expands environment variables in file paths.
    /// </summary>
    /// <param name="path">The path containing environment variables</param>
    /// <returns>The expanded path</returns>
    public string ExpandEnvironmentVariables(string path)
    {
        if (string.IsNullOrEmpty(path))
            return path;

        try
        {
            return Environment.ExpandEnvironmentVariables(path);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Failed to expand environment variables in path: {Path}", path);
            return path;
        }
    }
}