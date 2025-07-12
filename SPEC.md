# crowdcontrol: Containerized Development Environment Specification

## Overview

crowdcontrol is a containerized development environment system that enables parallel development across multiple projects using Claude Code as an AI coding assistant. Each repository runs in its own isolated container with persistent storage, authentication, and repository-defined setup logic.

## crowdcontrol Development Setup

### Prerequisites

- Rust (latest stable version)
- Docker Desktop or Docker Engine
- Git

### Initial Setup

1. **Clone crowdcontrol repository**:

   ```bash
   git clone git@github.com:yourorg/crowdcontrol.git
   cd crowdcontrol
   ```

2. **Build the CLI tool**:

   ```bash
   cargo build --release
   ```

3. **Install the CLI globally**:

   ```bash
   cargo install --path .
   # Or symlink for development:
   ln -s $(pwd)/target/release/crowdcontrol /usr/local/bin/crowdcontrol
   ```

4. **Verify installation**:
   ```bash
   crowdcontrol --help
   ```

### Development Workflow

```bash
# Make changes to the CLI code
cargo build

# Test locally
./target/debug/crowdcontrol --help

# Run tests
cargo test

# Install updated version
cargo install --path .
```

## Architecture Goals

- **Parallel Development**: Run multiple isolated development environments simultaneously
- **Resource Efficiency**: Start/stop containers as needed to manage resource usage
- **Repository Agnostic**: Single container image works across different technology stacks
- **Persistent Storage**: File changes, git history, and configurations persist across container lifecycles
- **Authentication Integration**: Seamless git and Claude Code authentication
- **Repository-Driven Setup**: Each repository defines its own setup, start, and stop logic through committed scripts

## System Components

### 1. Generic Container Image

A single, reusable Docker image that provides:

- Docker-in-Docker capability for running application stacks
- Claude Code CLI for AI-assisted development
- Git and SSH client for version control
- Common development tools and utilities
- Generic entrypoint that executes repository-defined scripts

### 2. Repository Configuration Scripts

Each repository defines its own setup through committed scripts in `.crowdcontrol/`:

- `.crowdcontrol/setup.sh` - One-time setup tasks (runs once per container)
- `.crowdcontrol/start.sh` - Startup tasks (runs every time container starts)
- `.crowdcontrol/stop.sh` - Cleanup tasks (runs when container stops)

### 3. crowdcontrol CLI Tool

A Rust-based command-line interface built with Clap that provides:

- Repository setup and cloning
- Container lifecycle management (start/stop/connect)
- Agent listing and status monitoring
- Global configuration management
- Shell completion support

## Implementation Details

### Container Image (Dockerfile)

```dockerfile
FROM ubuntu:22.04

# Install core dependencies
RUN apt-get update && apt-get install -y \
    docker.io \
    git ssh-client curl wget \
    jq unzip build-essential \
    python3 python3-pip \
    sudo vim nano \
    && rm -rf /var/lib/apt/lists/*

# Install docker-compose as standalone binary
RUN curl -L "https://github.com/docker/compose/releases/download/v2.24.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose \
    && chmod +x /usr/local/bin/docker-compose

# Install Node.js 22
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs

# Install Claude Code
RUN npm install -g @anthropic-ai/claude-code

# Copy and configure entrypoint
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Set working directory
WORKDIR /workspace

# Use generic entrypoint
ENTRYPOINT ["/entrypoint.sh"]
```

### Generic Entrypoint Script

