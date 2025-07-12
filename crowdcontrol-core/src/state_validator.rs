use anyhow::Result;
use crate::{AgentStatus, Config, DockerClient};
use crate::agent::{load_agent_metadata, list_all_agents};
use tracing::{debug, info, warn, error};
use std::collections::HashSet;

/// Represents various inconsistencies that can occur in the system
#[derive(Debug)]
pub enum StateInconsistency {
    /// Metadata exists but workspace directory is missing
    MissingWorkspace { agent_name: String },
    
    /// Container exists but metadata is missing
    OrphanedContainer { container_name: String },
    
    /// Metadata says running but container doesn't exist
    MissingContainer { agent_name: String },
    
    /// Metadata says stopped but container is running
    IncorrectStatus { agent_name: String, expected: AgentStatus, actual: AgentStatus },
    
    /// Container ID in metadata doesn't match actual container
    ContainerIdMismatch { agent_name: String, metadata_id: String, actual_id: String },
    
    /// Multiple containers for same agent
    DuplicateContainers { agent_name: String, container_ids: Vec<String> },
    
    /// Corrupted metadata file
    CorruptedMetadata { agent_name: String, error: String },
}

/// Validates the consistency of the entire system state
pub struct StateValidator {
    config: Config,
    docker_client: DockerClient,
}

impl StateValidator {
    pub fn new(config: Config) -> Result<Self> {
        let docker_client = DockerClient::new(config.clone())?;
        Ok(Self { config, docker_client })
    }
    
