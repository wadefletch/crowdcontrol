# CrowdControl

CrowdControl is a containerized development environment manager that enables parallel development across multiple repositories using isolated Docker containers with Claude Code as an AI coding assistant.

## Architecture

The project is organized as a Rust workspace with two crates:

- `crowdcontrol-core`: Core functionality for Docker management, agent operations, and configuration
- `crowdcontrol-cli`: Command-line interface built on top of the core library

This architecture allows for easy extension with additional interfaces (e.g., HTTP API, GUI) in the future.

## Features

- **Parallel Development**: Run multiple isolated development environments simultaneously
- **Resource Efficiency**: Start/stop containers as needed to manage resource usage
- **Repository Agnostic**: Single container image works across different technology stacks
- **Persistent Storage**: File changes, git history, and configurations persist across container lifecycles
- **Authentication Integration**: Seamless git and Claude Code authentication
- **Repository-Driven Setup**: Each repository defines its own setup logic through committed scripts

## Prerequisites

- Rust (latest stable version)
- Docker Desktop or Docker Engine
- Git

## Installation

### Option 1: npm/pnpm (Cross-platform)

```bash
# Install globally via npm
npm install -g @wadefletch/crowdcontrol

# Or with pnpm
pnpm install -g @wadefletch/crowdcontrol

# Pull the Docker image
docker pull crowdcontrol/crowdcontrol:latest

# Verify installation
crowdcontrol --help
```

### Option 2: Cargo (Rust developers)

```bash
# Install via Cargo
cargo install crowdcontrol-cli

# Pull the Docker image
docker pull crowdcontrol/crowdcontrol:latest

# Verify installation
crowdcontrol --help
```

### Option 3: Download Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/wadefletch/crowdcontrol/releases).

```bash
# Example for macOS (replace with your platform)
curl -L -o crowdcontrol.tar.gz https://github.com/wadefletch/crowdcontrol/releases/latest/download/crowdcontrol-aarch64-apple-darwin.tar.gz
tar -xzf crowdcontrol.tar.gz
sudo mv crowdcontrol /usr/local/bin/

# Pull the Docker image
docker pull crowdcontrol/crowdcontrol:latest

# Verify installation
crowdcontrol --help
```

### Option 4: Build from Source

1. **Clone the repository**:

   ```bash
   git clone git@github.com:wadefletch/crowdcontrol.git
   cd crowdcontrol
   ```

2. **Build the Docker image** (if you want to customize):

   ```bash
   docker build -t crowdcontrol:latest ./container/
   ```

3. **Build and install the CLI**:

   ```bash
   cargo build --release
   cargo install --path crowdcontrol-cli
   ```

4. **Verify installation**:

   ```bash
   crowdcontrol --help
   ```

### Shell Completions (Optional)

```bash
# For bash
crowdcontrol completions bash > /etc/bash_completion.d/crowdcontrol

# For zsh
crowdcontrol completions zsh > ~/.zsh/completions/_crowdcontrol

# For fish
crowdcontrol completions fish > ~/.config/fish/completions/crowdcontrol.fish
```

## Usage

### Creating a new agent

```bash
# Clone a repository and create a new agent
crowdcontrol new myapp-main git@github.com:org/myapp.git

# Clone with a specific branch
crowdcontrol new myapp-feature git@github.com:org/myapp.git --branch feature/auth

# Set custom resource limits
crowdcontrol new myapp-test git@github.com:org/myapp.git --memory 4g --cpus 2
```

### Managing agents

```bash
# Start an agent
crowdcontrol start myapp-main

# Connect to an agent with Claude Code
crowdcontrol connect myapp-main

# Stop an agent
crowdcontrol stop myapp-main

# Stop all running agents
crowdcontrol stop --all

# List all agents
crowdcontrol list

# View agent logs
crowdcontrol logs myapp-main

# Remove an agent
crowdcontrol remove myapp-main
```

### Configuration

CrowdControl supports configuration through multiple sources, with the following priority order (highest to lowest):

1. **Command-line arguments** - Override any other settings
2. **Environment variables** - Use `CROWDCONTROL_` prefix
3. **Config file** - `~/.config/crowdcontrol/config.toml`
4. **Default values**

#### Config File

Create a configuration file at `~/.config/crowdcontrol/config.toml`:

```toml
# Directory for storing agent workspaces
workspaces_dir = "~/custom-workspaces"

# Docker image to use for agents
image = "crowdcontrol:custom"

# Default resource limits for new agents
default_memory = "4g"
default_cpus = "2"

# Verbosity level (0-2)
verbose = 1
```

See `config.example.toml` for a complete example.

#### Environment Variables