```bash
#!/bin/bash
# /entrypoint.sh

set -e

echo "Starting crowdcontrol development environment..."

# Start docker daemon in background
dockerd &

# Wait for docker to be ready
echo "Waiting for Docker daemon..."
timeout=30
while ! docker info >/dev/null 2>&1; do
    sleep 1
    timeout=$((timeout - 1))
    if [ $timeout -eq 0 ]; then
        echo "Docker daemon failed to start"
        exit 1
    fi
done

echo "Docker daemon ready"

# Find the repository directory (should be only subdirectory in /workspace)
REPO_DIR=$(find /workspace -maxdepth 1 -type d ! -path /workspace | head -1)

if [ -z "$REPO_DIR" ]; then
    echo "No repository directory found in /workspace"
    echo "Container ready for manual setup"
    tail -f /dev/null
fi

cd "$REPO_DIR"
REPO_NAME=$(basename "$REPO_DIR")
echo "Working in: $REPO_DIR"

# Export environment variables for scripts
export CROWDCONTROL_REPO_PATH="$REPO_DIR"
export CROWDCONTROL_REPO_NAME="$REPO_NAME"
export CROWDCONTROL_WORKSPACE="/workspace"

# Run repository-specific setup if it exists and hasn't been run
if [ -f ".crowdcontrol/setup.sh" ] && [ ! -f ".crowdcontrol/.setup-complete" ]; then
    echo "Running repository setup for $REPO_NAME..."
    chmod +x .crowdcontrol/setup.sh
    ./.crowdcontrol/setup.sh
    if [ $? -eq 0 ]; then
        touch .crowdcontrol/.setup-complete
        echo "Setup completed successfully"
    else
        echo "Setup failed"
        exit 1
    fi
fi

# Run repository-specific start script if it exists
if [ -f ".crowdcontrol/start.sh" ]; then
    echo "Running repository start script for $REPO_NAME..."
    chmod +x .crowdcontrol/start.sh
    ./.crowdcontrol/start.sh
    if [ $? -eq 0 ]; then
        echo "Repository services started successfully"
    else
        echo "Start script failed"
        exit 1
    fi
fi

echo "Container ready for development"
echo "Connect with: docker exec -it \$CONTAINER_NAME claude --dangerously-skip-permissions"

# Keep container running
tail -f /dev/null
```

### crowdcontrol CLI Implementation

The crowdcontrol CLI is implemented as a Rust application using the Clap derive API. Clap provides ergonomic argument parsing with automatic help generation and subcommand support.

#### Project Structure

```
crowdcontrol/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point and command definitions
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── new.rs        # Create command implementation
│   │   ├── start.rs         # Start command implementation
│   │   ├── stop.rs          # Stop command implementation
│   │   ├── connect.rs       # Connect command implementation
│   │   └── list.rs          # List command implementation
│   ├── config.rs            # Configuration management
│   ├── docker.rs            # Docker operations
│   └── utils.rs             # Shared utilities
└── container/
    ├── Dockerfile           # Container image definition
    └── entrypoint.sh        # Container entrypoint script
```

#### Cargo.toml

```toml
[package]
name = "crowdcontrol"
version = "0.1.0"
edition = "2021"
description = "Containerized development environment manager with Claude Code integration"
authors = ["Your Team <team@yourorg.com>"]
license = "MIT OR Apache-2.0"

[[bin]]
name = "crowdcontrol"
path = "src/main.rs"

[dependencies]
clap = { version = "4.5", features = ["derive", "env", "cargo"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
dirs = "5.0"
which = "6.0"
bollard = "0.16"  # Docker API client
colored = "2.0"
indicatif = "0.17"  # Progress bars
chrono = { version = "0.4", features = ["serde"] }

[dependencies.uuid]
version = "1.0"
features = ["v4", "serde"]
```

#### CLI Structure (src/main.rs)

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod docker;
mod utils;

use commands::*;
use config::Config;

/// crowdcontrol: Containerized development environments with Claude Code
#[derive(Parser)]
#[command(
    name = "crowdcontrol",
    version,
    about = "Manage containerized development environments with Claude Code integration",
    long_about = "crowdcontrol enables parallel development across multiple repositories using \
                  isolated Docker containers with Claude Code as an AI coding assistant."
)]
struct Cli {
    /// Global configuration options
    #[command(flatten)]
    global: GlobalOptions,

    /// Available commands
    #[command(subcommand)]
    command: Commands,
}

/// Global configuration options available to all commands
#[derive(Parser)]
struct GlobalOptions {
    /// Custom workspaces directory
    #[arg(
        long,
        env = "CROWDCONTROL_WORKSPACES_DIR",
        global = true,
        help = "Directory for storing agent workspaces"
    )]
    workspaces_dir: Option<PathBuf>,

    /// Custom container image name
    #[arg(
        long,
        env = "CROWDCONTROL_IMAGE",
        global = true,
        default_value = "crowdcontrol:latest",
        help = "Docker image to use for agents"
    )]
    image: String,

    /// Enable verbose output
    #[arg(
        short,
        long,
        global = true,
        action = clap::ArgAction::Count,
        help = "Increase verbosity (can be used multiple times)"
    )]
    verbose: u8,

    /// Disable colored output
    #[arg(
        long,
        env = "NO_COLOR",
        global = true,
        help = "Disable colored output"
    )]
    no_color: bool,
}

