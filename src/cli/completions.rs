//! Shell completions generator for Anvil CLI
//!
//! This module provides functionality to generate and install shell completion scripts
//! for various shells including Bash, Zsh, Fish, PowerShell, and Elvish.
use std::io;

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::Shell;

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
        ShellType::Powershell => r#"Anvil PowerShell Completions

Installation:
  1. Save this output to a file:
     anvil completions powershell > "$HOME\Documents\PowerShell\anvil-completions.ps1"

  2. Add to your PowerShell profile ($PROFILE):
     . "$HOME\Documents\PowerShell\anvil-completions.ps1"

  3. Restart PowerShell or run: . $PROFILE

Alternative (add directly to profile):
  anvil completions powershell >> $PROFILE"#
            .to_string(),
        ShellType::Bash => r#"Anvil Bash Completions

Installation (system-wide, requires root):
  anvil completions bash | sudo tee /etc/bash_completion.d/anvil > /dev/null

Installation (user-only):
  1. Create completions directory:
     mkdir -p ~/.local/share/bash-completion/completions

  2. Save completions:
     anvil completions bash > ~/.local/share/bash-completion/completions/anvil

  3. Restart your shell or run:
     source ~/.local/share/bash-completion/completions/anvil"#
            .to_string(),
        ShellType::Zsh => r#"Anvil Zsh Completions

Installation:
  1. Create a completions directory if it doesn't exist:
     mkdir -p ~/.zfunc

  2. Add the directory to fpath in your .zshrc (before compinit):
     fpath=(~/.zfunc $fpath)

  3. Save completions:
     anvil completions zsh > ~/.zfunc/_anvil

  4. Regenerate completions cache:
     rm -f ~/.zcompdump; compinit

  5. Restart your shell"#
            .to_string(),
        ShellType::Fish => r#"Anvil Fish Completions

Installation:
  anvil completions fish > ~/.config/fish/completions/anvil.fish

The completions will be automatically loaded on next shell start."#
            .to_string(),
        ShellType::Elvish => r#"Anvil Elvish Completions

Installation:
  1. Create lib directory:
     mkdir -p ~/.config/elvish/lib

  2. Save completions:
     anvil completions elvish > ~/.config/elvish/lib/anvil.elv

  3. Add to your rc.elv:
     use anvil"#
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_powershell_completions() {
        let mut output = Vec::new();
        let mut cmd = Cli::command();
        clap_complete::generate(Shell::PowerShell, &mut cmd, "anvil", &mut output);
        let content = String::from_utf8(output).unwrap();
        assert!(content.contains("anvil"));
    }

    #[test]
    fn test_generate_bash_completions() {
        let mut output = Vec::new();
        let mut cmd = Cli::command();
        clap_complete::generate(Shell::Bash, &mut cmd, "anvil", &mut output);
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
}
