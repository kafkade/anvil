//! Windows Terminal settings provider
//!
//! Manages Windows Terminal configuration by merging color schemes
//! and profile defaults into the terminal's settings.json file.
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::workload::{ColorScheme, TerminalConfig};

#[derive(Error, Debug)]
pub enum TerminalError {
    #[error("Windows Terminal settings not found")]
    SettingsNotFound,
    #[error("Failed to parse settings: {0}")]
    ParseFailed(String),
    #[error("Failed to write settings: {0}")]
    WriteFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of applying terminal settings
pub struct TerminalResult {
    pub schemes_added: usize,
    pub schemes_updated: usize,
    #[allow(dead_code)] // Used by callers for reporting
    pub defaults_updated: bool,
    pub already_configured: bool,
}

pub struct TerminalProvider;

impl TerminalProvider {
    /// Apply terminal settings from a workload TerminalConfig
    pub fn apply_settings(
        config: &TerminalConfig,
        dry_run: bool,
    ) -> Result<TerminalResult, TerminalError> {
        let settings_path = match Self::find_settings_path() {
            Some(p) => p,
            None => return Err(TerminalError::SettingsNotFound),
        };

        let mut settings = Self::read_settings(&settings_path)?;

        let mut schemes_added = 0;
        let mut schemes_updated = 0;

        if let Some(ref schemes) = config.schemes {
            let (added, updated) = Self::merge_schemes(&mut settings, schemes);
            schemes_added = added;
            schemes_updated = updated;
        }

        let defaults_updated = if let Some(ref defaults) = config.profile_defaults {
            Self::merge_profile_defaults(&mut settings, defaults);
            true
        } else {
            false
        };

        let already_configured = schemes_added == 0 && schemes_updated == 0 && !defaults_updated;

        if !dry_run && !already_configured {
            Self::write_settings(&settings_path, &settings)?;
        }

        Ok(TerminalResult {
            schemes_added,
            schemes_updated,
            defaults_updated,
            already_configured,
        })
    }

