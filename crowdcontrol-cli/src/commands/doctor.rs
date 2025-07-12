use anyhow::Result;
use clap::Args;
use colored::Colorize;
use crowdcontrol_core::{Config, StateValidator, StateInconsistency};

#[derive(Args, Debug)]
pub struct DoctorCommand {
    /// Automatically repair inconsistencies if possible
    #[arg(long)]
    repair: bool,
    
    /// Show detailed information about each check
    #[arg(long)]
    verbose: bool,
}

pub async fn execute(config: Config, cmd: DoctorCommand) -> Result<()> {
    println!("{}", "Running system diagnostics...".bold());
    
    // Create state validator
    let validator = StateValidator::new(config)?;
    
    // Check for inconsistencies
    let inconsistencies = validator.validate_all().await?;
    
    if inconsistencies.is_empty() {
        println!("{}", "✓ All checks passed! System state is consistent.".green().bold());
        return Ok(());
    }
    
    // Display issues found
    println!("\n{}", format!("Found {} issue(s):", inconsistencies.len()).yellow().bold());
    
    for (i, issue) in inconsistencies.iter().enumerate() {
        println!("\n{}. {}", i + 1, format_issue(issue, cmd.verbose));
    }
    
    // Repair if requested
    if cmd.repair {
        println!("\n{}", "Attempting to repair issues...".bold());
        validator.repair_inconsistencies(inconsistencies).await?;
        
        // Re-validate to show current state
        println!("\n{}", "Re-validating system state...".bold());
        let remaining_issues = validator.validate_all().await?;
        
        if remaining_issues.is_empty() {
            println!("{}", "✓ All issues resolved!".green().bold());
        } else {
            println!("{}", format!("⚠ {} issue(s) require manual intervention", remaining_issues.len()).yellow());
            
            for issue in remaining_issues {
                println!("  • {}", format_issue(&issue, false));
            }
        }
    } else {
        println!("\n{}", "Run with --repair to attempt automatic fixes".dimmed());
    }
    
    Ok(())
}

fn format_issue(issue: &StateInconsistency, verbose: bool) -> String {
    use StateInconsistency::*;
    
    match issue {
        MissingWorkspace { agent_name } => {
            let msg = format!("Missing workspace directory for agent '{}'", agent_name.red());
            if verbose {
                format!("{}\n    The agent's metadata exists but its workspace directory is missing.\n    This may happen if the directory was manually deleted.", msg)
            } else {
                msg
            }
        }
        
        OrphanedContainer { container_name } => {
            let msg = format!("Orphaned container 'crowdcontrol-{}' has no metadata", container_name.red());
            if verbose {
                format!("{}\n    A Docker container exists but there's no corresponding agent metadata.\n    This may happen if metadata was manually deleted or corrupted.", msg)
            } else {
                msg
            }
        }
        
        MissingContainer { agent_name } => {
            let msg = format!("Agent '{}' marked as running but container doesn't exist", agent_name.yellow());
            if verbose {
                format!("{}\n    The metadata indicates the agent is running, but no Docker container was found.\n    This can happen if Docker was restarted or the container was manually removed.", msg)
            } else {
                msg
            }
        }
        
        IncorrectStatus { agent_name, expected, actual } => {
            let msg = format!(
                "Agent '{}' status mismatch: metadata says {:?} but container is {:?}", 
                agent_name.yellow(), expected, actual
            );
            if verbose {
                format!("{}\n    The agent's metadata status doesn't match the actual container state.\n    This is usually harmless and can be auto-repaired.", msg)
            } else {
                msg
            }
        }
        
        ContainerIdMismatch { agent_name, metadata_id, actual_id } => {
            let msg = format!(
                "Agent '{}' container ID mismatch", 
                agent_name.yellow()
            );
            if verbose {
                format!("{}\n    Metadata ID: {}\n    Actual ID: {}\n    This might indicate the container was recreated outside of crowdcontrol.", 
                    msg, metadata_id.dimmed(), actual_id.dimmed())
            } else {
                msg
            }
        }
        
        DuplicateContainers { agent_name, container_ids } => {
            let msg = format!(
                "Multiple containers found for agent '{}'", 
                agent_name.red()
            );
            if verbose {
                format!("{}\n    Container IDs: {}\n    This requires manual intervention to remove duplicates.", 
                    msg, container_ids.join(", ").dimmed())
            } else {
                msg
            }
        }
        
        CorruptedMetadata { agent_name, error } => {
            let msg = format!(
                "Corrupted metadata for agent '{}'", 
                agent_name.red()
            );
            if verbose {
                format!("{}\n    Error: {}\n    The metadata file may be corrupted or have invalid JSON.", 
                    msg, error.dimmed())
            } else {
                msg
            }
        }
    }
}