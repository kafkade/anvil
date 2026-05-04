//! Git provider for managing remote workload sources
//!
//! This module provides git operations (clone, pull, status) using
//! `std::process::Command` to shell out to the `git` CLI.

use std::path::Path;
use std::process::Command;

use thiserror::Error;

/// Errors that can occur during git operations
#[derive(Error, Debug)]
pub enum GitError {
    #[error("git is not installed or not in PATH")]
    GitNotFound,

    #[error("git clone failed: {0}")]
    CloneFailed(String),

    #[error("git fetch failed: {0}")]
    FetchFailed(String),

    #[error("git pull failed: {0}")]
    PullFailed(String),

    #[error("repository has local modifications")]
    DirtyWorkingTree,

    #[error("git command failed: {0}")]
    CommandFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Git operations provider
pub struct GitProvider;

impl GitProvider {
    /// Check if git is available on the system
    pub fn is_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Clone a git repository
    ///
    /// Uses shallow clone (`--depth 1`) for faster initial setup.
    /// If `git_ref` is provided, checks out that branch/tag after cloning.
    pub fn clone(url: &str, dest: &Path, git_ref: Option<&str>) -> Result<(), GitError> {
        if !Self::is_available() {
            return Err(GitError::GitNotFound);
        }

        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth").arg("1");

        if let Some(ref_name) = git_ref {
            cmd.arg("--branch").arg(ref_name);
        }

        cmd.arg(url).arg(dest);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CloneFailed(stderr));
        }

        Ok(())
    }

    /// Pull the latest changes for a repository
    ///
    /// For branches: runs `git pull`.
    /// For tags/detached HEAD: runs `git fetch` + `git checkout <ref>`.
    pub fn sync(repo_path: &Path, git_ref: Option<&str>) -> Result<(), GitError> {
        if !Self::is_available() {
            return Err(GitError::GitNotFound);
        }

        // Check for dirty working tree
        if Self::is_dirty(repo_path)? {
            return Err(GitError::DirtyWorkingTree);
        }

        // Fetch first
        let fetch_output = Command::new("git")
            .args(["fetch", "--depth", "1"])
            .current_dir(repo_path)
            .output()?;

        if !fetch_output.status.success() {
            let stderr = String::from_utf8_lossy(&fetch_output.stderr).to_string();
            return Err(GitError::FetchFailed(stderr));
        }

        if let Some(ref_name) = git_ref {
            // For a specific ref, fetch and checkout
            let checkout_output = Command::new("git")
                .args(["checkout", ref_name])
                .current_dir(repo_path)
                .output()?;

            if !checkout_output.status.success() {
                // Try as FETCH_HEAD for tags
                let pull_output = Command::new("git")
                    .args(["pull", "origin", ref_name])
                    .current_dir(repo_path)
                    .output()?;

                if !pull_output.status.success() {
                    let stderr = String::from_utf8_lossy(&pull_output.stderr).to_string();
                    return Err(GitError::PullFailed(stderr));
                }
            }
        } else {
            // Default branch — just pull
            let pull_output = Command::new("git")
                .args(["pull"])
                .current_dir(repo_path)
                .output()?;

            if !pull_output.status.success() {
                let stderr = String::from_utf8_lossy(&pull_output.stderr).to_string();
                return Err(GitError::PullFailed(stderr));
            }
        }

        Ok(())
    }

    /// Check if the working tree has uncommitted changes
    pub fn is_dirty(repo_path: &Path) -> Result<bool, GitError> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed(stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    /// Get the current HEAD ref (branch name or commit hash)
    pub fn current_ref(repo_path: &Path) -> Result<String, GitError> {
        // Try symbolic-ref first (for branches)
        let output = Command::new("git")
            .args(["symbolic-ref", "--short", "HEAD"])
            .current_dir(repo_path)
            .output()?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(branch);
        }

        // Fall back to rev-parse (for detached HEAD / tags)
        let output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(repo_path)
            .output()?;

        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(hash)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(GitError::CommandFailed(stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_is_available() {
        // git should be available in the development environment
        assert!(GitProvider::is_available());
    }

    #[test]
    fn test_clone_invalid_url() {
        let temp = tempfile::TempDir::new().unwrap();
        let dest = temp.path().join("nonexistent-repo");
        let result =
            GitProvider::clone("https://example.invalid/nonexistent/repo.git", &dest, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_dirty_not_a_repo() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = GitProvider::is_dirty(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_current_ref_not_a_repo() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = GitProvider::current_ref(temp.path());
        assert!(result.is_err());
    }
}