/// Available subcommands
#[derive(Subcommand)]
enum Commands {
    /// Set up a new agent from a git repository
    Setup(SetupArgs),

    /// Start an existing agent
    Start(StartArgs),

    /// Stop a running agent
    Stop(StopArgs),

    /// Connect to a running agent with Claude Code
    Connect(ConnectArgs),

    /// List all agents and their status
    List(ListArgs),

    /// Remove an agent and its workspace
    Remove(RemoveArgs),

    /// Show agent logs
    Logs(LogsArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize configuration
    let config = Config::new(cli.global)?;

    // Execute the appropriate command
    match cli.command {
        Commands::New(args) => new::execute(config, args).await,
        Commands::Start(args) => start::execute(config, args).await,
        Commands::Stop(args) => stop::execute(config, args).await,
        Commands::Connect(args) => connect::execute(config, args).await,
        Commands::List(args) => list::execute(config, args).await,
        Commands::Remove(args) => remove::execute(config, args).await,
        Commands::Logs(args) => logs::execute(config, args).await,
        Commands::Completions(args) => completions::execute(config, args).await,
    }
}
```

#### Command Argument Structures

```rust
use clap::Args;
use std::path::PathBuf;

/// Arguments for the new command
#[derive(Args)]
pub struct SetupArgs {
    /// Name for the agent (must be unique)
    #[arg(help = "Unique name for the agent")]
    name: String,

    /// Git repository URL to clone
    #[arg(help = "Git repository URL (ssh format: git@github.com:org/repo.git)")]
    repository: String,

    /// Custom branch to checkout
    #[arg(
        short,
        long,
        help = "Git branch to checkout (defaults to repository default)"
    )]
    branch: Option<String>,

    /// Skip repository verification
    #[arg(
        long,
        help = "Skip checking if repository contains .crowdcontrol/ directory"
    )]
    skip_verification: bool,

    /// Custom resource limits
    #[arg(long, help = "Memory limit (e.g., 2g, 1024m)")]
    memory: Option<String>,

    #[arg(long, help = "CPU limit (e.g., 1.5, 2)")]
    cpus: Option<String>,
}

/// Arguments for the start command
#[derive(Args)]
pub struct StartArgs {
    /// Name of the agent to start
    #[arg(help = "Name of the agent to start")]
    name: String,

    /// Wait for agent to be ready before returning
    #[arg(
        short,
        long,
        help = "Wait for agent initialization to complete"
    )]
    wait: bool,

    /// Timeout for waiting (in seconds)
    #[arg(
        long,
        default_value = "60",
        requires = "wait",
        help = "Timeout for wait operation"
    )]
    timeout: u64,
}

/// Arguments for the stop command
#[derive(Args)]
pub struct StopArgs {
    /// Name of the agent to stop
    #[arg(help = "Name of the agent to stop")]
    name: Option<String>,

    /// Stop all running agents
    #[arg(
        long,
        conflicts_with = "name",
        help = "Stop all running agents"
    )]
    all: bool,

    /// Force stop (kill instead of graceful shutdown)
    #[arg(
        short,
        long,
        help = "Force stop the agent (SIGKILL)"
    )]
    force: bool,
}

/// Arguments for the connect command
#[derive(Args)]
pub struct ConnectArgs {
    /// Name of the agent to connect to
    #[arg(help = "Name of the agent to connect to")]
    name: String,

    /// Command to run instead of Claude Code
    #[arg(
        short,
        long,
        help = "Custom command to run (defaults to 'claude --dangerously-skip-permissions')"
    )]
    command: Option<String>,

    /// Run command in the background
    #[arg(
        short,
        long,
        help = "Run command in background and return immediately"
    )]
    detach: bool,
}

/// Arguments for the list command
#[derive(Args)]
pub struct ListArgs {
    /// Show all agents (including stopped)
    #[arg(
        short,
        long,
        help = "Show all agents, including stopped ones"
    )]
    all: bool,

    /// Output format
    #[arg(
        long,
        value_enum,
        default_value = "table",
        help = "Output format"
    )]
    format: OutputFormat,

    /// Filter agents by status
    #[arg(
        long,
        value_enum,
        help = "Filter agents by status"
    )]
    status: Option<AgentStatus>,
}

