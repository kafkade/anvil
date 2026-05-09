//! Font installation provider
//!
//! Downloads font archives from URLs, extracts font files,
//! copies them to the system fonts directory, and registers
//! them in the Windows registry.
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

use crate::config::workload::FontEntry;

#[derive(Error, Debug)]
pub enum FontError {
    #[error("Failed to download font: {0}")]
    DownloadFailed(String),
    #[error("Failed to extract font archive: {0}")]
    ExtractionFailed(String),
    #[error("Failed to install font: {0}")]
    InstallFailed(String),
    #[error("Font registration failed: {0}")]
    RegistrationFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of a font installation operation
pub struct FontInstallResult {
    #[allow(dead_code)] // Used by callers for logging
    pub font_name: String,
    pub files_installed: usize,
    pub already_installed: bool,
}

pub struct FontProvider;

impl FontProvider {
    /// Install a font from a workload FontEntry
    pub fn install_font(entry: &FontEntry, dry_run: bool) -> Result<FontInstallResult, FontError> {
        // Check if already installed
        if Self::is_font_installed(&entry.name) {
            return Ok(FontInstallResult {
                font_name: entry.name.clone(),
                files_installed: 0,
                already_installed: true,
            });
        }

        if dry_run {
            return Ok(FontInstallResult {
                font_name: entry.name.clone(),
                files_installed: 0,
                already_installed: false,
            });
        }

        // Create temp directory for download and extraction
        let temp_dir = std::env::temp_dir().join(format!(
            "anvil-font-{}",
            entry.name.to_lowercase().replace(' ', "-")
        ));
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir)?;

        let zip_path = temp_dir.join("font.zip");
        let extract_path = temp_dir.join("extracted");

        // Download
        Self::download_archive(&entry.url, &zip_path)?;

        // Extract
        Self::extract_archive(&zip_path, &extract_path)?;

        // Find font files
        let font_files = Self::find_font_files(
            &extract_path,
            &entry.format,
            entry.subfolder.as_deref(),
            entry.variants.as_deref(),
        );

        if font_files.is_empty() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(FontError::InstallFailed(format!(
                "No .{} font files found in archive",
                entry.format
            )));
        }

        // Get system fonts directory
        let fonts_dir = PathBuf::from(
            std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string()),
        )
        .join("Fonts");

        // Copy font files to system fonts dir
        let files_installed = Self::install_font_files(&font_files, &fonts_dir)?;

        // Register fonts in registry
        Self::register_fonts(&font_files)?;

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(FontInstallResult {
            font_name: entry.name.clone(),
            files_installed,
            already_installed: false,
        })
    }

    /// Download an archive from a URL using PowerShell
    fn download_archive(url: &str, dest: &Path) -> Result<(), FontError> {
        let script = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             (New-Object System.Net.WebClient).DownloadFile('{}', '{}')",
            url,
            dest.display()
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| FontError::DownloadFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FontError::DownloadFailed(stderr.to_string()));
        }

        Ok(())
    }

    /// Extract a zip archive using PowerShell
    fn extract_archive(zip_path: &Path, dest: &Path) -> Result<(), FontError> {
        let script = format!(
            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            zip_path.display(),
            dest.display()
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| FontError::ExtractionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FontError::ExtractionFailed(stderr.to_string()));
        }

        Ok(())
    }

    /// Find font files matching the given format in a directory
    pub fn find_font_files(
        dir: &Path,
        format: &str,
        subfolder: Option<&str>,
        variants: Option<&[String]>,
    ) -> Vec<PathBuf> {
        let search_dir = if let Some(sub) = subfolder {
            // Search recursively for the subfolder
            find_subfolder(dir, sub).unwrap_or_else(|| dir.to_path_buf())
        } else {
            dir.to_path_buf()
        };

        let extension = format.to_lowercase();
        let mut font_files = Vec::new();

        // Collect matching files
        collect_font_files(&search_dir, &extension, &mut font_files);

        // If no files found in the target dir, try recursively from root
        if font_files.is_empty() && subfolder.is_some() {
            collect_font_files(dir, &extension, &mut font_files);
        }

        // Filter by variants if specified
        if let Some(variants) = variants {
            font_files.retain(|f| {
                let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
                variants.iter().any(|v| stem.contains(v.as_str()))
            });
        }

        font_files
    }

    /// Copy font files to the system fonts directory
    fn install_font_files(files: &[PathBuf], fonts_dir: &Path) -> Result<usize, FontError> {
        let mut installed = 0;
        for file in files {
            let file_name = file
                .file_name()
                .ok_or_else(|| FontError::InstallFailed("Font file has no filename".to_string()))?;
            let dest = fonts_dir.join(file_name);
            std::fs::copy(file, &dest).map_err(|e| {
                FontError::InstallFailed(format!(
                    "Failed to copy {} to {}: {}",
                    file.display(),
                    dest.display(),
                    e
                ))
            })?;
            installed += 1;
        }
        Ok(installed)
    }

    /// Register fonts in the Windows registry
    fn register_fonts(files: &[PathBuf]) -> Result<(), FontError> {
        let reg_path = r"HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";

        for file in files {
            let file_name = file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let font_name = file
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            let script = format!(
                "Set-ItemProperty -Path '{}' -Name '{} (TrueType)' -Value '{}' -Force",
                reg_path, font_name, file_name
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| FontError::RegistrationFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(FontError::RegistrationFailed(format!(
                    "Failed to register font '{}': {}",
                    font_name, stderr
                )));
            }
        }

        Ok(())
    }

    /// Check if a font is already installed by querying the registry
    pub fn is_font_installed(name_pattern: &str) -> bool {
        let script = format!(
            "$fonts = Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts' -ErrorAction SilentlyContinue; \
             if ($fonts) {{ ($fonts.PSObject.Properties | Where-Object {{ $_.Name -like '*{}*' }}).Count -gt 0 }} else {{ $false }}",
            name_pattern.replace('\'', "''")
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output();

        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("true"),
            _ => false,
        }
    }

    /// Check if a font is installed (used by the assertion engine)
    #[allow(dead_code)] // Public API for assertion engine
    pub fn check_font_installed(name: &str) -> Result<bool, FontError> {
        Ok(Self::is_font_installed(name))
    }
}

