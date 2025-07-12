use clap::Args;

pub mod completions;
pub mod connect;
pub mod doctor;
pub mod list;
pub mod logs;
pub mod new;
pub mod remove;
pub mod start;
pub mod stop;

/// Arguments for the new command
#[derive(Args)]
pub struct NewArgs {
    /// Name for the agent (must be unique)
    #[arg(help = "Unique name for the agent")]
    pub name: String,

    /// Git repository URL to clone
    #[arg(help = "Git repository URL (ssh format: git@github.com:org/repo.git)")]
    pub repository: String,

    /// Custom branch to checkout
    #[arg(
        short,
        long,
        help = "Git branch to checkout (defaults to repository default)"
    )]
    pub branch: Option<String>,

    /// Skip repository verification
    #[arg(
        long,
        help = "Skip checking if repository contains .crowdcontrol/ directory"
    )]
    pub skip_verification: bool,

    /// Custom resource limits
    #[arg(long, help = "Memory limit (e.g., 2g, 1024m)")]
    pub memory: Option<String>,

    #[arg(long, help = "CPU limit (e.g., 1.5, 2)")]
    pub cpus: Option<String>,
}

/// Arguments for the start command
#[derive(Args)]
pub struct StartArgs {
    /// Name of the agent to start
    #[arg(help = "Name of the agent to start")]
    pub name: String,

    /// Wait for agent to be ready before returning
    #[arg(short, long, help = "Wait for agent initialization to complete")]
    pub wait: bool,

    /// Timeout for waiting (in seconds)
    #[arg(
        long,
        default_value = "60",
        requires = "wait",
        help = "Timeout for wait operation"
    )]
    pub timeout: u64,
}

/// Arguments for the stop command
#[derive(Args)]
pub struct StopArgs {
    /// Name of the agent to stop
    #[arg(help = "Name of the agent to stop")]
    pub name: Option<String>,

    /// Stop all running agents
    #[arg(long, conflicts_with = "name", help = "Stop all running agents")]
    pub all: bool,

    /// Force stop (kill instead of graceful shutdown)
    #[arg(short, long, help = "Force stop the agent (SIGKILL)")]
    pub force: bool,
}

/// Arguments for the connect command
#[derive(Args)]
pub struct ConnectArgs {
    /// Name of the agent to connect to
    #[arg(help = "Name of the agent to connect to")]
    pub name: String,

    /// Command to run instead of Claude Code
    #[arg(
        short,
        long,
        help = "Custom command to run (defaults to 'claude --dangerously-skip-permissions')"
    )]
    pub command: Option<String>,

    /// Run command in the background
    #[arg(short, long, help = "Run command in background and return immediately")]
    pub detach: bool,
}

/// Arguments for the list command
#[derive(Args)]
pub struct ListArgs {
    /// Show all agents (including stopped)
    #[arg(short, long, help = "Show all agents, including stopped ones")]
    pub all: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "table", help = "Output format")]
    pub format: OutputFormat,

    /// Filter agents by status
    #[arg(long, value_enum, help = "Filter agents by status")]
    pub status: Option<AgentStatusFilter>,
}

/// Arguments for the remove command
#[derive(Args)]
pub struct RemoveArgs {
    /// Name of the agent to remove
    #[arg(help = "Name of the agent to remove")]
    pub name: String,

    /// Remove without confirmation
    #[arg(short, long, help = "Remove without confirmation prompt")]
    pub force: bool,

    /// Keep workspace directory
    #[arg(long, help = "Keep the workspace directory (only remove container)")]
    pub keep_workspace: bool,
}

/// Arguments for the logs command
#[derive(Args)]
pub struct LogsArgs {
    /// Name of the agent
    #[arg(help = "Name of the agent")]
    pub name: String,

    /// Follow log output
    #[arg(short, long, help = "Follow log output (like tail -f)")]
    pub follow: bool,

    /// Number of lines to show
    #[arg(
        short = 'n',
        long,
        default_value = "50",
        help = "Number of lines to show from the end"
    )]
    pub tail: u32,

    /// Show timestamps
    #[arg(short, long, help = "Show timestamps")]
    pub timestamps: bool,
}

/// Arguments for the completions command
#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(help = "Shell to generate completions for")]
    pub shell: clap_complete::Shell,
}

/// Output format options
#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

/// Agent status options for filtering
#[derive(clap::ValueEnum, Clone)]
pub enum AgentStatusFilter {
    Running,
    Stopped,
    Created,
    Error,
}