    /// Check for all types of inconsistencies
    pub async fn validate_all(&self) -> Result<Vec<StateInconsistency>> {
        info!("Starting state validation");
        let mut inconsistencies = Vec::new();
        
        // Get all agents from metadata
        let agent_names = list_all_agents(&self.config)?;
        debug!("Found {} agents to validate", agent_names.len());
        
        // Get all containers from Docker
        let containers = self.docker_client.list_all_containers().await?;
        let container_names: HashSet<String> = containers.iter()
            .filter_map(|c| c.names.as_ref()?.first())
            .filter_map(|name| {
                // Container names start with "/crowdcontrol-"
                name.strip_prefix("/crowdcontrol-").map(|s| s.to_string())
            })
            .collect();
        
        // Check each agent for inconsistencies
        for agent_name in &agent_names {
            match self.validate_agent(agent_name).await {
                Ok(mut agent_issues) => inconsistencies.append(&mut agent_issues),
                Err(e) => {
                    inconsistencies.push(StateInconsistency::CorruptedMetadata {
                        agent_name: agent_name.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }
        
        // Check for orphaned containers (containers without metadata)
        for container_name in container_names {
            if !agent_names.contains(&container_name) {
                inconsistencies.push(StateInconsistency::OrphanedContainer {
                    container_name,
                });
            }
        }
        
        Ok(inconsistencies)
    }
    
    /// Validate a single agent's state
    async fn validate_agent(&self, agent_name: &str) -> Result<Vec<StateInconsistency>> {
        let mut inconsistencies = Vec::new();
        
        // Load metadata
        let agent = load_agent_metadata(&self.config, agent_name)?;
        
        // Check workspace exists
        if !agent.workspace_path.exists() {
            inconsistencies.push(StateInconsistency::MissingWorkspace {
                agent_name: agent_name.to_string(),
            });
        }
        
        // Get container details from Docker for validation
        let container_info = self.docker_client
            .find_container_details(&format!("crowdcontrol-{}", agent_name))
            .await?;
        
        match (agent.status, &container_info) {
            (AgentStatus::Running, &None) => {
                // Metadata says running but no container exists
                inconsistencies.push(StateInconsistency::MissingContainer {
                    agent_name: agent_name.to_string(),
                });
            }
            (AgentStatus::Stopped, &Some(ref status)) if status.is_running => {
                // Metadata says stopped but container is running
                inconsistencies.push(StateInconsistency::IncorrectStatus {
                    agent_name: agent_name.to_string(),
                    expected: AgentStatus::Stopped,
                    actual: AgentStatus::Running,
                });
            }
            (AgentStatus::Running, &Some(ref status)) if !status.is_running => {
                // Metadata says running but container is stopped
                inconsistencies.push(StateInconsistency::IncorrectStatus {
                    agent_name: agent_name.to_string(),
                    expected: AgentStatus::Running,
                    actual: AgentStatus::Stopped,
                });
            }
            _ => {}
        }
        
        // Check container ID matches
        if let (Some(metadata_id), Some(container_info)) = (agent.container_id, container_info.as_ref()) {
            if metadata_id != container_info.id {
                inconsistencies.push(StateInconsistency::ContainerIdMismatch {
                    agent_name: agent_name.to_string(),
                    metadata_id,
                    actual_id: container_info.id.clone(),
                });
            }
        }
        
        // Check for duplicate containers
        let matching_containers = self.docker_client
            .find_containers_by_name(&format!("crowdcontrol-{}", agent_name))
            .await?;
        
        if matching_containers.len() > 1 {
            inconsistencies.push(StateInconsistency::DuplicateContainers {
                agent_name: agent_name.to_string(),
                container_ids: matching_containers.into_iter()
                    .map(|c| c.id.unwrap_or_default())
                    .collect(),
            });
        }
        
        Ok(inconsistencies)
    }
    
    /// Attempt to repair inconsistencies
    pub async fn repair_inconsistencies(&self, inconsistencies: Vec<StateInconsistency>) -> Result<()> {
        use crate::agent::update_agent_metadata;
        
        info!("Attempting to repair {} inconsistencies", inconsistencies.len());
        
        for inconsistency in inconsistencies {
            match inconsistency {
                StateInconsistency::MissingWorkspace { agent_name } => {
                    warn!("Workspace missing for agent '{}'. Consider removing the agent.", agent_name);
                }
                
                StateInconsistency::OrphanedContainer { container_name } => {
                    warn!("Found orphaned container '{}'. Consider removing it manually.", container_name);
                    // Could auto-remove: self.docker_client.remove_container(&format!("crowdcontrol-{}", container_name)).await?;
                }
                
                StateInconsistency::MissingContainer { agent_name } => {
                    // Update metadata to reflect container is stopped
                    debug!("Updating agent '{}' status to Stopped (container missing)", agent_name);
                    update_agent_metadata(&self.config, &agent_name, |agent| {
                        agent.status = AgentStatus::Stopped;
                        agent.container_id = None;
                        Ok(())
                    })?;
                    info!("Fixed: Updated agent '{}' status to Stopped", agent_name);
                }
                
                StateInconsistency::IncorrectStatus { agent_name, expected: _, actual } => {
                    // Update metadata to match actual container state
                    debug!("Updating agent '{}' status to {:?}", agent_name, actual);
                    update_agent_metadata(&self.config, &agent_name, |agent| {
                        agent.status = actual;
                        Ok(())
                    })?;
                    info!("Fixed: Updated agent '{}' status to match container state", agent_name);
                }
                
                StateInconsistency::ContainerIdMismatch { agent_name, metadata_id: _, actual_id } => {
                    // Update metadata with correct container ID
                    debug!("Updating agent '{}' container ID to {}", agent_name, actual_id);
                    update_agent_metadata(&self.config, &agent_name, |agent| {
                        agent.container_id = Some(actual_id.clone());
                        Ok(())
                    })?;
                    info!("Fixed: Updated agent '{}' container ID", agent_name);
                }
                
                StateInconsistency::DuplicateContainers { agent_name, container_ids } => {
                    error!("Multiple containers found for agent '{}': {:?}", agent_name, container_ids);
                    warn!("Manual intervention required to remove duplicate containers.");
                }
                
                StateInconsistency::CorruptedMetadata { agent_name, error } => {
                    error!("Corrupted metadata for agent '{}': {}", agent_name, error);
                    warn!("Consider removing and re-creating the agent.");
                }
            }
        }
        
        Ok(())
    }
}

/// Container validation details
#[derive(Debug)]
pub struct ContainerValidationInfo {
    pub id: String,
    pub is_running: bool,
}

/// Extension methods for DockerClient to support state validation
impl DockerClient {
    /// Find container details for state validation (returns None if not found)
    pub async fn find_container_details(&self, container_name: &str) -> Result<Option<ContainerValidationInfo>> {
        let containers = self.list_all_containers().await?;
        
        for container in containers {
            if let Some(names) = &container.names {
                if names.iter().any(|n| n == &format!("/{}", container_name)) {
                    return Ok(Some(ContainerValidationInfo {
                        id: container.id.unwrap_or_default(),
                        is_running: container.state == Some("running".to_string()),
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Find all containers matching a name pattern
    pub async fn find_containers_by_name(&self, name_pattern: &str) -> Result<Vec<bollard::models::ContainerSummary>> {
        let containers = self.list_all_containers().await?;
        
        Ok(containers.into_iter()
            .filter(|c| {
                if let Some(names) = &c.names {
                    names.iter().any(|n| n.contains(name_pattern))
                } else {
                    false
                }
            })
            .collect())
    }
    
}