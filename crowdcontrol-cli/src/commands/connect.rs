use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

use crate::commands::ConnectArgs;
use crate::utils::*;
use crowdcontrol_core::load_agent_metadata;
use crowdcontrol_core::parse_agent_identifier;
use crowdcontrol_core::Config;
use crowdcontrol_core::{AgentStatus, DockerClient};

pub async fn execute(config: Config, args: ConnectArgs) -> Result<()> {
    // Parse the identifier to handle project:label format
    let identifier = parse_agent_identifier(&args.name)?;
    let filesystem_name = identifier.to_filesystem_name();
    let display_name = identifier.to_display_name();

    // Load agent metadata using filesystem name
    let agent = load_agent_metadata(&config, &filesystem_name)?;

    // Create Docker client
    let docker = DockerClient::new(config.clone())?;

    // Check if container is running (validates container_id and gets live status)
    let status = agent.compute_live_status(&docker).await?;
    if status != AgentStatus::Running {
        return Err(anyhow!(
            "Agent '{}' is not running. Start it with: crowdcontrol start {}",
            display_name,
            display_name
        ));
    }

    // Get container name
    let container_name = format!("crowdcontrol-{}", filesystem_name);

    // Prepare command
    let default_command = vec!["claude", "--dangerously-skip-permissions"];
    let command_parts: Vec<&str> = if let Some(cmd) = &args.command {
        cmd.split_whitespace().collect()
    } else {
        default_command
    };

    if args.detach {
        // Run in background
        docker
            .exec_in_container_as_user(&container_name, command_parts, false, Some("developer"))
            .await?;
        print_success(&format!(
            "Command started in background in agent '{}'",
            display_name
        ));
    } else {
        // Interactive connection
        print_info(&format!("Connecting to agent '{}'...", display_name));

        // Use docker exec directly for better TTY handling
        let mut cmd = Command::new("docker")
            .arg("exec")
            .arg("-it")
            .arg("-u")
            .arg("developer")
            .arg(&container_name)
            .args(&command_parts)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow!("Failed to connect to agent: {}", e))?;

        let status = cmd.wait().map_err(|e| anyhow!("Connection error: {}", e))?;

        if !status.success() {
            return Err(anyhow!("Connection to agent terminated with error"));
        }
    }

    Ok(())
}
