// NOTE: These tests verify that agent status is always computed live from Docker
// and that container ID validation works correctly.

use anyhow::Result;
use crowdcontrol_core::{Agent, AgentStatus, Config, DockerClient};
use std::fs;
use tempfile::TempDir;
use tokio;

fn create_test_config() -> (Config, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };
    (config, temp_dir)
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_agent_status_computed_live_from_docker() -> Result<()> {
    let (config, _temp_dir) = create_test_config();
    let docker = DockerClient::new(config.clone())?;
    
    // Create agent metadata without container
    let agent_name = "test-live-status";
    let workspace_path = config.agent_workspace_path(agent_name);
    fs::create_dir_all(&workspace_path)?;
    
    let agent = Agent {
        name: agent_name.to_string(),
        status: AgentStatus::Created, // This should be ignored
        container_id: None,
        repository: "https://github.com/test/repo.git".to_string(),
        branch: Some("main".to_string()),
        created_at: chrono::Utc::now(),
        workspace_path: workspace_path.clone(),
    };
    
    crowdcontrol_core::agent::save_agent_metadata(&config, &agent)?;
    
    // Test 1: Agent with no container should have Created status
    let live_status = agent.compute_live_status(&docker).await?;
    assert_eq!(live_status, AgentStatus::Created);
    
    // Test 2: Create and start container, status should be Running
    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;
    
    // Update agent with container ID
    let mut agent_with_container = agent.clone();
    agent_with_container.container_id = Some(container_id.clone());
    
    // Wait for container to be running
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    let live_status = agent_with_container.compute_live_status(&docker).await?;
    assert_eq!(live_status, AgentStatus::Running);
    
    // Test 3: Stop container, status should be Stopped
    docker.stop_container(&container_id, false).await?;
    
    let live_status = agent_with_container.compute_live_status(&docker).await?;
    assert_eq!(live_status, AgentStatus::Stopped);
    
    // Cleanup
    docker.remove_container(&container_id).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"] 
async fn test_stale_container_id_detection() -> Result<()> {
    let (config, _temp_dir) = create_test_config();
    let docker = DockerClient::new(config.clone())?;
    
    let agent_name = "test-stale-container";
    let workspace_path = config.agent_workspace_path(agent_name);
    fs::create_dir_all(&workspace_path)?;
    
    // Create agent with fake container ID
    let agent = Agent {
        name: agent_name.to_string(),
        status: AgentStatus::Running, // This should be ignored
        container_id: Some("fake-container-id-123".to_string()),
        repository: "https://github.com/test/repo.git".to_string(),
        branch: Some("main".to_string()),
        created_at: chrono::Utc::now(),
        workspace_path: workspace_path.clone(),
    };
    
    // Test: Agent with stale container ID should detect and return Created status
    let live_status = agent.compute_live_status(&docker).await?;
    assert_eq!(live_status, AgentStatus::Created);
    
    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_id_validation() -> Result<()> {
    let (config, _temp_dir) = create_test_config();
    let docker = DockerClient::new(config.clone())?;
    
    let agent_name = "test-validate-container";
    let workspace_path = config.agent_workspace_path(agent_name);
    fs::create_dir_all(&workspace_path)?;
    
    // Test 1: Validate fake container ID - should be false
    let is_valid = docker.validate_container_id(agent_name, "fake-id").await?;
    assert!(!is_valid);
    
    // Test 2: Create real container and validate - should be true  
    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    
    let is_valid = docker.validate_container_id(agent_name, &container_id).await?;
    assert!(is_valid);
    
    // Test 3: Validate with wrong agent name - should be false
    let is_valid = docker.validate_container_id("wrong-agent", &container_id).await?;
    assert!(!is_valid);
    
    // Cleanup
    docker.remove_container(&container_id).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_agent_auto_repair_stale_container_id() -> Result<()> {
    let (config, _temp_dir) = create_test_config();
    
    let agent_name = "test-auto-repair";
    let workspace_path = config.agent_workspace_path(agent_name);
    fs::create_dir_all(&workspace_path)?;
    
    // Create agent with stale container ID
    let agent = Agent {
        name: agent_name.to_string(),
        status: AgentStatus::Running, // This should be ignored
        container_id: Some("stale-container-id".to_string()),
        repository: "https://github.com/test/repo.git".to_string(),
        branch: Some("main".to_string()),
        created_at: chrono::Utc::now(),
        workspace_path: workspace_path.clone(),
    };
    
    crowdcontrol_core::agent::save_agent_metadata(&config, &agent)?;
    
    // Test: Auto-repair should clear stale container ID
    crowdcontrol_core::agent::auto_repair_stale_container_id(&config, agent_name).await?;
    
    // Verify container ID was cleared
    let repaired_agent = crowdcontrol_core::agent::load_agent_metadata(&config, agent_name)?;
    assert_eq!(repaired_agent.container_id, None);
    
    Ok(())
}

#[tokio::test]
async fn test_agent_always_loads_with_created_status() -> Result<()> {
    let (config, _temp_dir) = create_test_config();
    
    let agent_name = "test-load-status";
    let workspace_path = config.agent_workspace_path(agent_name);
    fs::create_dir_all(&workspace_path)?;
    
    // Create agent with any status (should be ignored)
    let agent = Agent {
        name: agent_name.to_string(),
        status: AgentStatus::Running, // This should be ignored when saved/loaded
        container_id: Some("some-container-id".to_string()),
        repository: "https://github.com/test/repo.git".to_string(),
        branch: Some("main".to_string()),
        created_at: chrono::Utc::now(),
        workspace_path: workspace_path.clone(),
    };
    
    crowdcontrol_core::agent::save_agent_metadata(&config, &agent)?;
    
    // Test: Loaded agent should always have Created status
    let loaded_agent = crowdcontrol_core::agent::load_agent_metadata(&config, agent_name)?;
    assert_eq!(loaded_agent.status, AgentStatus::Created);
    
    // Container ID should be preserved though
    assert_eq!(loaded_agent.container_id, Some("some-container-id".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_status_field_completely_ignored() -> Result<()> {
    let (config, _temp_dir) = create_test_config();
    
    let agent_name = "test-status-ignored";
    let workspace_path = config.agent_workspace_path(agent_name);
    fs::create_dir_all(&workspace_path)?;
    
    // Create agent with Running status
    let mut agent = Agent {
        name: agent_name.to_string(),
        status: AgentStatus::Running, // This should be completely ignored
        container_id: None,
        repository: "https://github.com/test/repo.git".to_string(),
        branch: Some("main".to_string()),
        created_at: chrono::Utc::now(),
        workspace_path: workspace_path.clone(),
    };
    
    crowdcontrol_core::agent::save_agent_metadata(&config, &agent)?;
    
    // Change status in memory
    agent.status = AgentStatus::Stopped;
    crowdcontrol_core::agent::save_agent_metadata(&config, &agent)?;
    
    // Change status again
    agent.status = AgentStatus::Error;
    crowdcontrol_core::agent::save_agent_metadata(&config, &agent)?;
    
    // Test: No matter what status we set and save, loaded agent is always Created
    let loaded_agent = crowdcontrol_core::agent::load_agent_metadata(&config, agent_name)?;
    assert_eq!(loaded_agent.status, AgentStatus::Created);
    
    Ok(())
}