| Variable                      | Default                     | Description                            |
| ----------------------------- | --------------------------- | -------------------------------------- |
| `CROWDCONTROL_WORKSPACES_DIR` | `~/crowdcontrol-workspaces` | Directory for storing agent workspaces |
| `CROWDCONTROL_IMAGE`          | `crowdcontrol:latest`       | Docker image to use for agents         |
| `CROWDCONTROL_DEFAULT_MEMORY` | None                        | Default memory limit for agents        |
| `CROWDCONTROL_DEFAULT_CPUS`   | None                        | Default CPU limit for agents           |
| `NO_COLOR`                    | `false`                     | Disable colored output                 |

## Repository Configuration

Repositories can define their own setup logic by creating a `.crowdcontrol/` directory with these optional scripts:

- **`.crowdcontrol/setup.sh`** - One-time setup tasks (runs once per container)
- **`.crowdcontrol/start.sh`** - Startup tasks (runs every time container starts)
- **`.crowdcontrol/stop.sh`** - Cleanup tasks (runs when container stops)

### Example Repository Configuration

**`.crowdcontrol/setup.sh`**

```bash
#!/bin/bash
echo "Setting up Node.js project..."

# Install global tools
npm install -g prisma @nestjs/cli

# Install dependencies
npm install

# Generate Prisma client
npx prisma generate

echo "Setup complete"
```

**`.crowdcontrol/start.sh`**

```bash
#!/bin/bash
echo "Starting services..."

# Start database
docker-compose up -d postgres

# Run migrations
npx prisma migrate deploy

echo "Services ready"
```

### Available Environment Variables in Scripts

- `CROWDCONTROL_REPO_PATH` - Full path to the cloned repository
- `CROWDCONTROL_REPO_NAME` - Name of the repository directory
- `CROWDCONTROL_WORKSPACE` - Path to the workspace directory (/workspace)

## Development

### Setting up the Development Environment

```bash
git clone git@github.com:wadefletch/crowdcontrol.git
cd crowdcontrol
cargo build
```

### Development Workflow

This project follows [Conventional Commits](https://www.conventionalcommits.org/) for automated versioning and changelog generation. Supported scopes: `core`, `cli`, `ci`.

#### Making Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
git commit -m "feat(core): add agent status monitoring"
git commit -m "fix(cli): resolve container startup timeout"
git commit -m "docs: update installation instructions"
```

### Testing

The test suite is organized into different categories:

```bash
# Run unit tests (fast, no Docker required)
cargo test

# Run integration tests (requires Docker, run serially) - using alias
cargo test-integration

# Run all Docker-dependent tests including user journey tests - using alias
cargo test-docker

# Alternative: run integration tests manually
cargo test --test integration_test -- --ignored --test-threads=1

# Alternative: run all Docker tests manually
cargo test -- --ignored --test-threads=1
```

**Note**: Integration tests require Docker to be running and the `crowdcontrol:latest` image to be built. Tests that require Docker are marked with `#[ignore]` so they don't run during regular `cargo test` execution.

**Cargo Aliases**: The project includes helpful cargo aliases defined in `.cargo/config.toml`:
- `cargo test-integration` - Run only Docker integration tests
- `cargo test-docker` - Run all Docker-dependent tests

### Releases

Releases are automated using [Cocogitto](https://docs.cocogitto.io/):

1. **Conventional commits** on `main` trigger automatic version bumping
2. **Cocogitto** generates changelog and creates GitHub releases  
3. **GitHub Actions** build and publish to:
   - Cargo (crates.io)
   - npm (@wadefletch/crowdcontrol)
   - GitHub releases (cross-platform binaries)

Releases happen automatically when conventional commits are pushed to `main`.

### Project Structure

```
crowdcontrol/
├── crowdcontrol-core/       # Core library crate
├── crowdcontrol-cli/        # CLI crate  
├── container/               # Docker container definition
├── npm/                     # npm package distribution
├── .github/workflows/       # CI/CD workflows
├── .cargo/config.toml       # Cargo aliases
├── cog.toml                 # Cocogitto configuration
├── install.sh               # Installation script
└── Cargo.toml               # Workspace configuration
```

## Troubleshooting

### Container fails to start

```bash
# Check logs
crowdcontrol logs <agent-name>

# Connect with bash for debugging
crowdcontrol connect <agent-name> --command /bin/bash
```

### Permission issues

Ensure your SSH keys and git config are properly set up in your home directory. crowdcontrol automatically mounts these as read-only volumes.

### Resource constraints

Adjust memory and CPU limits when creating agents:

```bash
crowdcontrol new myapp --memory 4g --cpus 2 git@github.com:org/myapp.git
```

## License

MIT OR Apache-2.0
