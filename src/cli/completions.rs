//! Shell completions generator for Anvil CLI
//!
//! This module provides functionality to generate and install shell completion scripts
//! for various shells including Bash, Zsh, Fish, PowerShell, and Elvish.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::Shell;
use colored::Colorize;

use super::commands::{CompletionsArgs, ShellType};
use super::Cli;

/// Convert our ShellType to clap_complete's Shell enum
impl From<ShellType> for Shell {
    fn from(shell: ShellType) -> Self {
        match shell {
            ShellType::Bash => Shell::Bash,
            ShellType::Zsh => Shell::Zsh,
            ShellType::Fish => Shell::Fish,
            ShellType::Powershell => Shell::PowerShell,
            ShellType::Elvish => Shell::Elvish,
        }
    }
}

/// Generate shell completions and write to stdout
///
/// # Arguments
///
/// * `args` - The completions command arguments containing the target shell
///
/// # Examples
///
/// ```bash
/// # Generate and install PowerShell completions
/// anvil completions powershell | Out-File -FilePath $PROFILE -Append
///
/// # Generate Bash completions
/// anvil completions bash > /etc/bash_completion.d/anvil
///
/// # Generate Zsh completions
/// anvil completions zsh > ~/.zsh/completions/_anvil
/// ```
pub fn generate_completions(args: &CompletionsArgs) -> Result<()> {
    let shell: Shell = args.shell.into();

    // Print installation instructions as comments first
    print_installation_instructions(args.shell)?;

    // Generate the actual completions
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "anvil", &mut io::stdout());

    Ok(())
}

/// Print installation instructions for the specified shell
fn print_installation_instructions(shell: ShellType) -> Result<()> {
    let instructions = get_installation_instructions(shell);

    // Print as comments appropriate for the shell
    let comment_prefix = match shell {
        ShellType::Powershell => "#",
        ShellType::Bash | ShellType::Zsh | ShellType::Fish | ShellType::Elvish => "#",
    };

    for line in instructions.lines() {
        if line.is_empty() {
            println!("{}", comment_prefix);
        } else {
            println!("{} {}", comment_prefix, line);
        }
    }
    println!("{}", comment_prefix);

    Ok(())
}

/// Get installation instructions for a specific shell
fn get_installation_instructions(shell: ShellType) -> String {
    match shell {
        ShellType::Powershell => {
            r#"Anvil PowerShell Completions

Installation:
  1. Save this output to a file:
     anvil completions powershell > "$HOME\Documents\PowerShell\anvil-completions.ps1"

  2. Add to your PowerShell profile ($PROFILE):
     . "$HOME\Documents\PowerShell\anvil-completions.ps1"

  3. Restart PowerShell or run: . $PROFILE

Alternative (add directly to profile):
  anvil completions powershell >> $PROFILE"#.to_string()
        }
        ShellType::Bash => {
            r#"Anvil Bash Completions

Installation (system-wide, requires root):
  anvil completions bash | sudo tee /etc/bash_completion.d/anvil > /dev/null

Installation (user-only):
  1. Create completions directory:
     mkdir -p ~/.local/share/bash-completion/completions

  2. Save completions:
     anvil completions bash > ~/.local/share/bash-completion/completions/anvil

  3. Restart your shell or run:
     source ~/.local/share/bash-completion/completions/anvil"#.to_string()
        }
        ShellType::Zsh => {
            r#"Anvil Zsh Completions

Installation:
  1. Create a completions directory if it doesn't exist:
     mkdir -p ~/.zfunc

  2. Add the directory to fpath in your .zshrc (before compinit):
     fpath=(~/.zfunc $fpath)

  3. Save completions:
     anvil completions zsh > ~/.zfunc/_anvil

  4. Regenerate completions cache:
     rm -f ~/.zcompdump; compinit

  5. Restart your shell"#.to_string()
        }
        ShellType::Fish => {
            r#"Anvil Fish Completions

Installation:
  anvil completions fish > ~/.config/fish/completions/anvil.fish

The completions will be automatically loaded on next shell start."#.to_string()
        }
        ShellType::Elvish => {
            r#"Anvil Elvish Completions

Installation:
  1. Create lib directory:
     mkdir -p ~/.config/elvish/lib

  2. Save completions:
     anvil completions elvish > ~/.config/elvish/lib/anvil.elv

  3. Add to your rc.elv:
     use anvil"#.to_string()
        }
    }
}

