use anyhow::{anyhow, Result};

use crate::commands::StopArgs;
use crate::utils::*;
use crowdcontrol_core::Config;
use crowdcontrol_core::{list_all_agents, load_agent_metadata, save_agent_metadata};
use crowdcontrol_core::{AgentStatus, DockerClient};
pub async fn execute(config: Config, args: StopArgs) -> Result<()> {
    let docker = DockerClient::new(config.clone())?;

    if args.all {
        // Stop all running agents
        let agents = list_all_agents(&config)?;
        let mut stopped_count = 0;
        let mut error_count = 0;

        for agent_name in agents {
            match stop_agent(&docker, &config, &agent_name, args.force).await {
                Ok(true) => stopped_count += 1,
                Ok(false) => {} // Agent was not running
                Err(e) => {
                    print_error(&format!("Failed to stop {}: {}", agent_name, e));
                    error_count += 1;
                }
            }
        }

        if stopped_count > 0 {
            print_success(&format!("Stopped {} agent(s)", stopped_count));
        }

        if error_count > 0 {
            return Err(anyhow!("Failed to stop {} agent(s)", error_count));
        }

        if stopped_count == 0 {
            print_info("No running agents to stop");
        }
    } else {
        // Stop specific agent
        let name = args
            .name
            .ok_or_else(|| anyhow!("Agent name required when not using --all"))?;
        let stopped = stop_agent(&docker, &config, &name, args.force).await?;

        if !stopped {
            print_info(&format!("Agent '{}' is not running", name));
        }
    }

    Ok(())
}

async fn stop_agent(
    docker: &DockerClient,
    config: &Config,
    name: &str,
    force: bool,
) -> Result<bool> {
    // Load agent metadata
    let mut agent = load_agent_metadata(config, name)?;

    // Check current status (validates container_id and gets live status)
    let status = agent.compute_live_status(docker).await?;

    if status != AgentStatus::Running {
        return Ok(false);
    }

    // Get container ID
    let container_id = agent
        .container_id
        .as_ref()
        .ok_or_else(|| anyhow!("No container ID found for agent '{}'", name))?;

    // Stop container
    let pb = create_progress_bar(&format!("Stopping agent '{}'...", name));
    docker.stop_container(container_id, force).await?;
    pb.finish_and_clear();

    print_success(&format!("Agent '{}' stopped successfully", name));

    // Clear container ID since container is now stopped
    agent.container_id = None;
    save_agent_metadata(config, &agent)?;

    Ok(true)
}
