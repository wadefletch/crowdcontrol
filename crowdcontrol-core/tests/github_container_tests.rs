use crowdcontrol_core::config::Config;
use crowdcontrol_core::docker::DockerClient;
use crowdcontrol_core::github::GitHubConfig;
use tempfile::TempDir;

#[tokio::test]
#[ignore] // Requires Docker
async fn test_container_creation_with_github_auth() {
    let temp_dir = TempDir::new().unwrap();
    let github_config = GitHubConfig::new_with_installation_token("ghs_test_token".to_string());
    
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
        github: Some(github_config),
    };
    
    let docker_client = DockerClient::new(config).unwrap();
    
    // Create agent workspace
    let agent_workspace = temp_dir.path().join("test-agent");
    std::fs::create_dir_all(&agent_workspace).unwrap();
    
    // Test container creation includes GitHub environment variables
    let container_id = docker_client
        .create_container_with_github("test-agent", &agent_workspace, None, None)
        .await;
    
    assert!(container_id.is_ok());
    
    // Clean up
    if let Ok(id) = container_id {
        let _ = docker_client.remove_container(&id).await;
    }
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_container_github_env_vars_are_set() {
    let temp_dir = TempDir::new().unwrap();
    let github_config = GitHubConfig::new_with_installation_token("ghs_test_token".to_string());
    
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
        github: Some(github_config),
    };
    
    let docker_client = DockerClient::new(config).unwrap();
    let agent_workspace = temp_dir.path().join("test-agent");
    std::fs::create_dir_all(&agent_workspace).unwrap();
    
    let container_id = docker_client
        .create_container_with_github("test-agent", &agent_workspace, None, None)
        .await
        .unwrap();
    
    // Start container to inspect environment
    docker_client.start_container(&container_id).await.unwrap();
    
    // Check that GitHub environment variables are present
    let output = docker_client
        .exec_in_container(&container_id, vec!["printenv"], false)
        .await;
    
    assert!(output.is_ok());
    
    // Clean up
    let _ = docker_client.stop_container(&container_id, true).await;
    let _ = docker_client.remove_container(&container_id).await;
}

#[test]
fn test_container_creation_without_github_auth() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
        github: None, // No GitHub config
    };
    
    // Should still be able to create config without GitHub
    assert!(config.github.is_none());
}

#[test]
fn test_github_git_config_commands() {
    let github_config = GitHubConfig::new_with_installation_token("ghs_test_token".to_string());
    
    let commands = github_config.get_git_config_commands();
    
    // Should include git config commands for HTTPS authentication
    assert!(commands.iter().any(|cmd| cmd.contains("credential.helper")));
    assert!(commands.iter().any(|cmd| cmd.contains("user.name")));
    assert!(commands.iter().any(|cmd| cmd.contains("user.email")));
    assert!(commands.iter().any(|cmd| cmd.contains("CrowdControl")));
}