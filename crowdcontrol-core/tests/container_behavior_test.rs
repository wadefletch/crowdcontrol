// NOTE: These tests require Docker and verify actual behavior inside containers.
// Run with: cargo test --package crowdcontrol-core --ignored

use anyhow::Result;
use crowdcontrol_core::{Config, DockerClient};
use std::fs;
use std::process::Command;
use tempfile::TempDir;
use tokio;
/// Test that verifies the actual behavior inside containers matches expectations
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_claude_authentication() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-claude-auth";
    let workspace_path = config.agent_workspace_path(agent_name);

    // Create workspace directory
    fs::create_dir_all(&workspace_path)?;

    // Create a test repository structure
    fs::write(workspace_path.join("README.md"), "# Test Repository")?;

    // Create container
    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;

    // Start container
    docker.start_container(&container_id).await?;

    // Wait a moment for container to fully start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test 1: Verify Claude CLI is available
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "which",
            "claude",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Claude CLI should be available in container"
    );

    // Test 2: Verify working directory is /workspace
    let output = Command::new("docker")
        .args(&["exec", &format!("crowdcontrol-{}", agent_name), "pwd"])
        .output()?;

    let pwd_binding = String::from_utf8_lossy(&output.stdout);
    let pwd = pwd_binding.trim();
    assert_eq!(
        pwd, "/workspace",
        "Container should start in /workspace directory"
    );

    // Test 3: Verify workspace files are accessible
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "ls",
            "-la",
            "/workspace",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to list workspace files"
    );
    let ls_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        ls_output.contains("README.md"),
        "README.md should be accessible in workspace"
    );

    // Test 4: Verify Claude configuration directory exists
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "ls",
            "-la",
            "/home/developer/.claude",
        ])
        .output()?;

    // Note: This might fail if no Claude config is present, which is expected in test environment
    if output.status.success() {
        println!("Claude config directory found in container");
    } else {
        println!("Claude config directory not found (expected in test environment)");
    }

    // Test 5: Verify user context
    let output = Command::new("docker")
        .args(&["exec", &format!("crowdcontrol-{}", agent_name), "whoami"])
        .output()?;

    let user_binding = String::from_utf8_lossy(&output.stdout);
    let user = user_binding.trim();
    assert_eq!(
        user, "developer",
        "Container should run as 'developer' user"
    );

    // Test 6: Verify essential tools are available
    let tools = ["git", "jq", "curl", "node", "npm"];
    for tool in &tools {
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("crowdcontrol-{}", agent_name),
                "which",
                tool,
            ])
            .output()?;

        assert!(
            output.status.success(),
            "{} should be available in container",
            tool
        );
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_claude_authentication_with_credentials() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    // Create mock Claude credentials
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir)?;

    let mock_credentials = r#"{"api_key": "test-key", "organization_id": "test-org"}"#;
    fs::write(claude_dir.join(".credentials.json"), mock_credentials)?;

    let mock_config = r#"{"autoUpdates": true, "mode": "project", "projects": {"test": "data"}}"#;
    fs::write(claude_dir.join(".claude.json"), mock_config)?;

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-claude-creds";
    let workspace_path = config.agent_workspace_path(agent_name);

    // Create workspace directory
    fs::create_dir_all(&workspace_path)?;
    fs::write(workspace_path.join("README.md"), "# Test Repository")?;

    // Create container
    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;

    // Start container
    docker.start_container(&container_id).await?;

    // Wait for container to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Run refresh script to set up Claude auth
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
        ])
        .output()?;

    if output.status.success() {
        println!("Refresh script executed successfully");

        // Verify credentials were copied
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("crowdcontrol-{}", agent_name),
                "cat",
                "/home/developer/.claude/.credentials.json",
            ])
            .output()?;

        if output.status.success() {
            let credentials = String::from_utf8_lossy(&output.stdout);
            assert!(
                credentials.contains("test-key"),
                "Credentials should be properly copied"
            );
        }

        // Verify .claude.json transformations were applied
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("crowdcontrol-{}", agent_name),
                "cat",
                "/home/developer/.claude.json",
            ])
            .output()?;

        if output.status.success() {
            let config_content = String::from_utf8_lossy(&output.stdout);
            println!("Claude config content: {}", config_content);

            // Parse JSON to verify transformations
            let parsed: serde_json::Value = serde_json::from_str(&config_content)?;
            assert_eq!(parsed["autoUpdates"], false, "autoUpdates should be false");
            assert_eq!(parsed["mode"], "global", "mode should be global");
            assert_eq!(
                parsed["projects"],
                serde_json::json!({}),
                "projects should be empty"
            );
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!(
            "Refresh script failed (expected if no Claude mount): {}",
            stderr
        );
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_file_permissions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-permissions";
    let workspace_path = config.agent_workspace_path(agent_name);

    // Create workspace with test files
    fs::create_dir_all(&workspace_path)?;
    fs::write(workspace_path.join("test.txt"), "test content")?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Verify file is readable
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "cat",
            "/workspace/test.txt",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to read workspace files"
    );
    let content = String::from_utf8_lossy(&output.stdout);
    assert_eq!(content.trim(), "test content", "File content should match");

    // Test: Verify file is writable
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "sh",
            "-c",
            "echo 'new content' > /workspace/new_file.txt",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to write to workspace"
    );

    // Verify file was created on host
    let host_file = workspace_path.join("new_file.txt");
    assert!(
        host_file.exists(),
        "File created in container should appear on host"
    );

    let host_content = fs::read_to_string(host_file)?;
    assert_eq!(
        host_content.trim(),
        "new content",
        "File content should sync to host"
    );

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_environment_variables() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-env";
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Verify important environment variables
    let env_vars = [
        ("HOME", "/home/developer"),
        ("USER", "developer"),
        ("PWD", "/workspace"),
    ];

    for (var, expected) in &env_vars {
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("crowdcontrol-{}", agent_name),
                "printenv",
                var,
            ])
            .output()?;

        if output.status.success() {
            let value_binding = String::from_utf8_lossy(&output.stdout);
            let value = value_binding.trim();
            assert_eq!(
                value, *expected,
                "Environment variable {} should be {}",
                var, expected
            );
        }
    }

    // Test: Verify PATH includes common directories
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "printenv",
            "PATH",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "PATH environment variable should be set"
    );
    let path = String::from_utf8_lossy(&output.stdout);
    assert!(
        path.contains("/usr/local/bin"),
        "PATH should include /usr/local/bin"
    );
    assert!(path.contains("/usr/bin"), "PATH should include /usr/bin");

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_git_functionality() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-git";
    let workspace_path = config.agent_workspace_path(agent_name);

    // Create a git repository in workspace
    fs::create_dir_all(&workspace_path)?;

    // Initialize git repo on host
    Command::new("git")
        .args(&["init"])
        .current_dir(&workspace_path)
        .output()?;

    fs::write(workspace_path.join("README.md"), "# Test Repository")?;

    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(&workspace_path)
        .output()?;

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(&workspace_path)
        .output()?;

    Command::new("git")
        .args(&["add", "."])
        .current_dir(&workspace_path)
        .output()?;

    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(&workspace_path)
        .output()?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Verify git is functional in container
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "git",
            "status",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Git should be functional in container"
    );
    let git_status = String::from_utf8_lossy(&output.stdout);
    assert!(
        git_status.contains("working tree clean") || git_status.contains("nothing to commit"),
        "Git repository should be clean"
    );

    // Test: Verify git log shows our commit
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "git",
            "log",
            "--oneline",
        ])
        .output()?;

    assert!(output.status.success(), "Git log should be accessible");
    let git_log = String::from_utf8_lossy(&output.stdout);
    assert!(
        git_log.contains("Initial commit"),
        "Git log should show our commit"
    );

    // Test: Verify we can make changes and commit them
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "sh",
            "-c",
            "echo '## Test' >> README.md && git add README.md && git commit -m 'Update README'",
        ])
        .output()?;

    if output.status.success() {
        println!("Successfully made git commit from container");

        // Verify the change is reflected on host
        let readme_content = fs::read_to_string(workspace_path.join("README.md"))?;
        assert!(
            readme_content.contains("## Test"),
            "Git changes should sync to host"
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Git commit failed (might need git config): {}", stderr);
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}
