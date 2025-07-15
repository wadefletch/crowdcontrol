use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::time::sleep;

use crate::commands::{StartArgs, NewArgs};
use crate::utils::*;
use crate::commands::new;
use crowdcontrol_core::load_agent_metadata;
use crowdcontrol_core::Config;
use crowdcontrol_core::{AgentStatus, DockerClient};

pub async fn execute(config: Config, args: StartArgs) -> Result<()> {
    // If --repo is provided, try to create the agent first if it doesn't exist
    if let Some(repo_url) = &args.repo {
        match load_agent_metadata(&config, &args.name) {
            Err(_) => {
                // Agent doesn't exist, create it first
                print_info(&format!("Agent '{}' not found, creating from repository: {}", args.name, repo_url));
                let new_args = NewArgs {
                    name: args.name.clone(),
                    repository: repo_url.clone(),
                    branch: None,
                    skip_verification: true, // Auto-skip verification for convenience
                    memory: None,
                    cpus: None,
                };
                new::execute(config.clone(), new_args).await?;
            }
            Ok(_) => {
                // Agent exists, proceed with normal start
                print_info(&format!("Agent '{}' already exists, starting...", args.name));
            }
        }
    }

    // Load agent metadata (either existing or just created)
    let agent = load_agent_metadata(&config, &args.name)?;

    // Create Docker client
    let docker = DockerClient::new(config.clone())?;

    // Check current status (validates container_id and gets live status)
    let status = agent.compute_live_status(&docker).await?;

    match status {
        AgentStatus::Running => {
            print_info(&format!("Agent '{}' is already running", args.name));
            return Ok(());
        }
        AgentStatus::Error => {
            return Err(anyhow!(
                "Agent '{}' is in error state. Please remove and recreate it.",
                args.name
            ));
        }
        AgentStatus::Created => {
            // Status is Created - either no container or container ID is stale
            // If we have a container ID, try to start it first
            if let Some(container_id) = &agent.container_id {
                // Try to start the existing container
                let pb = create_progress_bar(&format!("Starting agent '{}'...", args.name));
                match docker.start_container(container_id).await {
                    Ok(_) => {
                        pb.finish_and_clear();
                    }
                    Err(_) => {
                        // Failed to start, container might be stale
                        pb.finish_and_clear();
                        
                        // Remove the stale container if it exists
                        if let Err(e) = docker.remove_container(container_id).await {
                            print_warning(&format!("Failed to remove stale container: {}", e));
                        }
                        
                        // Create a new container
                        let pb = create_progress_bar(&format!("Creating container for agent '{}'...", args.name));
                        let new_container_id = docker.create_container(&args.name, &agent.workspace_path, None, None).await?;
                        pb.finish_and_clear();
                        
                        // Update metadata with new container ID
                        use crowdcontrol_core::update_agent_metadata;
                        update_agent_metadata(&config, &args.name, |agent| {
                            agent.container_id = Some(new_container_id.clone());
                            Ok(())
                        })?;
                        
                        // Start the new container
                        let pb = create_progress_bar(&format!("Starting agent '{}'...", args.name));
                        docker.start_container(&new_container_id).await?;
                        pb.finish_and_clear();
                    }
                }
            } else {
                // No container ID, create a new container
                let pb = create_progress_bar(&format!("Creating container for agent '{}'...", args.name));
                let container_id = docker.create_container(&args.name, &agent.workspace_path, None, None).await?;
                pb.finish_and_clear();
                
                // Update metadata with new container ID
                use crowdcontrol_core::update_agent_metadata;
                update_agent_metadata(&config, &args.name, |agent| {
                    agent.container_id = Some(container_id.clone());
                    Ok(())
                })?;
                
                // Start the new container
                let pb = create_progress_bar(&format!("Starting agent '{}'...", args.name));
                docker.start_container(&container_id).await?;
                pb.finish_and_clear();
            }
        }
        AgentStatus::Stopped => {
            // Container exists but is stopped, start it
            let container_id = agent
                .container_id
                .as_ref()
                .ok_or_else(|| anyhow!("No container ID found for agent '{}'", args.name))?;

            let pb = create_progress_bar(&format!("Starting agent '{}'...", args.name));
            docker.start_container(container_id).await?;
            pb.finish_and_clear();
        }
    }

    print_success(&format!("Agent '{}' started successfully", args.name));

    // Wait for initialization if requested
    if args.wait {
        let pb = create_progress_bar("Waiting for agent initialization...");
        let timeout_duration = Duration::from_secs(args.timeout);
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout_duration {
                pb.finish_and_clear();
                print_warning("Timeout waiting for agent initialization");
                break;
            }

            // Check if container is still running
            let status = agent.compute_live_status(&docker).await?;
            if status != AgentStatus::Running {
                pb.finish_and_clear();
                return Err(anyhow!("Agent stopped unexpectedly during initialization"));
            }

            // TODO: Add actual readiness check (e.g., check if Docker daemon is ready inside container)
            // For now, just wait a fixed amount of time
            sleep(Duration::from_secs(2)).await;
            pb.finish_and_clear();
            print_success("Agent initialization complete");
            break;
        }
    }

    // Note: Agent status is now computed live from Docker, no need to save it

    print_info(&format!(
        "Connect to the agent with: crowdcontrol connect {}",
        args.name
    ));

    Ok(())
}
