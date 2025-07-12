use anyhow::{anyhow, Result};

use crate::commands::LogsArgs;
use crowdcontrol_core::{Config, DockerClient, load_agent_metadata};
pub async fn execute(config: Config, args: LogsArgs) -> Result<()> {
    // Load agent metadata
    let agent = load_agent_metadata(&config, &args.name)?;
    
    // Get container ID
    let container_id = agent.container_id
        .ok_or_else(|| anyhow!("No container ID found for agent '{}'", args.name))?;
    
    // Create Docker client
    let docker = DockerClient::new(config)?;
    
    // Get logs
    docker.get_container_logs(
        &container_id,
        args.follow,
        Some(args.tail.to_string()),
        args.timestamps,
    ).await?;
    
    Ok(())
}