/// Arguments for the remove command
#[derive(Args)]
pub struct RemoveArgs {
    /// Name of the agent to remove
    #[arg(help = "Name of the agent to remove")]
    name: String,

    /// Remove without confirmation
    #[arg(
        short,
        long,
        help = "Remove without confirmation prompt"
    )]
    force: bool,

    /// Keep workspace directory
    #[arg(
        long,
        help = "Keep the workspace directory (only remove container)"
    )]
    keep_workspace: bool,
}

/// Arguments for the logs command
#[derive(Args)]
pub struct LogsArgs {
    /// Name of the agent
    #[arg(help = "Name of the agent")]
    name: String,

    /// Follow log output
    #[arg(
        short,
        long,
        help = "Follow log output (like tail -f)"
    )]
    follow: bool,

    /// Number of lines to show
    #[arg(
        short,
        long,
        default_value = "50",
        help = "Number of lines to show from the end"
    )]
    tail: u32,

    /// Show timestamps
    #[arg(
        short,
        long,
        help = "Show timestamps"
    )]
    timestamps: bool,
}

/// Arguments for the completions command
#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(help = "Shell to generate completions for")]
    shell: clap_complete::Shell,
}

/// Output format options
#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

/// Agent status options
#[derive(clap::ValueEnum, Clone)]
pub enum AgentStatus {
    Running,
    Stopped,
    Created,
    Error,
}
```

#### Configuration Management (src/config.rs)

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::GlobalOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspaces_dir: PathBuf,
    pub image: String,
    pub verbose: u8,
    pub no_color: bool,
}

impl Config {
    pub fn new(global: GlobalOptions) -> Result<Self> {
        let workspaces_dir = global.workspaces_dir
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("Unable to determine home directory")
                    .join("crowdcontrol-workspaces")
            });

        // Ensure workspaces directory exists
        fs::create_dir_all(&workspaces_dir)
            .with_context(|| format!("Failed to create workspaces directory: {:?}", workspaces_dir))?;

        Ok(Config {
            workspaces_dir,
            image: global.image,
            verbose: global.verbose,
            no_color: global.no_color,
        })
    }

    pub fn agent_workspace_path(&self, name: &str) -> PathBuf {
        self.workspaces_dir.join(name)
    }
}
```

## Global Configuration

crowdcontrol uses environment variables for global configuration that applies to all commands:

### Environment Variables

| Variable                      | Default                     | Description                            |
| ----------------------------- | --------------------------- | -------------------------------------- |
| `CROWDCONTROL_WORKSPACES_DIR` | `~/crowdcontrol-workspaces` | Directory for storing agent workspaces |
| `CROWDCONTROL_IMAGE`          | `crowdcontrol:latest`       | Docker image to use for agents         |
| `NO_COLOR`                    | `false`                     | Disable colored output                 |

### Configuration Precedence

1. Command-line flags (highest priority)
2. Environment variables
3. Default values (lowest priority)

### Example Configuration

```bash
# In ~/.bashrc or ~/.zshrc
export CROWDCONTROL_WORKSPACES_DIR="/custom/workspaces/path"
export CROWDCONTROL_IMAGE="crowdcontrol:v1.2.3"

# Override for specific command
crowdcontrol --workspaces-dir /tmp/workspaces list
```

## Repository Configuration Example

### Node.js + PostgreSQL Repository

**`.crowdcontrol/setup.sh`**

```bash
#!/bin/bash
echo "Setting up Node.js project with PostgreSQL..."
echo "Repository path: $CROWDCONTROL_REPO_PATH"
echo "Repository name: $CROWDCONTROL_REPO_NAME"

# Install project-specific tools
npm install -g prisma @nestjs/cli typescript

# Install dependencies
if [ -f "package.json" ]; then
    npm install
fi

# Generate Prisma client if schema exists
if [ -f "prisma/schema.prisma" ]; then
    npx prisma generate
fi

echo "Node.js project setup complete"
```

**`.crowdcontrol/start.sh`**

```bash
#!/bin/bash
echo "Starting Node.js project services..."
echo "Working in: $CROWDCONTROL_REPO_PATH"

# Start application stack using docker-compose from repo
docker-compose up -d

# Wait for PostgreSQL
echo "Waiting for PostgreSQL..."
sleep 10

# Run database migrations
if [ -f "prisma/schema.prisma" ]; then
    npx prisma migrate deploy
fi

echo "Node.js project ready!"
echo "PostgreSQL: localhost:5432"
echo "Application services started"
```

