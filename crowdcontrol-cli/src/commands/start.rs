use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::time::sleep;

use crate::commands::StartArgs;
use crowdcontrol_core::Config;
use crowdcontrol_core::{AgentStatus, DockerClient};
use crate::utils::*;
use crowdcontrol_core::{load_agent_metadata, save_agent_metadata};
pub async fn execute(config: Config, args: StartArgs) -> Result<()> {
    // Load agent metadata
    let mut agent = load_agent_metadata(&config, &args.name)?;
    
    // Create Docker client
    let docker = DockerClient::new(config.clone())?;
    
    // Check current status
    let status = docker.get_container_status(&args.name).await?;
    
    match status {
        AgentStatus::Running => {
            print_info(&format!("Agent '{}' is already running", args.name));
            return Ok(());
        }
        AgentStatus::Error => {
            return Err(anyhow!("Agent '{}' is in error state. Please remove and recreate it.", args.name));
        }
        _ => {}
    }
    
    // Get container ID
    let container_id = agent.container_id.as_ref()
        .ok_or_else(|| anyhow!("No container ID found for agent '{}'", args.name))?;
    
    // Start container
    let pb = create_progress_bar(&format!("Starting agent '{}'...", args.name));
    docker.start_container(container_id).await?;
    pb.finish_and_clear();
    
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
            let status = docker.get_container_status(&args.name).await?;
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
    
    // Update agent status
    agent.status = AgentStatus::Running;
    save_agent_metadata(&config, &agent)?;
    
    print_info(&format!("Connect to the agent with: crowdcontrol connect {}", args.name));
    
    Ok(())
}