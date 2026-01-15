use anyhow::{anyhow, Context, Result};

use crate::commands::RefreshArgs;
use crate::utils::*;
use crowdcontrol_core::{load_agent_metadata, Config, DockerClient};

pub async fn execute(config: Config, args: RefreshArgs) -> Result<()> {
    // Load agent metadata
    let agent = load_agent_metadata(&config, &args.name)?;

    print_info(&format!(
        "Refreshing Claude Code authentication for agent: {}",
        args.name
    ));

    // Create Docker client
    let docker = DockerClient::new(config.clone())?;

    // Get container status (validates container_id and gets live status)
    let status = agent.compute_live_status(&docker).await?;
    if !matches!(status, crowdcontrol_core::AgentStatus::Running) {
        return Err(anyhow!(
            "Agent '{}' must be running to refresh configs. Start it first with: crowdcontrol start {}",
            args.name, args.name
        ));
    }

    // Get container name for exec operations (use filesystem name, not display name)
    let container_name = format!("crowdcontrol-{}", agent.name);

    // Run the refresh script, optionally with keychain credentials
    if args.extract_keychain {
        if cfg!(target_os = "macos") {
            print_info("Extracting Claude Code credentials from macOS keychain...");
            let credentials = extract_keychain_credentials()
                .context("Make sure you're logged in to Claude Code")?;
            print_success("Keychain credentials extracted successfully");

            let cmd = vec!["/usr/local/bin/refresh-claude-auth.sh", &credentials];
            docker
                .exec_in_container(&container_name, cmd, false)
                .await
                .context(
                    "Failed to refresh Claude Code authentication with keychain credentials",
                )?;
        } else {
            print_warning("--extract-keychain flag is only supported on macOS");
            return Ok(());
        }
    } else {
        // Run the standard refresh script in the container
        let cmd = vec!["/usr/local/bin/refresh-claude-auth.sh"];
        docker
            .exec_in_container(&container_name, cmd, false)
            .await
            .context("Failed to refresh Claude Code authentication")?;
    }

    print_success(&format!(
        "Claude Code authentication refreshed successfully for agent '{}'",
        args.name
    ));
    print_info("You can now use Claude Code with the updated authentication");

    Ok(())
}
