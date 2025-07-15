use anyhow::Result;
use clap::Args;
use colored::Colorize;
use crowdcontrol_core::{Config, DockerClient, load_agent_metadata, format_duration, AgentStatus};

#[derive(Args, Debug)]
pub struct StatusCommand {
    /// Name of the agent to check status for
    name: String,
}

pub async fn execute(config: Config, cmd: StatusCommand) -> Result<()> {
    let docker = DockerClient::new(config.clone())?;
    
    // Load agent metadata
    let agent = load_agent_metadata(&config, &cmd.name)?;
    
    println!("{}", format!("Agent Status: {}", cmd.name).bold());
    println!();
    
    // Basic info
    println!("Name: {}", agent.name);
    println!("Repository: {}", agent.repository);
    
    if let Some(branch) = &agent.branch {
        println!("Branch: {}", branch);
    }
    
    println!("Created: {}", agent.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("Age: {}", format_duration(agent.created_at));
    
    // Get current live status
    let status = agent
        .compute_live_status(&docker)
        .await
        .unwrap_or(AgentStatus::Error);
    
    let status_str = match status {
        AgentStatus::Running => "Running".green(),
        AgentStatus::Stopped => "Stopped".red(),
        AgentStatus::Created => "Created".yellow(),
        AgentStatus::Error => "Error".red(),
    };
    println!("Status: {}", status_str);
    
    // Workspace path
    println!("Workspace: {}", agent.workspace_path.display());
    
    // Container info if available
    if let Some(container_id) = &agent.container_id {
        println!("Container ID: {}", container_id);
        
        // Try to get additional container info from Docker
        if status == AgentStatus::Running {
            // Could add more container details here like resource usage, ports, etc.
        }
    }
    
    Ok(())
}