use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::fs;

use crate::commands::NewArgs;
use crate::utils::*;
use crowdcontrol_core::{
    clone_repository, save_agent_metadata, validate_agent_name, verify_repository_setup, Agent,
    AgentStatus, Config, DockerClient,
};

pub async fn execute(config: Config, args: NewArgs) -> Result<()> {
    // Validate agent name
    validate_agent_name(&args.name)?;

    // Check if agent already exists
    let workspace_path = config.agent_workspace_path(&args.name);
    if workspace_path.exists() {
        return Err(anyhow!("Agent '{}' already exists", args.name));
    }

    print_info(&format!("Creating new agent: {}", args.name));

    // Create workspace directory
    fs::create_dir_all(&workspace_path)
        .with_context(|| format!("Failed to create workspace directory: {:?}", workspace_path))?;

    // Clone repository
    let repo_target = workspace_path.join(&args.name);
    let pb = create_progress_bar("Cloning repository...");

    // Wrap clone operation in a closure that handles cleanup on failure
    let clone_result =
        (|| clone_repository(&args.repository, &repo_target, args.branch.as_deref()))();

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

    // Create Docker client
    let docker = DockerClient::new(config.clone())?;

    // Check if container already exists
    if docker
        .container_exists(&format!("crowdcontrol-{}", args.name))
        .await?
    {
        print_warning(&format!(
            "Container crowdcontrol-{} already exists",
            args.name
        ));
    } else {
        // Pull image if needed
        docker.pull_image().await?;

        // Create container with defaults from config if not specified
        let pb = create_progress_bar("Creating container...");
        let memory = args.memory.or(config.default_memory.clone());
        let cpus = args.cpus.or(config.default_cpus.clone());
        let container_id = docker
            .create_container(&args.name, &workspace_path, memory, cpus)
            .await?;
        pb.finish_and_clear();
        print_success("Container created successfully");

        // Save agent metadata
        let agent = Agent {
            name: args.name.clone(),
            status: AgentStatus::Created,
            container_id: Some(container_id),
            repository: args.repository.clone(),
            branch: args.branch.clone(),
            created_at: Utc::now(),
            workspace_path: workspace_path.clone(),
        };

        save_agent_metadata(&config, &agent)?;
    }

    print_success(&format!("Agent '{}' setup complete!", args.name));
    print_info(&format!(
        "Start the agent with: crowdcontrol start {}",
        args.name
    ));

    Ok(())
}