/// Recursively find a subfolder by name
fn find_subfolder(dir: &Path, name: &str) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
                {
                    return Some(path);
                }
                // Recurse into subdirectories
                if let Some(found) = find_subfolder(&path, name) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Recursively collect font files matching the given extension
fn collect_font_files(dir: &Path, extension: &str, results: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_font_files(&path, extension, results);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case(extension))
                .unwrap_or(false)
            {
                results.push(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_font_files() {
        let temp = TempDir::new().unwrap();
        // Create some test font files
        std::fs::write(temp.path().join("Font-Regular.ttf"), b"fake").unwrap();
        std::fs::write(temp.path().join("Font-Bold.ttf"), b"fake").unwrap();
        std::fs::write(temp.path().join("readme.txt"), b"readme").unwrap();

        let files = FontProvider::find_font_files(temp.path(), "ttf", None, None);
        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .all(|f| f.extension().and_then(|e| e.to_str()).unwrap() == "ttf"));
    }

    #[test]
    fn test_find_font_files_with_subfolder() {
        let temp = TempDir::new().unwrap();
        let ttf_dir = temp.path().join("ttf");
        std::fs::create_dir_all(&ttf_dir).unwrap();
        std::fs::write(ttf_dir.join("Font-Regular.ttf"), b"fake").unwrap();
        std::fs::write(ttf_dir.join("Font-Bold.ttf"), b"fake").unwrap();
        // Also a file in the root that should NOT be found when subfolder is specified
        std::fs::write(temp.path().join("OtherFont.ttf"), b"fake").unwrap();

        let files = FontProvider::find_font_files(temp.path(), "ttf", Some("ttf"), None);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_find_font_files_with_variants() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("Font-Regular.ttf"), b"fake").unwrap();
        std::fs::write(temp.path().join("Font-Bold.ttf"), b"fake").unwrap();
        std::fs::write(temp.path().join("Font-Italic.ttf"), b"fake").unwrap();

        let variants = vec!["Regular".to_string(), "Bold".to_string()];
        let files =
            FontProvider::find_font_files(temp.path(), "ttf", None, Some(variants.as_slice()));
        assert_eq!(files.len(), 2);
        // Should NOT include Italic
        assert!(files.iter().all(
            |f| f.to_string_lossy().contains("Regular") || f.to_string_lossy().contains("Bold")
        ));
    }

    #[test]
    fn test_find_font_files_empty_dir() {
        let temp = TempDir::new().unwrap();
        let files = FontProvider::find_font_files(temp.path(), "ttf", None, None);
        assert!(files.is_empty());
    }

    #[test]
    fn test_is_font_installed_not_found() {
        // Use a name that definitely won't be installed
        assert!(!FontProvider::is_font_installed(
            "AnvilTestFontXyz12345NonExistent"
        ));
    }
}