**`.crowdcontrol/stop.sh`**

```bash
#!/bin/bash
echo "Stopping Node.js project services..."

# Stop application stack
docker-compose down

echo "Services stopped"
```

**Available Environment Variables in Scripts:**

- `CROWDCONTROL_REPO_PATH` - Full path to the cloned repository
- `CROWDCONTROL_REPO_NAME` - Name of the repository directory
- `CROWDCONTROL_WORKSPACE` - Path to the workspace directory (/workspace)

### Agent Naming Conventions

Since multiple agents typically work on the same repository, use descriptive naming patterns:

**Recommended Patterns:**

```bash
# Branch-based naming
crowdcontrol new myapp-main git@github.com:org/myapp.git
crowdcontrol new myapp-feature-auth git@github.com:org/myapp.git --branch feature/auth
crowdcontrol new myapp-hotfix-123 git@github.com:org/myapp.git --branch hotfix/issue-123

# Task-based naming
crowdcontrol new myapp-frontend git@github.com:org/myapp.git
crowdcontrol new myapp-backend git@github.com:org/myapp.git
crowdcontrol new myapp-testing git@github.com:org/myapp.git

# Developer-based naming (for team environments)
crowdcontrol new myapp-alice-feature git@github.com:org/myapp.git --branch alice/new-feature
crowdcontrol new myapp-bob-bugfix git@github.com:org/myapp.git --branch bob/bugfix
```

## Usage Workflows

### Initial Setup

### Initial Setup

```bash
# 1. Build the container image
docker build -t crowdcontrol:latest ./container/

# 2. Install crowdcontrol CLI (see development setup section above)
cargo install --path .

# 3. Set up a new agent (automatically clones from git)
crowdcontrol new ecommerce-api-main git@github.com:myorg/ecommerce-api.git

# 4. Start the agent
crowdcontrol start ecommerce-api-main

# 5. Connect and start developing
crowdcontrol connect ecommerce-api-main
```

### Daily Development Workflow

```bash
# Start agents for different features of the same repository
crowdcontrol start ecommerce-api-main
crowdcontrol start ecommerce-api-auth-feature
crowdcontrol start ecommerce-api-bugfix-123

# Connect to work on specific feature
crowdcontrol connect ecommerce-api-auth-feature
# (Opens Claude Code in that feature's container)

# When done, stop to save resources
crowdcontrol stop ecommerce-api-auth-feature
```

### Managing Multiple Agents

```bash
# Set up multiple agents for the same repository on different branches
crowdcontrol new ecommerce-api-main git@github.com:myorg/ecommerce-api.git
crowdcontrol new ecommerce-api-auth-feature git@github.com:myorg/ecommerce-api.git --branch feature/auth-system
crowdcontrol new ecommerce-api-bugfix git@github.com:myorg/ecommerce-api.git --branch bugfix/payment-issue

# Set up agents for different repositories when needed
crowdcontrol new frontend-react-main git@github.com:myorg/frontend-react.git
crowdcontrol new docs-site git@github.com:myorg/documentation.git

# List all agents with status
crowdcontrol list

# List all agents in JSON format
crowdcontrol list --format json

# List only running agents
crowdcontrol list --status running

# List all agents including stopped ones
crowdcontrol list --all

# Start multiple agents for parallel development
crowdcontrol start ecommerce-api-main
crowdcontrol start ecommerce-api-auth-feature
crowdcontrol start ecommerce-api-bugfix

# Stop all agents
crowdcontrol stop --all

# View agent logs
crowdcontrol logs ecommerce-api-main
crowdcontrol logs --follow ecommerce-api-auth-feature

# Remove an agent and its workspace
crowdcontrol remove ecommerce-api-bugfix
```

### CLI Help and Completions

```bash
# Get help for main command
crowdcontrol --help

# Get help for specific subcommand
crowdcontrol new --help
crowdcontrol list --help

# Generate shell completions
crowdcontrol completions bash > /etc/bash_completion.d/crowdcontrol
crowdcontrol completions zsh > ~/.zsh/completions/_crowdcontrol
crowdcontrol completions fish > ~/.config/fish/completions/crowdcontrol.fish
```