    /// Find the Windows Terminal settings.json path
    pub fn find_settings_path() -> Option<PathBuf> {
        let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
        let base = PathBuf::from(local_app_data).join("Packages");

        let candidates = [
            "Microsoft.WindowsTerminal_8wekyb3d8bbwe",
            "Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe",
        ];

        for candidate in &candidates {
            let path = base
                .join(candidate)
                .join("LocalState")
                .join("settings.json");
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Read and parse the settings file, stripping JSONC comments
    pub fn read_settings(path: &Path) -> Result<serde_json::Value, TerminalError> {
        let raw = std::fs::read_to_string(path)?;
        let stripped = strip_jsonc_comments(&raw);
        serde_json::from_str(&stripped)
            .map_err(|e| TerminalError::ParseFailed(format!("{}: {}", path.display(), e)))
    }

    /// Merge color schemes into the settings, returning (added, updated) counts
    pub fn merge_schemes(
        settings: &mut serde_json::Value,
        schemes: &[ColorScheme],
    ) -> (usize, usize) {
        let schemes_array = settings
            .as_object_mut()
            .and_then(|obj| {
                if !obj.contains_key("schemes") {
                    obj.insert("schemes".to_string(), serde_json::json!([]));
                }
                obj.get_mut("schemes")
            })
            .and_then(|v| v.as_array_mut());

        let schemes_array = match schemes_array {
            Some(arr) => arr,
            None => return (0, 0),
        };

        let mut added = 0;
        let mut updated = 0;

        for scheme in schemes {
            let scheme_value = scheme_to_value(scheme);
            if let Some(existing) = schemes_array
                .iter_mut()
                .find(|s| s.get("name").and_then(|n| n.as_str()) == Some(&scheme.name))
            {
                *existing = scheme_value;
                updated += 1;
            } else {
                schemes_array.push(scheme_value);
                added += 1;
            }
        }

        (added, updated)
    }

    /// Merge profile defaults into the settings
    pub fn merge_profile_defaults(settings: &mut serde_json::Value, defaults: &serde_json::Value) {
        let profiles = settings
            .as_object_mut()
            .and_then(|obj| {
                if !obj.contains_key("profiles") {
                    obj.insert("profiles".to_string(), serde_json::json!({}));
                }
                obj.get_mut("profiles")
            })
            .and_then(|v| v.as_object_mut());

        let profiles = match profiles {
            Some(p) => p,
            None => return,
        };

        if !profiles.contains_key("defaults") {
            profiles.insert("defaults".to_string(), serde_json::json!({}));
        }

        if let Some(existing_defaults) = profiles.get_mut("defaults") {
            merge_json_objects(existing_defaults, defaults);
        }
    }

    /// Write settings back to disk, creating a backup first
    pub fn write_settings(path: &Path, settings: &serde_json::Value) -> Result<(), TerminalError> {
        // Create backup
        let backup_path = path.with_extension("json.bak");
        std::fs::copy(path, &backup_path)
            .map_err(|e| TerminalError::WriteFailed(format!("Failed to create backup: {}", e)))?;

        let json_str = serde_json::to_string_pretty(settings)
            .map_err(|e| TerminalError::WriteFailed(e.to_string()))?;

        std::fs::write(path, json_str).map_err(|e| TerminalError::WriteFailed(e.to_string()))?;

        Ok(())
    }

    /// Check if a color scheme exists in the terminal settings
    pub fn check_scheme_exists(scheme_name: &str) -> bool {
        let settings_path = match Self::find_settings_path() {
            Some(p) => p,
            None => return false,
        };

        let settings = match Self::read_settings(&settings_path) {
            Ok(s) => s,
            Err(_) => return false,
        };

        settings
            .get("schemes")
            .and_then(|s| s.as_array())
            .map(|schemes| {
                schemes
                    .iter()
                    .any(|s| s.get("name").and_then(|n| n.as_str()) == Some(scheme_name))
            })
            .unwrap_or(false)
    }
}

/// Strip full-line // comments from JSONC content
fn strip_jsonc_comments(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("//") {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a ColorScheme to a serde_json::Value
fn scheme_to_value(scheme: &ColorScheme) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("name".to_string(), serde_json::json!(scheme.name));

    macro_rules! insert_opt {
        ($field:ident, $key:expr) => {
            if let Some(ref v) = scheme.$field {
                map.insert($key.to_string(), serde_json::json!(v));
            }
        };
    }

    insert_opt!(background, "background");
    insert_opt!(foreground, "foreground");
    insert_opt!(black, "black");
    insert_opt!(red, "red");
    insert_opt!(green, "green");
    insert_opt!(yellow, "yellow");
    insert_opt!(blue, "blue");
    insert_opt!(purple, "purple");
    insert_opt!(cyan, "cyan");
    insert_opt!(white, "white");
    insert_opt!(bright_black, "brightBlack");
    insert_opt!(bright_red, "brightRed");
    insert_opt!(bright_green, "brightGreen");
    insert_opt!(bright_yellow, "brightYellow");
    insert_opt!(bright_blue, "brightBlue");
    insert_opt!(bright_purple, "brightPurple");
    insert_opt!(bright_cyan, "brightCyan");
    insert_opt!(bright_white, "brightWhite");
    insert_opt!(cursor_color, "cursorColor");
    insert_opt!(selection_background, "selectionBackground");

    serde_json::Value::Object(map)
}

/// Recursively merge source JSON object into target
fn merge_json_objects(target: &mut serde_json::Value, source: &serde_json::Value) {
    if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
        for (key, value) in source_obj {
            if let Some(existing) = target_obj.get_mut(key) {
                if existing.is_object() && value.is_object() {
                    merge_json_objects(existing, value);
                    continue;
                }
            }
            target_obj.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_jsonc_comments() {
        let input = r#"{
    // This is a comment
    "name": "test",
    // Another comment
    "value": 42
}"#;
        let result = strip_jsonc_comments(input);
        assert!(!result.contains("// This is a comment"));
        assert!(!result.contains("// Another comment"));
        assert!(result.contains("\"name\": \"test\""));
        assert!(result.contains("\"value\": 42"));
        // Should parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["value"], 42);
    }

    #[test]
    fn test_find_settings_path_returns_option() {
        // Just verify it doesn't panic — may return None in CI
        let _result = TerminalProvider::find_settings_path();
    }

    #[test]
    fn test_merge_schemes_adds_new() {
        let mut settings = serde_json::json!({
            "schemes": []
        });

        let schemes = vec![ColorScheme {
            name: "TestScheme".to_string(),
            background: Some("#000000".to_string()),
            foreground: Some("#FFFFFF".to_string()),
            black: None,
            red: None,
            green: None,
            yellow: None,
            blue: None,
            purple: None,
            cyan: None,
            white: None,
            bright_black: None,
            bright_red: None,
            bright_green: None,
            bright_yellow: None,
            bright_blue: None,
            bright_purple: None,
            bright_cyan: None,
            bright_white: None,
            cursor_color: None,
            selection_background: None,
        }];

        let (added, updated) = TerminalProvider::merge_schemes(&mut settings, &schemes);
        assert_eq!(added, 1);
        assert_eq!(updated, 0);

        let arr = settings["schemes"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "TestScheme");
        assert_eq!(arr[0]["background"], "#000000");
    }

    #[test]
    fn test_merge_schemes_updates_existing() {
        let mut settings = serde_json::json!({
            "schemes": [
                {
                    "name": "Existing",
                    "background": "#111111",
                    "foreground": "#222222"
                }
            ]
        });

        let schemes = vec![ColorScheme {
            name: "Existing".to_string(),
            background: Some("#AAAAAA".to_string()),
            foreground: Some("#BBBBBB".to_string()),
            black: None,
            red: None,
            green: None,
            yellow: None,
            blue: None,
            purple: None,
            cyan: None,
            white: None,
            bright_black: None,
            bright_red: None,
            bright_green: None,
            bright_yellow: None,
            bright_blue: None,
            bright_purple: None,
            bright_cyan: None,
            bright_white: None,
            cursor_color: None,
            selection_background: None,
        }];

        let (added, updated) = TerminalProvider::merge_schemes(&mut settings, &schemes);
        assert_eq!(added, 0);
        assert_eq!(updated, 1);

        let arr = settings["schemes"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["background"], "#AAAAAA");
        assert_eq!(arr[0]["foreground"], "#BBBBBB");
    }

    #[test]
    fn test_merge_profile_defaults() {
        let mut settings = serde_json::json!({
            "profiles": {
                "defaults": {
                    "colorScheme": "Old"
                }
            }
        });

        let defaults = serde_json::json!({
            "colorScheme": "Sorcerer",
            "font": {
                "face": "Cascadia Code NF",
                "size": 12
            }
        });

        TerminalProvider::merge_profile_defaults(&mut settings, &defaults);

        assert_eq!(settings["profiles"]["defaults"]["colorScheme"], "Sorcerer");
        assert_eq!(
            settings["profiles"]["defaults"]["font"]["face"],
            "Cascadia Code NF"
        );
        assert_eq!(settings["profiles"]["defaults"]["font"]["size"], 12);
    }

    #[test]
    fn test_merge_profile_defaults_creates_structure() {
        let mut settings = serde_json::json!({});

        let defaults = serde_json::json!({
            "colorScheme": "Sorcerer"
        });

        TerminalProvider::merge_profile_defaults(&mut settings, &defaults);

        assert_eq!(settings["profiles"]["defaults"]["colorScheme"], "Sorcerer");
    }
}
