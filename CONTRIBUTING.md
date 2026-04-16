# Contributing to Anvil

Thank you for your interest in contributing to Anvil! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Creating Workloads](#creating-workloads)
- [License](#license)

---

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

---

## How to Contribute

### Reporting Bugs

Before reporting a bug:

1. Check existing [issues](https://github.com/kafkade/anvil/issues) to avoid duplicates
2. Gather relevant information:
   - Anvil version (`anvil --version`)
   - Windows version
   - Steps to reproduce
   - Expected vs actual behavior
   - Verbose output (`anvil -vvv <command>`)

Create a bug report with this template:

```markdown
## Environment
- Anvil version: 
- Windows version: 
- PowerShell version: 

## Description
Brief description of the bug.

## Steps to Reproduce
1. 
2. 
3. 

## Expected Behavior
What you expected to happen.

## Actual Behavior
What actually happened.

## Verbose Output
```
(paste -vvv output here)
```

## Workload (if applicable)
```yaml
(paste workload.yaml here)
```
```

### Suggesting Features

1. Check existing issues and discussions for similar suggestions
2. Open a feature request issue with:
   - Clear description of the feature
   - Use case and motivation
   - Proposed implementation (if you have ideas)

### Submitting Code

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run lints (`cargo clippy`)
6. Format code (`cargo fmt`)
7. Commit with a descriptive message
8. Push to your fork
9. Open a Pull Request

---

## Development Setup

### Prerequisites

- **Rust**: 1.75 or later ([rustup.rs](https://rustup.rs/))
- **Visual Studio Build Tools**: For Windows linking
- **Windows 10/11**: Required for testing
- **Windows Package Manager (winget)**: For package operations

### Building

```powershell
# Clone your fork
git clone https://github.com/YOUR_USERNAME/anvil.git
cd anvil

# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run clippy lints
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Run with debug output
cargo run -- -vvv list
```

### Running Tests

```powershell
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test '*'

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

---

## Project Structure

```
anvil/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli/                 # Command line interface
│   │   ├── mod.rs           # CLI definitions (clap)
│   │   ├── commands.rs      # Command argument structs
│   │   ├── completions.rs   # Shell completions
│   │   ├── output.rs        # Output handling
│   │   ├── progress.rs      # Progress indicators
│   │   └── formats/         # Output formatters
│   │       ├── mod.rs
│   │       ├── table.rs     # Table formatter
│   │       ├── json.rs      # JSON formatter
│   │       ├── yaml.rs      # YAML formatter
│   │       └── html.rs      # HTML report generator
│   ├── config/              # Configuration handling
│   │   ├── mod.rs
│   │   ├── schema.rs        # Workload schema validation
│   │   ├── workload.rs      # Workload parsing
│   │   ├── inheritance.rs   # Inheritance resolution
│   │   └── global.rs        # Global configuration
│   ├── operations/          # Command implementations
│   │   ├── mod.rs
│   │   ├── install.rs       # Install command
│   │   ├── health.rs        # Health check command
│   │   ├── list.rs          # List command
│   │   ├── show.rs          # Show command
│   │   ├── validate.rs      # Validate command
│   │   ├── init.rs          # Init command
│   │   ├── status.rs        # Status command
│   │   ├── backup.rs        # Backup command
│   │   └── config.rs        # Config command
│   ├── providers/           # External integrations
│   │   ├── mod.rs
│   │   ├── winget.rs        # Winget package manager
│   │   ├── filesystem.rs    # File operations
│   │   ├── script.rs        # Script execution
│   │   ├── template.rs      # Template processing
│   │   └── backup.rs        # Backup provider
│   └── state/               # State management
│       ├── mod.rs
│       ├── installation.rs  # Installation state
│       ├── cache.rs         # Cache management
│       └── files.rs         # File state tracking
├── workloads/               # Bundled workloads
│   ├── essentials/
│   ├── rust-developer/
│   ├── python-developer/
│   └── ui/
├── tests/                   # Integration tests
│   ├── cli_tests.rs
│   └── common/
│       └── mod.rs
└── docs/                    # Documentation
    ├── SPECIFICATION.md
    ├── USER_GUIDE.md
    ├── WORKLOAD_AUTHORING.md
    └── TROUBLESHOOTING.md
```

### Key Modules

- **cli**: Handles command-line parsing with clap and output formatting
- **config**: Parses workload YAML files and handles inheritance
- **operations**: Implements each CLI command's business logic
- **providers**: Interfaces with external systems (winget, filesystem, PowerShell)
- **state**: Tracks installation state and manages caching

---

## Coding Standards

### Rust Style

- Follow standard Rust conventions and idioms
- Use `rustfmt` for formatting (default settings)
- Address all `clippy` warnings
- Use `thiserror` for error types
- Use `anyhow` for error propagation in application code

### Code Organization

- Keep functions focused and small
- Use descriptive names for functions and variables
- Add doc comments for public APIs
- Use modules to organize related functionality

### Error Handling

```rust
// Define custom errors with thiserror
#[derive(Debug, thiserror::Error)]
pub enum WorkloadError {
    #[error("Workload '{name}' not found")]
    NotFound { name: String },
    
    #[error("Invalid workload schema: {message}")]
    InvalidSchema { message: String },
}

// Use anyhow for propagation
pub fn load_workload(name: &str) -> anyhow::Result<Workload> {
    // ...
}
```

### Documentation

```rust
/// Brief description of the function.
///
/// More detailed description if needed.
///
/// # Arguments
///
/// * `name` - The workload name to load
///
/// # Returns
///
/// The loaded workload or an error
///
/// # Errors
///
/// Returns an error if the workload is not found
///
/// # Examples
///
/// ```
/// let workload = load_workload("my-workload")?;
/// ```
pub fn load_workload(name: &str) -> Result<Workload> {
    // ...
}
```

### Commit Messages

- Use present tense ("Add feature" not "Added feature")
- Use imperative mood ("Fix bug" not "Fixes bug")
- Keep the first line under 72 characters
- Reference issues when applicable

Examples:
```
Add shell completions for Fish

Implement Fish shell completion generation using clap_complete.
Closes #42
```

```
Fix file backup path on Windows

Handle UNC paths correctly when creating backups.
Fixes #55
```

---

## Testing

### Unit Tests

Add unit tests in the same file as the code being tested:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = "test";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration Tests

Add integration tests in `tests/cli_tests.rs`:

```rust
#[test]
fn new_command_works() {
    anvil()
        .args(["new-command", "arg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("expected output"));
}
```

### Test Guidelines

- Each test should be independent
- Use descriptive test names
- Test both success and failure cases
- Use `tempfile::TempDir` for file operations
- Don't rely on external system state where avoidable

---

## Submitting Changes

### Pull Request Process

1. **Update documentation** if needed
2. **Add tests** for new features
3. **Ensure CI passes** (format, lint, test, build)
4. **Write a clear PR description** explaining:
   - What the change does
   - Why it's needed
   - How to test it
5. **Request review** from maintainers
6. **Address feedback** promptly
7. **Squash commits** if requested

### PR Title Format

- `feat: Add new feature`
- `fix: Fix specific bug`
- `docs: Update documentation`
- `test: Add tests`
- `refactor: Restructure code`
- `chore: Update dependencies`

---

## Creating Workloads

Contributions of new bundled workloads are welcome! See the [Workload Authoring Guide](docs/WORKLOAD_AUTHORING.md) for details.

### Workload Guidelines

1. **Include meaningful health checks** - Verify the workload achieves its purpose
2. **Document the workload purpose** - Clear description and comments
3. **Test on clean Windows installation** - Ensure it works from scratch
4. **Use inheritance** for common bases (extend `essentials` if appropriate)
5. **Keep packages minimal** - Only include what's necessary
6. **Validate before submitting** - `anvil validate your-workload --strict`

### Workload Structure

```
workload-name/
├── workload.yaml       # Required: workload definition
├── files/              # Optional: configuration files
└── scripts/            # Optional: setup/health scripts
```

### Example

```yaml
name: my-workload
version: "1.0.0"
description: "Brief description of what this workload provides"

extends:
  - essentials     # If applicable

packages:
  winget:
    - id: Package.ID

scripts:
  health_check:
    - path: scripts/health.ps1
      name: "Verification"
      description: "Verify installation"
```

---

## License

By contributing to Anvil, you agree that your contributions will be licensed under the [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) license, at the user's choice.

---

## Questions?

- Open a [Discussion](https://github.com/kafkade/anvil/discussions)
- Check the [Documentation](docs/)
- Review existing [Issues](https://github.com/kafkade/anvil/issues)

Thank you for contributing to Anvil! 🎉