## Directory Structure

```
Host Machine:
~/crowdcontrol-workspaces/
├── ecommerce-api-main/            # Main branch workspace
│   ├── .crowdcontrol/
│   │   ├── setup.sh
│   │   ├── start.sh
│   │   ├── stop.sh
│   │   └── .setup-complete
│   ├── docker-compose.yml
│   ├── src/
│   └── ...
├── ecommerce-api-auth/            # Auth feature branch workspace
│   ├── .crowdcontrol/
│   ├── src/
│   └── ...
├── ecommerce-api-bugfix/          # Bugfix branch workspace
│   ├── .crowdcontrol/
│   ├── src/
│   └── ...
└── frontend-react-main/           # Different repository
    ├── .crowdcontrol/
    ├── package.json
    └── ...

Container View:
/workspace/
└── <agent-name>/                  # Unique per container
    ├── .crowdcontrol/
    ├── [repository files]
    └── ...
```

## Technical Requirements

### Host System Requirements

- Docker Desktop or Docker Engine with Docker Compose
- Minimum 8GB RAM (16GB recommended for multiple agents)
- SSH keys configured for git authentication
- Claude Code authentication configured

### Container Resource Limits

```bash
# Recommended resource limits per container
docker create --privileged \
    --memory=2g \
    --cpus=2 \
    --name $AGENT_ID \
    [other options...]
```

### Network Considerations

- Each container runs its own Docker daemon
- Application services (PostgreSQL, Redis, etc.) bind to container-internal networks only
- No host port conflicts between containers
- Claude Code connects to Anthropic's API from within containers

## Security Considerations

- SSH keys are mounted read-only
- Claude Code runs with `--dangerously-skip-permissions` within container isolation
- Each container has its own Docker daemon and network namespace
- Host Docker socket is shared (required for Docker-in-Docker)

## Troubleshooting

### Common Issues

1. **Container fails to start**: Check Docker daemon logs with `docker logs <agent-id>`
2. **Setup script fails**: Verify `.container/setup.sh` has proper permissions and syntax
3. **Services don't start**: Check `docker-compose.yml` syntax and port conflicts
4. **Authentication issues**: Verify SSH keys and Claude Code configs are properly mounted
5. **Resource issues**: Monitor memory usage and adjust container limits

### Debugging Commands

```bash
# Check container logs
crowdcontrol logs <agent-name>

# Follow logs in real-time
crowdcontrol logs --follow <agent-name>

# Connect to container without Claude Code
crowdcontrol connect <agent-name> --command /bin/bash

# Check Docker daemon status inside container
crowdcontrol connect <agent-name> --command "docker info"

# Check repository services
crowdcontrol connect <agent-name> --command "docker-compose ps"

# Get detailed agent information
crowdcontrol list --format json | jq '.[] | select(.name == "<agent-name>")'
```

## Extension Points

### Additional Authentication

Mount additional config directories as needed:

```bash
-v ~/.aws:/root/.aws:ro \
-v ~/.kube:/root/.kube:ro \
```

### Custom Tools

Add tool installation to project setup scripts:

```bash
# In .container/setup.sh
curl -sSL https://install.python-poetry.org | python3 -
```

### Monitoring

Add container monitoring:

```bash
# Check resource usage
docker stats --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}"
```

## Implementation Timeline

1. **Week 1**: Build base container image and entrypoint script
2. **Week 2**: Create container management scripts and test basic functionality
3. **Week 3**: Test with multiple project types and refine setup patterns
4. **Week 4**: Documentation, troubleshooting guides, and team training

## Success Criteria

- ✅ Multiple containers can run simultaneously without conflicts
- ✅ File changes persist across container restarts
- ✅ Git and Claude Code authentication work seamlessly
- ✅ Repository-specific setup scripts execute correctly
- ✅ Claude Code conversations are properly isolated by repository
- ✅ Resource usage is manageable with start/stop workflow
- ✅ Team can onboard new repositories using the same container pattern
- ✅ CLI provides intuitive subcommand interface (`crowdcontrol <command>`)
- ✅ Environment variables enable global configuration
- ✅ List command shows clear agent status and information
- ✅ Shell completions work for improved UX
- ✅ Error messages are helpful and actionable
- ✅ Repository-driven configuration through `.crowdcontrol/` directory
