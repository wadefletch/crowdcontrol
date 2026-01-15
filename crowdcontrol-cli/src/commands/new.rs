use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::fs;

use crate::commands::NewArgs;
use crate::utils::*;
use crowdcontrol_core::{
    clone_repository, parse_agent_identifier, save_agent_metadata, validate_agent_name,
    verify_repository_setup, Agent, AgentStatus, Config, DockerClient, ProjectStore,
};

fn get_projects_path() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("Unable to determine home directory")
        .join(".crowdcontrol")
        .join("projects.toml")
}

pub async fn execute(config: Config, args: NewArgs) -> Result<()> {
    // Parse the identifier to see if it's project:label format
    let identifier = parse_agent_identifier(&args.name)?;

    // Determine the repository URL and agent name based on the identifier
    let (filesystem_name, display_name, repository_url, project_slug, branch) =
        if let Some(slug) = &identifier.project_slug {
            // project:label format - look up the project
            let projects_path = get_projects_path();
            let store = ProjectStore::new(projects_path);

            let project = store.get(slug)?.ok_or_else(|| {
                anyhow!(
                    "Project '{}' not found. Add it with: crowdcontrol project add {} <url>",
                    slug,
                    slug
                )
            })?;

            // Use explicit branch arg, or project's default branch, or None
            let effective_branch = args.branch.or(project.default_branch);

            (
                identifier.to_filesystem_name(),
                identifier.to_display_name(),
                project.url,
                Some(slug.clone()),
                effective_branch,
            )
        } else {
            // Plain name format - requires repository URL
            let repository = args.repository.ok_or_else(|| {
                anyhow!(
                    "Repository URL required when not using project:label format.\n\
                     Usage: crowdcontrol new <name> <repository>\n\
                     Or register a project: crowdcontrol project add <slug> <url>"
                )
            })?;

            (
                args.name.clone(),
                args.name.clone(),
                repository,
                None,
                args.branch.clone(),
            )
        };

    // Validate agent name
    validate_agent_name(&filesystem_name)?;

    // Construct workspace path using nested structure (project/label or standalone)
    let workspace_path = identifier.to_path(&config.workspaces_dir);
    if workspace_path.exists() {
        return Err(anyhow!("Agent '{}' already exists", display_name));
    }

    print_info(&format!("Creating new agent: {}", display_name));

    // Create workspace directory
    fs::create_dir_all(&workspace_path)
        .with_context(|| format!("Failed to create workspace directory: {:?}", workspace_path))?;

    // Clone repository directly to workspace root
    let pb = create_progress_bar("Cloning repository...");

    // Wrap clone operation in a closure that handles cleanup on failure
    let clone_result = clone_repository(&repository_url, &workspace_path, branch.as_deref());

    pb.finish_and_clear();

    // If clone failed, cleanup workspace directory and return the error
    if let Err(e) = clone_result {
        // Cleanup the workspace directory we created
        if let Err(cleanup_err) = fs::remove_dir_all(&workspace_path) {
            eprintln!(
                "Warning: Failed to cleanup workspace directory after clone failure: {}",
                cleanup_err
            );
        }
        return Err(e);
    }

    print_success("Repository cloned successfully");

    // Verify repository setup if not skipped
    if !args.skip_verification {
        let has_crowdcontrol = verify_repository_setup(&workspace_path)?;
        if !has_crowdcontrol {
            print_warning("Repository does not contain .crowdcontrol/ directory");
            print_info(
                "The container will start but repository-specific setup scripts will not run",
            );
        }
    }

    // Helper to cleanup workspace on failure
    let cleanup_workspace = |workspace: &std::path::Path| {
        if let Err(cleanup_err) = fs::remove_dir_all(workspace) {
            eprintln!(
                "Warning: Failed to cleanup workspace directory after failure: {}",
                cleanup_err
            );
        }
    };

    // Create Docker client
    let docker = match DockerClient::new(config.clone()) {
        Ok(d) => d,
        Err(e) => {
            cleanup_workspace(&workspace_path);
            return Err(e);
        }
    };

    // Check if container already exists
    let container_exists = match docker
        .container_exists(&format!("crowdcontrol-{}", filesystem_name))
        .await
    {
        Ok(exists) => exists,
        Err(e) => {
            cleanup_workspace(&workspace_path);
            return Err(e);
        }
    };

    if container_exists {
        print_warning(&format!(
            "Container crowdcontrol-{} already exists",
            filesystem_name
        ));
    } else {
        // Pull image if needed
        if let Err(e) = docker.pull_image().await {
            cleanup_workspace(&workspace_path);
            return Err(e);
        }

        // Create container with defaults from config if not specified
        let pb = create_progress_bar("Creating container...");
        let memory = args.memory.or(config.default_memory.clone());
        let cpus = args.cpus.or(config.default_cpus.clone());
        let container_id = match docker
            .create_container(&filesystem_name, &workspace_path, memory, cpus)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                pb.finish_and_clear();
                cleanup_workspace(&workspace_path);
                return Err(e);
            }
        };
        pb.finish_and_clear();
        print_success("Container created successfully");

        // Save agent metadata
        let agent = Agent {
            name: filesystem_name.clone(),
            status: AgentStatus::Created,
            container_id: Some(container_id),
            repository: repository_url,
            branch,
            created_at: Utc::now(),
            workspace_path: workspace_path.clone(),
            project_slug,
        };

        if let Err(e) = save_agent_metadata(&config, &agent) {
            cleanup_workspace(&workspace_path);
            return Err(e);
        }
    }

    print_success(&format!("Agent '{}' setup complete!", display_name));
    print_info(&format!(
        "Start the agent with: crowdcontrol start {}",
        display_name
    ));

    Ok(())
}
