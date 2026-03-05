use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod utils;

use commands::*;
use crowdcontrol_core::{init_logger, Loader};
use serde::Serialize;

/// CrowdControl: Containerized development environments with Claude Code
#[derive(Parser)]
#[command(
    name = "crowdcontrol",
    version,
    about = "Manage containerized development environments with Claude Code integration",
    long_about = "CrowdControl enables parallel development across multiple repositories using \
                  isolated Docker containers with Claude Code as an AI coding assistant."
)]
pub struct Cli {
    /// Global configuration options
    #[command(flatten)]
    global: GlobalOptions,

    /// Available commands
    #[command(subcommand)]
    command: Commands,
}

/// Global configuration options available to all commands
#[derive(Parser, Clone)]
pub struct GlobalOptions {
    /// Custom workspaces directory
    #[arg(
        long,
        env = "CC_WORKSPACES_DIR",
        global = true,
        help = "Directory for storing agent workspaces"
    )]
    pub workspaces_dir: Option<PathBuf>,

    /// Custom container image name
    #[arg(
        long,
        env = "CC_IMAGE",
        global = true,
        help = "Docker image to use for agents"
    )]
    pub image: Option<String>,

    /// Enable verbose output
    #[arg(
        short,
        long,
        global = true,
        action = clap::ArgAction::Count,
        help = "Increase verbosity (can be used multiple times)"
    )]
    pub verbose: u8,

    /// Disable colored output
    #[arg(long, env = "NO_COLOR", global = true, help = "Disable colored output")]
    pub no_color: bool,
}

/// Available subcommands
#[derive(Subcommand)]
enum Commands {
    /// Create a new agent from a git repository
    New(NewArgs),

    /// Create a new agent from a git repository (alias for new)
    Setup(NewArgs),

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

    /// Refresh Claude Code authentication for an agent
    Refresh(RefreshArgs),

    /// Show or manage configuration
    Config(config::ConfigCommand),

    /// Show detailed status for a specific agent
    Status(status::StatusCommand),

    /// Generate shell completions
    Completions(CompletionsArgs),

    /// Check and repair system state inconsistencies
    Doctor(doctor::DoctorCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logger based on verbosity level
    if let Err(e) = init_logger(cli.global.verbose) {
        eprintln!("Warning: Failed to initialize file logger: {}", e);
        eprintln!("Falling back to console-only logging");
        crowdcontrol_core::logger::init_env_logger(cli.global.verbose);
    }

    // Set up colored output early
    if cli.global.no_color {
        colored::control::set_override(false);
    }

    // Create CLI overrides struct
    #[derive(Serialize)]
    struct CliOverrides {
        #[serde(skip_serializing_if = "Option::is_none")]
        workspaces_dir: Option<PathBuf>,
        #[serde(skip_serializing_if = "Option::is_none")]
        image: Option<String>,
        verbose: u8,
    }

    let overrides = CliOverrides {
        workspaces_dir: cli.global.workspaces_dir,
        image: cli.global.image,
        verbose: cli.global.verbose,
    };

    // Load configuration with overrides
    let mut loader = Loader::new();
    if let Ok(config_file) = std::env::var("CC_CONFIG_FILE") {
        loader = loader.config_file(config_file);
    }
    
    let mut config = loader
        .merge(&overrides)
        .load()?;
    
    // Validate the configuration
    config.validate()?;

    // Execute the appropriate command
    match cli.command {
        Commands::New(args) => new::execute(config, args).await,
        Commands::Setup(args) => new::execute(config, args).await, // Alias for new
        Commands::Start(args) => start::execute(config, args).await,
        Commands::Stop(args) => stop::execute(config, args).await,
        Commands::Connect(args) => connect::execute(config, args).await,
        Commands::List(args) => list::execute(config, args).await,
        Commands::Remove(args) => remove::execute(config, args).await,
        Commands::Logs(args) => logs::execute(config, args).await,
        Commands::Refresh(args) => refresh::execute(config, args).await,
        Commands::Config(args) => config::execute(config, args).await,
        Commands::Status(args) => status::execute(config, args).await,
        Commands::Completions(args) => completions::execute(config, args).await,
        Commands::Doctor(args) => doctor::execute(config, args).await,
    }
}
