use anyhow::Result;
use clap::Args;
use colored::Colorize;
use crowdcontrol_core::Config;

#[derive(Args, Debug)]
pub struct ConfigCommand {
    /// Show current configuration
    #[arg(long)]
    show: bool,
}

pub async fn execute(config: Config, cmd: ConfigCommand) -> Result<()> {
    if cmd.show {
        show_config(&config).await?;
    } else {
        // If no subcommand provided, show help or default behavior
        println!("Use --show to display current configuration");
    }
    
    Ok(())
}

async fn show_config(config: &Config) -> Result<()> {
    println!("{}", "Current Configuration:".bold());
    println!();
    
    // Core settings
    println!("{}", "Core Settings:".bold());
    println!("  workspaces_dir: {}", config.workspaces_dir.display());
    println!("  image: {}", config.image);
    
    if let Some(memory) = &config.default_memory {
        println!("  default_memory: {}", memory);
    }
    
    if let Some(cpus) = &config.default_cpus {
        println!("  default_cpus: {}", cpus);
    }
    
    println!();
    
    // GitHub configuration
    println!("{}", "GitHub Settings:".bold());
    if let Some(github_config) = &config.github {
        if github_config.installation_token.is_some() {
            println!("  installation_token: [configured]");
        }
        
        if let Some(app_id) = &github_config.app_id {
            println!("  app_id: {}", app_id);
        }
        
        if let Some(installation_id) = &github_config.installation_id {
            println!("  installation_id: {}", installation_id);
        }
        
        if github_config.private_key_path.is_some() {
            println!("  private_key_path: [configured]");
        }
        
        if let Some(base_url) = &github_config.base_url {
            println!("  base_url: {}", base_url);
        }
    } else {
        println!("  github: ~");
    }
    
    Ok(())
}