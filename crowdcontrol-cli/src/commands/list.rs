use anyhow::Result;
use colored::*;
use serde::Serialize;

use crate::commands::{AgentStatusFilter, ListArgs, OutputFormat};
use crate::utils::*;
use crowdcontrol_core::Config;
use crowdcontrol_core::{format_duration, list_all_agents, load_agent_metadata};
use crowdcontrol_core::{AgentStatus, DockerClient};
#[derive(Serialize)]
struct AgentInfo {
    name: String,
    status: String,
    repository: String,
    branch: Option<String>,
    created: String,
}

pub async fn execute(config: Config, args: ListArgs) -> Result<()> {
    let docker = DockerClient::new(config.clone())?;
    let agents = list_all_agents(&config)?;

    if agents.is_empty() {
        match args.format {
            OutputFormat::Json => println!("[]"),
            _ => print_info("No agents found"),
        }
        return Ok(());
    }

    let mut agent_infos = Vec::new();

    for agent_name in agents {
        // Load agent metadata
        let agent = match load_agent_metadata(&config, &agent_name) {
            Ok(a) => a,
            Err(_) => continue,
        };

        // Get current live status (this validates container_id and gets status from Docker)
        let status = agent
            .compute_live_status(&docker)
            .await
            .unwrap_or(AgentStatus::Error);

        // Apply status filter if provided
        if let Some(filter) = &args.status {
            let matches = match filter {
                AgentStatusFilter::Running => status == AgentStatus::Running,
                AgentStatusFilter::Stopped => status == AgentStatus::Stopped,
                AgentStatusFilter::Created => status == AgentStatus::Created,
                AgentStatusFilter::Error => status == AgentStatus::Error,
            };

            if !matches {
                continue;
            }
        }

        // Skip stopped agents unless --all is specified
        if !args.all && status == AgentStatus::Stopped {
            continue;
        }

        agent_infos.push(AgentInfo {
            name: agent.name.clone(),
            status: format!("{:?}", status),
            repository: agent.repository.clone(),
            branch: agent.branch.clone(),
            created: format_duration(agent.created_at),
        });
    }

    if agent_infos.is_empty() {
        match args.format {
            OutputFormat::Json => println!("[]"),
            _ => {
                if args.all {
                    print_info("No agents found");
                } else {
                    print_info("No running agents found (use --all to show all agents)");
                }
            }
        }
        return Ok(());
    }

    match args.format {
        OutputFormat::Table => print_table(&agent_infos),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&agent_infos)?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&agent_infos)?;
            print!("{}", yaml);
        }
    }

    Ok(())
}

fn print_table(agents: &[AgentInfo]) {
    // Calculate column widths
    let name_width = agents
        .iter()
        .map(|a| a.name.len())
        .max()
        .unwrap_or(4)
        .max(4);

    let status_width = 10;
    let created_width = 10;
    let repo_width = 30;

    // Print header
    println!(
        "{:<name_width$} {:<status_width$} {:<created_width$} {:<repo_width$} {}",
        "NAME".bold(),
        "STATUS".bold(),
        "CREATED".bold(),
        "REPOSITORY".bold(),
        "BRANCH".bold(),
        name_width = name_width,
        status_width = status_width,
        created_width = created_width,
        repo_width = repo_width,
    );

    // Print separator
    println!(
        "{} {} {} {} {}",
        "-".repeat(name_width),
        "-".repeat(status_width),
        "-".repeat(created_width),
        "-".repeat(repo_width),
        "-".repeat(20),
    );

    // Print agents
    for agent in agents {
        let status_colored = match agent.status.as_str() {
            "Running" => agent.status.green(),
            "Stopped" => agent.status.yellow(),
            "Created" => agent.status.white(),
            "Error" => agent.status.red(),
            _ => agent.status.normal(),
        };

        let repo_short = if agent.repository.len() > repo_width {
            format!(
                "...{}",
                &agent.repository[agent.repository.len() - repo_width + 3..]
            )
        } else {
            agent.repository.clone()
        };

        println!(
            "{:<name_width$} {:<status_width$} {:<created_width$} {:<repo_width$} {}",
            agent.name,
            status_colored,
            agent.created,
            repo_short,
            agent.branch.as_deref().unwrap_or("-"),
            name_width = name_width,
            status_width = status_width,
            created_width = created_width,
            repo_width = repo_width,
        );
    }
}
