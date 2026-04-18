# Cookbook

Practical recipes for common Anvil tasks. Each recipe is self-contained with copy-pasteable examples. See the [User Guide](user-guide.md) and [Workload Authoring](workload-authoring.md) for full reference.

## Creating Your First Workload

Start with `anvil init` to scaffold a new workload:

```sh
# Create a minimal workload
anvil init my-setup

# Create a full-featured template with all sections
anvil init my-setup --template full

# Create a workload that extends another
anvil init my-setup --extends essentials
```

This creates a directory with a `workload.yaml` and optional `files/` and `scripts/` subdirectories. Edit the YAML to define your environment:

```yaml
name: my-setup
version: "1.0.0"
description: "My development environment"

packages:
  winget:
    - id: Git.Git
    - id: Microsoft.VisualStudioCode
```

Preview what would happen, then apply:

```sh
anvil install my-setup --dry-run   # preview
anvil install my-setup             # apply
anvil health my-setup              # verify
```

## Adding Packages

### Basic winget packages

```yaml
packages:
  winget:
    - id: Git.Git
    - id: BurntSushi.ripgrep.MSVC
    - id: sharkdp.fd
```

### Pinning a version

```yaml
packages:
  winget:
    - id: Python.Python.3.12
      version: "3.12.4"
```

### Installing from the Microsoft Store

```yaml
packages:
  winget:
    - id: 9NBLGGH4NNS1        # Windows Terminal
      source: msstore
```

### Custom install arguments

```yaml
packages:
  winget:
    - id: Microsoft.VisualStudio.2022.BuildTools
      override:
        - --override
        - "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools"
```

### Multi-platform packages

Define packages for multiple platforms — Anvil picks the right manager for the current OS:

```yaml
packages:
  winget:
    - id: Git.Git
  brew:
    - name: git
  apt:
    - name: git
```

## Deploying Configuration Files

### Basic file deployment

Place source files in the `files/` subdirectory of your workload, then reference them in `workload.yaml`:

```yaml
files:
  - source: config.toml
    destination: "~/.cargo/config.toml"
    backup: true
```

The `~` expands to the user's home directory. With `backup: true`, Anvil saves the existing file before overwriting.

### Directory structure mirroring

Organize source files to mirror the target directory structure:

```
my-workload/
├── workload.yaml
└── files/
    └── .config/
        ├── starship.toml
        └── alacritty/
            └── alacritty.toml
```

```yaml
files:
  - source: .config/starship.toml
    destination: "~/.config/starship.toml"
    backup: true
  - source: .config/alacritty/alacritty.toml
    destination: "~/.config/alacritty/alacritty.toml"
    backup: true
```

### Deploying files without touching packages

```sh
anvil install my-setup --files-only --dry-run
```

## Using Assertions for Declarative Health Checks

Assertions are the recommended way to validate your environment — no scripting needed.

> **Note:** `scripts.health_check` was removed in v1.0. Use declarative `assertions` instead.

### Check that commands exist

```yaml
assertions:
  - name: Git is installed
    check:
      type: command_exists
      command: git

  - name: Cargo is installed
    check:
      type: command_exists
      command: cargo
```

### Check files and directories

```yaml
assertions:
  - name: SSH key exists
    check:
      type: file_exists
      path: "~/.ssh/id_ed25519"

  - name: Cargo directory exists
    check:
      type: dir_exists
      path: "~/.cargo"
```

### Check environment variables

```yaml
assertions:
  - name: RUST_BACKTRACE is set
    check:
      type: env_var
      name: RUST_BACKTRACE
      value: "1"

  - name: Cargo bin is on PATH
    check:
      type: path_contains
      substring: ".cargo/bin"
```

### Check Windows registry values

```yaml
assertions:
  - name: Developer mode enabled
    check:
      type: registry_value
      key: "HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock"
      value_name: "AllowDevelopmentWithoutDevLicense"
      expected: "1"
```

### Run a shell command as a check

```yaml
assertions:
  - name: Rust stable is default toolchain
    check:
      type: shell
      command: "rustup default"
      contains: "stable"
```

### Compose checks with all_of / any_of

```yaml
assertions:
  - name: Rust toolchain is fully configured
    check:
      type: all_of
      conditions:
        - type: command_exists
          command: rustc
        - type: command_exists
          command: cargo
        - type: dir_exists
          path: "~/.cargo"
        - type: path_contains
          substring: ".cargo/bin"
```

Run assertion checks:

