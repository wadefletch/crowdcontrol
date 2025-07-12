use anyhow::{anyhow, Context, Result};
use std::process::Command;

use crate::commands::RefreshArgs;
use crate::utils::*;
use crowdcontrol_core::{Config, DockerClient};

pub async fn execute(config: Config, args: RefreshArgs) -> Result<()> {
    // Check if agent exists
    let workspace_path = config.agent_workspace_path(&args.name);
    if !workspace_path.exists() {
        return Err(anyhow!("Agent '{}' does not exist", args.name));
    }

    print_info(&format!(
        "Refreshing Claude Code authentication for agent: {}",
        args.name
    ));

    // Create Docker client
    let docker = DockerClient::new(config.clone())?;

    // Check if container exists and is running
    let container_name = format!("crowdcontrol-{}", args.name);
    if !docker.container_exists(&container_name).await? {
        return Err(anyhow!(
            "Container for agent '{}' does not exist",
            args.name
        ));
    }

    // Get container status
    let status = docker.get_container_status(&args.name).await?;
    if !matches!(status, crowdcontrol_core::AgentStatus::Running) {
        return Err(anyhow!(
            "Agent '{}' must be running to refresh configs. Start it first with: crowdcontrol start {}",
            args.name, args.name
        ));
    }

    // Run the refresh script, optionally with keychain credentials
    if args.extract_keychain {
        if cfg!(target_os = "macos") {
            let credentials = extract_keychain_credentials()?;
            let cmd = vec!["/usr/local/bin/refresh-claude-auth.sh", &credentials];
            docker
                .exec_in_container(&container_name, cmd, false)
                .await
                .context("Failed to refresh Claude Code authentication with keychain credentials")?;
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

#[cfg(target_os = "macos")]
fn extract_keychain_credentials() -> Result<String> {
    print_info("Extracting Claude Code credentials from macOS keychain...");
    
    // Try to extract credentials from keychain
    let output = Command::new("security")
        .args(&[
            "find-generic-password",
            "-s", "Claude Code-credentials",
            "-a", &whoami::username(),
            "-w"
        ])
        .output()
        .context("Failed to execute security command")?;

    if !output.status.success() {
        return Err(anyhow!(
            "No Claude Code credentials found in keychain. Make sure you're logged in to Claude Code."
        ));
    }

    let credentials = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in keychain credentials")?
        .trim()
        .to_string();

    if credentials.is_empty() {
        return Err(anyhow!("Empty credentials returned from keychain"));
    }

    print_success("Keychain credentials extracted successfully");
    
    Ok(credentials)
}

#[cfg(not(target_os = "macos"))]
fn extract_keychain_credentials() -> Result<String> {
    Err(anyhow!("Keychain extraction is only supported on macOS"))
}
