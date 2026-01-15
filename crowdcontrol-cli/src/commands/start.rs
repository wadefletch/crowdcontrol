use anyhow::{anyhow, Context, Result};
use std::time::Duration;
use tokio::time::sleep;

use crate::commands::StartArgs;
use crate::utils::*;
use crowdcontrol_core::load_agent_metadata;
use crowdcontrol_core::Config;
use crowdcontrol_core::{AgentStatus, DockerClient};
pub async fn execute(config: Config, args: StartArgs) -> Result<()> {
    // Load agent metadata
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
        _ => {}
    }

    // Get container ID
    let container_id = agent
        .container_id
        .as_ref()
        .ok_or_else(|| anyhow!("No container ID found for agent '{}'", args.name))?;

    // Start container
    let pb = create_progress_bar(&format!("Starting agent '{}'...", args.name));
    docker.start_container(container_id).await?;
    pb.finish_and_clear();

    print_success(&format!("Agent '{}' started successfully", args.name));

    // Auto-inject keychain credentials on macOS
    if cfg!(target_os = "macos") {
        // Wait briefly for container to initialize
        sleep(Duration::from_secs(2)).await;

        if let Ok(credentials) = extract_keychain_credentials() {
            let container_name = format!("crowdcontrol-{}", agent.name);
            let cmd = vec!["/usr/local/bin/refresh-claude-auth.sh", &credentials];
            if docker
                .exec_in_container(&container_name, cmd, false)
                .await
                .context("Failed to inject keychain credentials")
                .is_ok()
            {
                print_success("Claude Code credentials injected from keychain");
            }
        }
    }

    // Wait for initialization if requested
    if args.wait {
        let pb = create_progress_bar("Waiting for agent initialization...");
        let timeout_duration = Duration::from_secs(args.timeout);
        let start_time = std::time::Instant::now();

        // TODO: Add actual readiness check (e.g., check if Docker daemon is ready inside container)
        // For now, just wait a fixed amount of time after verifying container is running
        sleep(Duration::from_secs(2)).await;

        if start_time.elapsed() > timeout_duration {
            pb.finish_and_clear();
            print_warning("Timeout waiting for agent initialization");
        } else {
            // Check if container is still running
            let status = agent.compute_live_status(&docker).await?;
            if status != AgentStatus::Running {
                pb.finish_and_clear();
                return Err(anyhow!("Agent stopped unexpectedly during initialization"));
            }

            pb.finish_and_clear();
            print_success("Agent initialization complete");
        }
    }

    // Note: Agent status is now computed live from Docker, no need to save it

    print_info(&format!(
        "Connect to the agent with: crowdcontrol connect {}",
        args.name
    ));

    Ok(())
}
