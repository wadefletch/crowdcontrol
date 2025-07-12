use crowdcontrol_core::config::Config;
use crowdcontrol_core::docker::DockerClient;
use crowdcontrol_core::github::GitHubConfig;
use std::env;
use tempfile::TempDir;

#[tokio::test]
#[ignore] // Requires Docker and GitHub token
async fn test_github_auth_integration() {
    // Skip test if no GitHub token available
    let github_token = match env::var("GITHUB_INSTALLATION_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            println!("Skipping GitHub integration test - no GITHUB_INSTALLATION_TOKEN set");
            return;
        }
    };

    let temp_dir = TempDir::new().unwrap();
    let github_config = GitHubConfig::new_with_installation_token(github_token);
    
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
    let agent_workspace = temp_dir.path().join("github-test-agent");
    std::fs::create_dir_all(&agent_workspace).unwrap();

    // Test container creation with GitHub auth
    let container_id = docker_client
        .create_container_with_github("github-test-agent", &agent_workspace, None, None)
        .await
        .expect("Failed to create container with GitHub auth");

    // Start container
    docker_client
        .start_container(&container_id)
        .await
        .expect("Failed to start container");

    // Wait a moment for container to initialize
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test that git config is properly set
    let git_config_result = docker_client
        .exec_in_container(
            &container_id,
            vec!["su", "developer", "-c", "git config --global user.name"],
            false,
        )
        .await;

    assert!(git_config_result.is_ok());

    // Test that credentials are configured
    let creds_result = docker_client
        .exec_in_container(
            &container_id,
            vec!["su", "developer", "-c", "test -f ~/.git-credentials && echo 'credentials exist'"],
            false,
        )
        .await;

    assert!(creds_result.is_ok());

    // Clean up
    let _ = docker_client.stop_container(&container_id, true).await;
    let _ = docker_client.remove_container(&container_id).await;
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_container_without_github_auth() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
        github: None, // No GitHub config
    };

    let docker_client = DockerClient::new(config).unwrap();
    let agent_workspace = temp_dir.path().join("no-github-agent");
    std::fs::create_dir_all(&agent_workspace).unwrap();

    // Should still be able to create container without GitHub
    let container_id = docker_client
        .create_container_with_github("no-github-agent", &agent_workspace, None, None)
        .await
        .expect("Failed to create container without GitHub auth");

    // Clean up
    let _ = docker_client.remove_container(&container_id).await;
}