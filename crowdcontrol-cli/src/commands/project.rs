use anyhow::Result;
use clap::{Args, Subcommand};

use crate::utils::*;
use crowdcontrol_core::{validate_slug, Config, Project, ProjectStore};

/// Manage project definitions
#[derive(Args)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub command: ProjectSubcommand,
}

#[derive(Subcommand)]
pub enum ProjectSubcommand {
    /// Add a new project
    Add(ProjectAddArgs),
    /// List all projects
    List(ProjectListArgs),
    /// Remove a project
    Remove(ProjectRemoveArgs),
}

#[derive(Args)]
pub struct ProjectAddArgs {
    /// Short slug for the project (e.g., "myapp")
    #[arg(help = "Short slug for the project")]
    pub slug: String,

    /// Git repository URL
    #[arg(help = "Git repository URL (e.g., git@github.com:org/repo.git)")]
    pub url: String,

    /// Default branch to use when creating agents
    #[arg(long, help = "Default branch to checkout when creating agents")]
    pub default_branch: Option<String>,
}

#[derive(Args)]
pub struct ProjectListArgs {
    /// Output format
    #[arg(long, value_enum, default_value = "table", help = "Output format")]
    pub format: OutputFormat,
}

#[derive(Args)]
pub struct ProjectRemoveArgs {
    /// Slug of the project to remove
    #[arg(help = "Slug of the project to remove")]
    pub slug: String,
}

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Table,
    Json,
}

fn get_projects_path() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("Unable to determine config directory")
        .join("crowdcontrol")
        .join("projects.toml")
}

pub async fn execute(_config: Config, args: ProjectCommand) -> Result<()> {
    match args.command {
        ProjectSubcommand::Add(add_args) => execute_add(add_args),
        ProjectSubcommand::List(list_args) => execute_list(list_args),
        ProjectSubcommand::Remove(remove_args) => execute_remove(remove_args),
    }
}

fn execute_add(args: ProjectAddArgs) -> Result<()> {
    // Validate slug
    validate_slug(&args.slug)?;

    let projects_path = get_projects_path();
    let mut store = ProjectStore::new(projects_path);

    let project = Project {
        slug: args.slug.clone(),
        url: args.url.clone(),
        default_branch: args.default_branch.clone(),
    };

    store.add(project)?;

    print_success(&format!("Added project '{}' -> {}", args.slug, args.url));

    if let Some(branch) = args.default_branch {
        print_info(&format!("Default branch: {}", branch));
    }

    Ok(())
}

fn execute_list(args: ProjectListArgs) -> Result<()> {
    let projects_path = get_projects_path();
    let store = ProjectStore::new(projects_path);
    let projects = store.list()?;

    if projects.is_empty() {
        match args.format {
            OutputFormat::Table => {
                print_info("No projects found");
                print_info("Add one with: crowdcontrol project add <slug> <url>");
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
            for project in projects {
                println!(
                    "{:<15} {:<50} {}",
                    project.slug,
                    project.url,
                    project.default_branch.as_deref().unwrap_or("-")
                );
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&projects)?;
            println!("{}", json);
        }
    }

    Ok(())
}

fn execute_remove(args: ProjectRemoveArgs) -> Result<()> {
    let projects_path = get_projects_path();
    let mut store = ProjectStore::new(projects_path);

    store.remove(&args.slug)?;

    print_success(&format!("Removed project '{}'", args.slug));

    Ok(())
}
