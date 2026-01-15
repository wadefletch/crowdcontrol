use anyhow::Result;
use clap::{Args, Subcommand};

use crate::utils::*;
use crowdcontrol_core::{validate_slug, Config, Repo, RepoStore};

/// Manage repository definitions
#[derive(Args)]
pub struct RepoCommand {
    #[command(subcommand)]
    pub command: RepoSubcommand,
}

#[derive(Subcommand)]
pub enum RepoSubcommand {
    /// Add a new repository
    Add(RepoAddArgs),
    /// List all repositories
    List(RepoListArgs),
    /// Remove a repository
    Remove(RepoRemoveArgs),
}

#[derive(Args)]
pub struct RepoAddArgs {
    /// Short slug for the repository (e.g., "myapp")
    #[arg(help = "Short slug for the repository")]
    pub slug: String,

    /// Git repository URL
    #[arg(help = "Git repository URL (e.g., git@github.com:org/repo.git)")]
    pub url: String,

    /// Default branch to use when creating agents
    #[arg(long, help = "Default branch to checkout when creating agents")]
    pub default_branch: Option<String>,
}

#[derive(Args)]
pub struct RepoListArgs {
    /// Output format
    #[arg(long, value_enum, default_value = "table", help = "Output format")]
    pub format: OutputFormat,
}

#[derive(Args)]
pub struct RepoRemoveArgs {
    /// Slug of the repository to remove
    #[arg(help = "Slug of the repository to remove")]
    pub slug: String,
}

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Table,
    Json,
}

fn get_repos_path() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("Unable to determine config directory")
        .join("crowdcontrol")
        .join("repos.toml")
}

pub async fn execute(_config: Config, args: RepoCommand) -> Result<()> {
    match args.command {
        RepoSubcommand::Add(add_args) => execute_add(add_args),
        RepoSubcommand::List(list_args) => execute_list(list_args),
        RepoSubcommand::Remove(remove_args) => execute_remove(remove_args),
    }
}

fn execute_add(args: RepoAddArgs) -> Result<()> {
    // Validate slug
    validate_slug(&args.slug)?;

    let repos_path = get_repos_path();
    let mut store = RepoStore::new(repos_path);

    let repo = Repo {
        slug: args.slug.clone(),
        url: args.url.clone(),
        default_branch: args.default_branch.clone(),
    };

    store.add(repo)?;

    print_success(&format!("Added repo '{}' -> {}", args.slug, args.url));

    if let Some(branch) = args.default_branch {
        print_info(&format!("Default branch: {}", branch));
    }

    Ok(())
}

fn execute_list(args: RepoListArgs) -> Result<()> {
    let repos_path = get_repos_path();
    let store = RepoStore::new(repos_path);
    let repos = store.list()?;

    if repos.is_empty() {
        match args.format {
            OutputFormat::Table => {
                print_info("No repos found");
                print_info("Add one with: crowdcontrol repo add <slug> <url>");
            }
            OutputFormat::Json => {
                println!("[]");
            }
        }
        return Ok(());
    }

    match args.format {
        OutputFormat::Table => {
            println!(
                "{:<15} {:<50} {}",
                "SLUG", "URL", "DEFAULT BRANCH"
            );
            println!("{}", "-".repeat(80));
            for repo in repos {
                println!(
                    "{:<15} {:<50} {}",
                    repo.slug,
                    repo.url,
                    repo.default_branch.as_deref().unwrap_or("-")
                );
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&repos)?;
            println!("{}", json);
        }
    }

    Ok(())
}

fn execute_remove(args: RepoRemoveArgs) -> Result<()> {
    let repos_path = get_repos_path();
    let mut store = RepoStore::new(repos_path);

    store.remove(&args.slug)?;

    print_success(&format!("Removed repo '{}'", args.slug));

    Ok(())
}
