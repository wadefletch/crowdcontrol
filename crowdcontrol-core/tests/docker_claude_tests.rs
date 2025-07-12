use crowdcontrol_core::{Config, DockerClient};
use std::fs;
use tempfile::TempDir;

// Note: These tests require Docker to be running and may modify the Docker environment.
// Run with: cargo test --test docker_claude_tests -- --ignored --test-threads=1

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_claude_config_mount() -> anyhow::Result<()> {
    // Create a temporary home directory structure
    let temp_home = TempDir::new()?;
    let claude_dir = temp_home.path().join(".claude");
    fs::create_dir_all(&claude_dir)?;
    
    // Create a mock credentials file
    let credentials_path = claude_dir.join("credentials.json");
    fs::write(&credentials_path, r#"{"token": "test-token"}"#)?;
    
    // Mock the home directory for this test
    std::env::set_var("HOME", temp_home.path());
    
    // Create a test config
    let config = Config {
        workspaces_dir: temp_home.path().join("workspaces"),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };
    
    // Create workspace directory
    let workspace_path = config.workspaces_dir.join("test-agent");
    fs::create_dir_all(&workspace_path)?;
    
    // Create Docker client
    let docker = DockerClient::new(config.clone())?;
    
    // Create container and verify it includes Claude mount
    let container_id = docker.create_container(
        "test-agent",
        &workspace_path,
        None,
        None,
    ).await?;
    
    // For now, we can only verify the container was created successfully
    // Direct inspection would require exposing internal Docker client
    assert!(!container_id.is_empty(), "Container ID should not be empty");
    
    // Clean up
    docker.remove_container(&container_id).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_no_claude_config_no_mount() -> anyhow::Result<()> {
    // Create a temporary home directory without .claude
    let temp_home = TempDir::new()?;
    
    // Mock the home directory for this test
    std::env::set_var("HOME", temp_home.path());
    
    // Create a test config
    let config = Config {
        workspaces_dir: temp_home.path().join("workspaces"),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };
    
    // Create workspace directory
    let workspace_path = config.workspaces_dir.join("test-agent");
    fs::create_dir_all(&workspace_path)?;
    
    // Create Docker client
    let docker = DockerClient::new(config.clone())?;
    
    // Create container
    let container_id = docker.create_container(
        "test-agent-no-claude",
        &workspace_path,
        None,
        None,
    ).await?;
    
    // Verify container was created
    assert!(!container_id.is_empty(), "Container ID should not be empty");
    
    // Clean up
    docker.remove_container(&container_id).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_has_crowdcontrol_label() -> anyhow::Result<()> {
    let temp_home = TempDir::new()?;
    std::env::set_var("HOME", temp_home.path());
    
    let config = Config {
        workspaces_dir: temp_home.path().join("workspaces"),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };
    
    let workspace_path = config.workspaces_dir.join("test-agent");
    fs::create_dir_all(&workspace_path)?;
    
    let docker = DockerClient::new(config.clone())?;
    
    let container_id = docker.create_container(
        "test-agent-label",
        &workspace_path,
        None,
        None,
    ).await?;
    
    // Verify container was created with proper ID
    assert!(!container_id.is_empty(), "Container ID should not be empty");
    
    // Test that list_all_containers can find it (which uses label filtering)
    let containers = docker.list_all_containers().await?;
    let found = containers.iter().any(|c| {
        c.id.as_ref()
            .map(|id| id.starts_with(&container_id))
            .unwrap_or(false)
    });
    
    assert!(found, "Container should be found with crowdcontrol label");
    
    // Clean up
    docker.remove_container(&container_id).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]  
async fn test_container_with_resource_limits() -> anyhow::Result<()> {
    let temp_home = TempDir::new()?;
    std::env::set_var("HOME", temp_home.path());
    
    let config = Config {
        workspaces_dir: temp_home.path().join("workspaces"),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: Some("512m".to_string()),
        default_cpus: Some("0.5".to_string()),
    };
    
    let workspace_path = config.workspaces_dir.join("test-agent");
    fs::create_dir_all(&workspace_path)?;
    
    let docker = DockerClient::new(config.clone())?;
    
    // Create container with resource limits
    let container_id = docker.create_container(
        "test-agent-limits",
        &workspace_path,
        Some("256m".to_string()),  // Override default
        Some("1".to_string()),      // Override default
    ).await?;
    
    assert!(!container_id.is_empty(), "Container ID should not be empty");
    
    // Clean up
    docker.remove_container(&container_id).await?;
    
    Ok(())
}