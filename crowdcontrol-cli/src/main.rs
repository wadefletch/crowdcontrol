use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod utils;

use commands::*;
use crowdcontrol_core::{init_logger, Config, Settings};

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
        env = "CROWDCONTROL_WORKSPACES_DIR",
        global = true,
        help = "Directory for storing agent workspaces"
    )]
    pub workspaces_dir: Option<PathBuf>,

    /// Custom container image name
    #[arg(
        long,
        env = "CROWDCONTROL_IMAGE",
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

    // Load settings with CLI overrides
    let settings = Settings::with_overrides(
        cli.global.workspaces_dir,
        cli.global.image,
        cli.global.verbose,
    )?;

    // Create config from settings
    let config = Config::from_settings(settings)?;

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
        Commands::Doctor(args) => doctor::execute(config, args).await,
    }
}
