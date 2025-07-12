use anyhow::Result;
use dialoguer::Confirm;
use std::fs;

use crate::commands::RemoveArgs;
use crowdcontrol_core::Config;
use crowdcontrol_core::DockerClient;
use crate::utils::*;
use crowdcontrol_core::load_agent_metadata;
pub async fn execute(config: Config, args: RemoveArgs) -> Result<()> {
    // Load agent metadata
    let agent = load_agent_metadata(&config, &args.name)?;
    
    // Confirm removal if not forced
    if !args.force {
        let prompt = if args.keep_workspace {
            format!("Are you sure you want to remove the container for agent '{}'?", args.name)
        } else {
            format!("Are you sure you want to remove agent '{}' and its workspace?", args.name)
        };
        
        let confirm = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        
        if !confirm {
            print_info("Removal cancelled");
            return Ok(());
        }
    }
    
    // Create Docker client
    let docker = DockerClient::new(config.clone())?;
    
    // Remove container if it exists
    if let Some(container_id) = &agent.container_id {
        let pb = create_progress_bar("Removing container...");
        match docker.remove_container(container_id).await {
            Ok(_) => {
                pb.finish_and_clear();
                print_success("Container removed successfully");
            }
            Err(e) => {
                pb.finish_and_clear();
                print_warning(&format!("Failed to remove container: {}", e));
            }
        }
    }
    
    // Remove workspace directory if requested
    if !args.keep_workspace {
        let pb = create_progress_bar("Removing workspace directory...");
        fs::remove_dir_all(&agent.workspace_path)?;
        pb.finish_and_clear();
        print_success("Workspace directory removed successfully");
    } else {
        // Remove metadata file only
        let metadata_path = agent.workspace_path.join(".crowdcontrol-metadata.json");
        if metadata_path.exists() {
            fs::remove_file(metadata_path)?;
        }
        print_info("Workspace directory kept");
    }
    
    print_success(&format!("Agent '{}' removed successfully", args.name));
    
    Ok(())
}