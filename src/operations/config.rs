//! Config operation module
//!
//! This module implements the `anvil config` command which provides
//! global configuration management functionality.

use std::io::Write;

use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

use crate::cli::commands::{ConfigArgs, ConfigCommand, OutputFormat};
use crate::cli::output::{print_error, print_info, print_success};
use crate::cli::Cli;
use crate::config::GlobalConfig;

/// Execute the config command
pub fn execute(args: &ConfigArgs, cli: &Cli) -> Result<()> {
    let use_color = !cli.should_disable_color();

    match &args.command {
        ConfigCommand::Get { key } => get_config(key, use_color),
        ConfigCommand::Set { key, value } => set_config(key, value),
        ConfigCommand::List { output } => list_config(*output, use_color),
        ConfigCommand::Reset { force } => reset_config(*force),
        ConfigCommand::Edit => edit_config(),
        ConfigCommand::Path => show_config_path(),
    }
}

/// Get a configuration value
fn get_config(key: &str, use_color: bool) -> Result<()> {
    let config = GlobalConfig::load().context("Failed to load configuration")?;

    match config.get(key) {
        Some(value) => {
            if use_color {
                println!("{} = {}", key.cyan(), value.green());
            } else {
                println!("{} = {}", key, value);
            }
        }
        None => {
            print_error(&format!("Unknown configuration key: {}", key));
            println!();
            println!("Available keys:");
            for (k, _) in config.list() {
                println!("  - {}", k);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Set a configuration value
fn set_config(key: &str, value: &str) -> Result<()> {
    let mut config = GlobalConfig::load().context("Failed to load configuration")?;

    // Validate and set the value
    config.set(key, value).with_context(|| {
        format!("Failed to set configuration key '{}' to '{}'", key, value)
    })?;

    // Save the updated configuration
    config.save().context("Failed to save configuration")?;

    print_success(&format!("Set {} = {}", key, value));

    Ok(())
}

/// List all configuration values
fn list_config(output: OutputFormat, use_color: bool) -> Result<()> {
    let config = GlobalConfig::load().context("Failed to load configuration")?;
    let items = config.list();

    match output {
        OutputFormat::Table => print_config_table(&items, use_color),
        OutputFormat::Json => {
            let map: std::collections::HashMap<String, String> = items.into_iter().collect();
            let json = serde_json::to_string_pretty(&map)?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let map: std::collections::HashMap<String, String> = items.into_iter().collect();
            let yaml = serde_yaml::to_string(&map)?;
            print!("{}", yaml);
        }
        OutputFormat::Html => {
            // Fall back to table for HTML
            print_config_table(&items, false);
        }
    }

    Ok(())
}

/// Print configuration as a formatted table
fn print_config_table(items: &[(String, String)], use_color: bool) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let headers = if use_color {
        vec![
            Cell::new("Key").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ]
    } else {
        vec![Cell::new("Key"), Cell::new("Value")]
    };
    table.set_header(headers);

    // Group by section
    let mut current_section = String::new();
    for (key, value) in items {
        let section = key.split('.').next().unwrap_or("");

        if section != current_section {
            current_section = section.to_string();
            // Add section header
            if use_color {
                table.add_row(vec![
                    Cell::new(format!("[{}]", section)).fg(Color::White),
                    Cell::new(""),
                ]);
            } else {
                table.add_row(vec![
                    Cell::new(format!("[{}]", section)),
                    Cell::new(""),
                ]);
            }
        }

        let row = if use_color {
            vec![
                Cell::new(format!("  {}", key)).fg(Color::Green),
                Cell::new(value),
            ]
        } else {
            vec![Cell::new(format!("  {}", key)), Cell::new(value)]
        };
        table.add_row(row);
    }

    println!("{}", table);
    println!();

    // Show config file location
    if let Ok(path) = GlobalConfig::config_path() {
        if use_color {
            println!(
                "{} {}",
                "Config file:".dimmed(),
                path.display().to_string().dimmed()
            );
        } else {
            println!("Config file: {}", path.display());
        }
    }
}

/// Reset configuration to defaults
fn reset_config(force: bool) -> Result<()> {
    if !force {
        print!("Reset all configuration to defaults? [y/N] ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            print_info("Reset cancelled.");
            return Ok(());
        }
    }

    let config = GlobalConfig::default();
    config.save().context("Failed to save default configuration")?;

    print_success("Configuration reset to defaults.");

    Ok(())
}

/// Open configuration file in default editor
fn edit_config() -> Result<()> {
    let config_path = GlobalConfig::config_path().context("Failed to get config path")?;

    // Ensure config file exists
    if !config_path.exists() {
        let config = GlobalConfig::default();
        config.save().context("Failed to create default configuration")?;
        print_info(&format!("Created default config at: {}", config_path.display()));
    }

    // Try to open with default editor
    #[cfg(windows)]
    {
        use std::process::Command;

        // Try common editors in order
        let editors = ["code", "notepad++", "notepad"];
        let mut opened = false;

        for editor in editors {
            let result = Command::new(editor)
                .arg(&config_path)
                .spawn();

            if result.is_ok() {
                print_success(&format!("Opened config file with {}", editor));
                opened = true;
                break;
            }
        }

        if !opened {
            // Fall back to shell open
            let result = Command::new("cmd")
                .args(["/C", "start", "", &config_path.to_string_lossy()])
                .spawn();

            if result.is_ok() {
                print_success("Opened config file with default application");
            } else {
                print_error("Failed to open config file");
                println!("Config file location: {}", config_path.display());
            }
        }
    }

    #[cfg(not(windows))]
    {
        use std::process::Command;

        // Try $EDITOR or fall back to common editors
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "vi".to_string());

        let result = Command::new(&editor)
            .arg(&config_path)
            .status();

        match result {
            Ok(status) if status.success() => {
                print_success("Config file edited successfully");
            }
            _ => {
                print_error("Failed to open config file");
                println!("Config file location: {}", config_path.display());
            }
        }
    }

    Ok(())
}

/// Show the configuration file path
fn show_config_path() -> Result<()> {
    let config_path = GlobalConfig::config_path().context("Failed to get config path")?;

    println!("{}", config_path.display());

    if config_path.exists() {
        print_info("Config file exists");
    } else {
        print_info("Config file does not exist yet (will be created on first write)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_config_table() {
        let items = vec![
            ("defaults.shell".to_string(), "powershell".to_string()),
            ("defaults.timeout".to_string(), "300".to_string()),
            ("backup.auto_backup".to_string(), "true".to_string()),
        ];

        // Should not panic
        print_config_table(&items, false);
    }
}