/// Get the default installation path for completions
#[allow(dead_code)]
pub fn get_completion_install_path(shell: ShellType) -> Result<PathBuf> {
    match shell {
        ShellType::Bash => {
            // Try user-local first, fall back to system
            let home = dirs::home_dir().context("Cannot find home directory")?;
            let local_path = home
                .join(".local")
                .join("share")
                .join("bash-completion")
                .join("completions")
                .join("anvil");
            Ok(local_path)
        }
        ShellType::Zsh => {
            let home = dirs::home_dir().context("Cannot find home directory")?;
            Ok(home.join(".zfunc").join("_anvil"))
        }
        ShellType::Fish => {
            let config = dirs::config_dir().context("Cannot find config directory")?;
            Ok(config.join("fish").join("completions").join("anvil.fish"))
        }
        ShellType::Powershell => {
            let docs = dirs::document_dir().context("Cannot find documents directory")?;
            Ok(docs.join("PowerShell").join("anvil-completions.ps1"))
        }
        ShellType::Elvish => {
            let config = dirs::config_dir().context("Cannot find config directory")?;
            Ok(config.join("elvish").join("lib").join("anvil.elv"))
        }
    }
}

/// Install completions to the appropriate location
#[allow(dead_code)]
pub fn install_completions(shell: ShellType) -> Result<PathBuf> {
    let install_path = get_completion_install_path(shell)?;

    // Create parent directories if needed
    if let Some(parent) = install_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Generate completions to a buffer
    let mut buffer = Vec::new();
    let mut cmd = Cli::command();
    let clap_shell: Shell = shell.into();
    clap_complete::generate(clap_shell, &mut cmd, "anvil", &mut buffer);

    // Write to file
    std::fs::write(&install_path, buffer)
        .with_context(|| format!("Failed to write completions to: {}", install_path.display()))?;

    Ok(install_path)
}

/// Print a success message for completion installation
#[allow(dead_code)]
pub fn print_install_success(shell: ShellType, path: &PathBuf) {
    println!("{} Completions installed to: {}", "✓".green(), path.display());
    println!();

    match shell {
        ShellType::Powershell => {
            println!("To enable, add to your PowerShell profile ($PROFILE):");
            println!("  . \"{}\"", path.display());
        }
        ShellType::Bash => {
            println!("Completions will be loaded automatically on next shell start.");
            println!("To load now, run:");
            println!("  source \"{}\"", path.display());
        }
        ShellType::Zsh => {
            println!("Ensure ~/.zfunc is in your fpath (add to .zshrc before compinit):");
            println!("  fpath=(~/.zfunc $fpath)");
            println!();
            println!("Then rebuild completion cache:");
            println!("  rm -f ~/.zcompdump; compinit");
        }
        ShellType::Fish => {
            println!("Completions will be loaded automatically on next shell start.");
        }
        ShellType::Elvish => {
            println!("Add to your rc.elv:");
            println!("  use anvil");
        }
    }
}

/// Generate completions to a specific writer (useful for testing)
#[allow(dead_code)]
pub fn generate_to<W: Write>(shell: Shell, writer: &mut W) -> Result<()> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "anvil", writer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_powershell_completions() {
        let mut output = Vec::new();
        generate_to(Shell::PowerShell, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();
        assert!(content.contains("anvil"));
    }

    #[test]
    fn test_generate_bash_completions() {
        let mut output = Vec::new();
        generate_to(Shell::Bash, &mut output).unwrap();
        let content = String::from_utf8(output).unwrap();
        assert!(content.contains("anvil"));
    }

    #[test]
    fn test_shell_type_conversion() {
        assert_eq!(Shell::from(ShellType::Bash), Shell::Bash);
        assert_eq!(Shell::from(ShellType::Zsh), Shell::Zsh);
        assert_eq!(Shell::from(ShellType::Fish), Shell::Fish);
        assert_eq!(Shell::from(ShellType::Powershell), Shell::PowerShell);
        assert_eq!(Shell::from(ShellType::Elvish), Shell::Elvish);
    }

    #[test]
    fn test_get_installation_instructions() {
        let ps_instructions = get_installation_instructions(ShellType::Powershell);
        assert!(ps_instructions.contains("PowerShell"));
        assert!(ps_instructions.contains("$PROFILE"));

        let bash_instructions = get_installation_instructions(ShellType::Bash);
        assert!(bash_instructions.contains("bash_completion"));

        let zsh_instructions = get_installation_instructions(ShellType::Zsh);
        assert!(zsh_instructions.contains("zfunc"));
        assert!(zsh_instructions.contains("compinit"));

        let fish_instructions = get_installation_instructions(ShellType::Fish);
        assert!(fish_instructions.contains(".config/fish"));

        let elvish_instructions = get_installation_instructions(ShellType::Elvish);
        assert!(elvish_instructions.contains("elvish"));
    }

    #[test]
    fn test_get_completion_install_path() {
        // These may fail if dirs can't find home/config directories
        // but shouldn't panic
        let _ = get_completion_install_path(ShellType::Bash);
        let _ = get_completion_install_path(ShellType::Zsh);
        let _ = get_completion_install_path(ShellType::Fish);
        let _ = get_completion_install_path(ShellType::Powershell);
        let _ = get_completion_install_path(ShellType::Elvish);
    }
}