```sh
anvil health my-setup --assertions-only
```

Enable assertions in the health config:

```yaml
health:
  assertion_check: true
```

## Running Post-Install Commands

Use the `commands` block for inline shell commands that run during installation — no separate script files needed.

### Basic commands

```yaml
commands:
  post_install:
    - run: "rustup component add clippy rustfmt"
      description: "Install Rust components"

    - run: "cargo install cargo-watch cargo-nextest"
      description: "Install cargo tools"
```

### Pre-install setup

```yaml
commands:
  pre_install:
    - run: "mkdir -p ~/.config"
      description: "Ensure config directory exists"
```

### Conditional execution

Only run a command when a condition is met:

```yaml
commands:
  post_install:
    - run: "rustup component add rust-analyzer"
      description: "Install rust-analyzer"
      when:
        command_exists: rustup

    - run: "npm install -g typescript"
      description: "Install TypeScript globally"
      when:
        command_exists: npm
```

### Continue on error

Allow a command to fail without stopping the install:

```yaml
commands:
  post_install:
    - run: "cargo install sccache"
      description: "Install sccache (optional)"
      continue_on_error: true
```

## Using Workload Inheritance

Inheritance lets you build on top of existing workloads. Child workloads inherit packages, files, scripts, and environment from their parents.

### Extending a base workload

```yaml
name: rust-developer
version: "1.0.0"
description: "Rust dev environment"

extends:
  - essentials        # inherits Git, VS Code, Terminal, etc.

packages:
  winget:
    - id: Rustlang.Rustup   # added on top of essentials packages
```

### Building a team workload chain

```
essentials          → shared dev tools
  └── backend       → adds Docker, Postgres, Redis
       └── rust-be  → adds Rust toolchain
       └── go-be    → adds Go toolchain
```

```yaml
# backend/workload.yaml
name: backend
version: "1.0.0"
description: "Backend development base"
extends:
  - essentials

packages:
  winget:
    - id: Docker.DockerDesktop
    - id: PostgreSQL.PostgreSQL
```

```yaml
# rust-be/workload.yaml
name: rust-be
version: "1.0.0"
description: "Rust backend developer"
extends:
  - backend

packages:
  winget:
    - id: Rustlang.Rustup
```

### How inheritance merges

- **Packages**: child packages are appended after parent packages
- **Files**: child files are appended; same destination = child overrides parent
- **Scripts**: concatenated (parent scripts run first)
- **Environment**: child variables override same-named parent variables
- **Assertions**: child assertions are appended after parent assertions

## Setting Up Environment Variables

### User-scoped variables

```yaml
environment:
  variables:
    - name: EDITOR
      value: "code --wait"
      scope: user

    - name: RUST_BACKTRACE
      value: "1"
      scope: user
```

### PATH additions

```yaml
environment:
  path_additions:
    - "~/.cargo/bin"
    - "~/go/bin"
    - "~/.local/bin"
```

## Managing Private Workload Repositories

Keep team-specific workloads in a private Git repo and configure Anvil to find them.

### Set up the search path

```sh
# Clone your team's workloads
git clone git@github.com:myteam/workloads.git ~/team-workloads

# Tell Anvil where to find them
anvil config set workloads.paths '["~/team-workloads"]'
```

Or edit `~/.anvil/config.yaml` directly:

```yaml
workloads:
  paths:
    - "~/team-workloads"
    - "~/personal-workloads"
```

### Search precedence

When multiple paths contain a workload with the same name, the first match wins:

1. Explicit `--path` argument (highest priority)
2. User-configured paths from `config.yaml`
3. Default search paths (bundled workloads)

See all discovered paths and any shadowed duplicates:

```sh
anvil list --all-paths --long
```

### Private repo structure

```
team-workloads/
├── base-dev/
│   └── workload.yaml
├── frontend/
│   └── workload.yaml
├── backend/
│   ├── workload.yaml
│   ├── files/
│   │   └── .pgpass
│   └── scripts/
│       └── post-install.ps1
└── data-science/
    └── workload.yaml
```

Team members clone the repo and configure the path once. Updates are pulled with `git pull`.

## Generating Reports

### JSON output for scripting

```sh
anvil health my-setup --output json | jq '.summary'
```

### HTML health report

```sh
anvil health my-setup --output html --file report.html
```

### YAML export

```sh
anvil show my-setup --output yaml
```

### List workloads as JSON

```sh
anvil list --output json | jq '.[].name'
```
