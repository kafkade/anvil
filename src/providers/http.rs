//! HTTP provider for fetching remote resources
//!
//! This module provides HTTP fetching using the `curl` CLI,
//! consistent with the pattern of shelling out to `git` for git operations.

use std::process::Command;

use thiserror::Error;

/// Errors that can occur during HTTP operations
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("curl is not installed or not in PATH")]
    CurlNotFound,

    #[error("HTTP request failed: {0}")]
    FetchFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// HTTP provider using curl CLI
pub struct HttpProvider;

impl HttpProvider {
    /// Check if curl is available on the system
    pub fn is_available() -> bool {
        Command::new("curl")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Fetch a URL and return the response body as a string
    pub fn fetch_string(url: &str) -> Result<String, HttpError> {
        if !Self::is_available() {
            return Err(HttpError::CurlNotFound);
        }

        let output = Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--fail",
                "--location", // follow redirects
                "--max-time",
                "30",
                url,
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(HttpError::FetchFailed(stderr));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| HttpError::FetchFailed(format!("Invalid UTF-8 response: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curl_is_available() {
        // curl should be available on modern Windows, macOS, and Linux
        assert!(HttpProvider::is_available());
    }

    #[test]
    fn test_fetch_invalid_url() {
        let result = HttpProvider::fetch_string("https://example.invalid/nonexistent");
        assert!(result.is_err());
    }
}
