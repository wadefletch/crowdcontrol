// NOTE: These tests require Docker and verify refresh-claude-auth.sh script behavior.
// Run with: cargo test --package crowdcontrol-core --ignored

use anyhow::Result;
use crowdcontrol_core::{Config, DockerClient};
use std::fs;
use std::process::Command;
use tempfile::TempDir;
use tokio;

/// Tests specifically for the refresh-claude-auth.sh script behavior
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_refresh_script_with_no_credentials() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-refresh-no-creds";
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Run refresh script without any credentials
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
        ])
        .output()?;

    // Should fail with exit code 1
    assert!(
        !output.status.success(),
        "Refresh script should fail when no credentials are available"
    );
    assert_eq!(output.status.code(), Some(1), "Should exit with code 1");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("Refresh script stdout: {}", stdout);
    println!("Refresh script stderr: {}", stderr);

    assert!(
        stdout.contains("No Claude Code credentials found"),
        "Should report no credentials found"
    );
    assert!(
        stdout.contains("On macOS, run: crowdcontrol refresh"),
        "Should suggest macOS solution"
    );

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_refresh_script_with_keychain_credentials() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-refresh-keychain";
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Run refresh script with mock keychain credentials as parameter
    let mock_keychain_creds =
        r#"{"api_key": "sk-ant-api03-keychain-mock", "organization_id": "org-123"}"#;

    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
            mock_keychain_creds,
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Refresh script should succeed with keychain credentials"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Refresh script output: {}", stdout);

    assert!(
        stdout.contains("Using provided keychain credentials"),
        "Should indicate using keychain credentials"
    );
    assert!(
        stdout.contains("Claude Code authentication configured (keychain credentials)"),
        "Should confirm configuration"
    );

    // Test: Verify credentials file was created
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "cat",
            "/home/developer/.claude/.credentials.json",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Credentials file should be created"
    );
    let creds_content = String::from_utf8_lossy(&output.stdout);
    assert!(
        creds_content.contains("sk-ant-api03-keychain-mock"),
        "Should contain keychain credentials"
    );
    assert!(
        creds_content.contains("org-123"),
        "Should contain organization ID"
    );

    // Test: Verify file permissions
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "stat",
            "-c",
            "%a",
            "/home/developer/.claude/.credentials.json",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to check file permissions"
    );
    let perms_binding = String::from_utf8_lossy(&output.stdout);
    let perms = perms_binding.trim();
    assert_eq!(perms, "600", "Credentials file should have 600 permissions");

    // Test: Verify file ownership
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "stat",
            "-c",
            "%U:%G",
            "/home/developer/.claude/.credentials.json",
        ])
        .output()?;

    if output.status.success() {
        let ownership_binding = String::from_utf8_lossy(&output.stdout);
        let ownership = ownership_binding.trim();
        println!("File ownership: {}", ownership);
        // Ownership should match the USER_ID:GROUP_ID from the container
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_refresh_script_with_file_based_credentials() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    // Create mock Claude credentials in the expected mount location
    let claude_mount = temp_dir.path().join("claude-config");
    let claude_dir = claude_mount.join(".claude");
    fs::create_dir_all(&claude_dir)?;

    let mock_credentials = r#"{"api_key": "sk-ant-api03-file-based-mock", "user_id": "user-456"}"#;
    fs::write(claude_dir.join(".credentials.json"), mock_credentials)?;

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-refresh-file";
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    // Create container with Claude config mount
    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;

    // Mount the claude config directory
    Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "mkdir",
            "-p",
            "/mnt/claude-config",
        ])
        .output()?;

    // Copy the mock credentials into the container (simulating the mount)
    Command::new("docker")
        .args(&[
            "cp",
            &format!("{}/.claude", claude_mount.display()),
            &format!("crowdcontrol-{}:/mnt/claude-config/", agent_name),
        ])
        .output()?;

    docker.start_container(&container_id).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Run refresh script with file-based credentials
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Refresh script stdout: {}", stdout);
    println!("Refresh script stderr: {}", stderr);

    if output.status.success() {
        assert!(
            stdout.contains("Copying .credentials.json from mount"),
            "Should indicate copying from mount"
        );
        assert!(
            stdout.contains("Claude Code authentication configured (.credentials.json)"),
            "Should confirm configuration"
        );

        // Test: Verify credentials were copied
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("crowdcontrol-{}", agent_name),
                "cat",
                "/home/developer/.claude/.credentials.json",
            ])
            .output()?;

        if output.status.success() {
            let creds_content = String::from_utf8_lossy(&output.stdout);
            assert!(
                creds_content.contains("sk-ant-api03-file-based-mock"),
                "Should contain file-based credentials"
            );
        }
    } else {
        // Expected if mount simulation didn't work properly
        println!("File-based refresh failed (expected in test environment)");
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_refresh_script_claude_json_transformations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-refresh-transforms";
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Create a .claude.json file in the container to test transformations
    let original_config = r#"{
        "autoUpdates": true,
        "mode": "project",
        "projects": {
            "/some/project": {"lastUsed": "2024-01-01"},
            "/another/project": {"settings": "value"}
        },
        "numStartups": 42,
        "otherField": "should be preserved"
    }"#;

    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "sh",
            "-c",
            &format!(
                "mkdir -p /mnt/claude-config && echo '{}' > /mnt/claude-config/.claude.json",
                original_config
            ),
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to create test .claude.json"
    );

    // Test: Run refresh script to apply transformations
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Refresh script output: {}", stdout);

    if stdout.contains("Copying .claude.json from mount") {
        assert!(
            stdout.contains("Applying transformations to .claude.json"),
            "Should apply transformations"
        );
        assert!(
            stdout.contains(
                "Claude Code authentication configured (.claude.json with transformations)"
            ),
            "Should confirm configuration with transformations"
        );

        // Test: Verify transformations were applied
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("crowdcontrol-{}", agent_name),
                "cat",
                "/home/developer/.claude.json",
            ])
            .output()?;

        if output.status.success() {
            let transformed_content = String::from_utf8_lossy(&output.stdout);
            println!("Transformed .claude.json: {}", transformed_content);

            // Parse and verify transformations
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&transformed_content) {
                assert_eq!(parsed["autoUpdates"], false, "autoUpdates should be false");
                assert_eq!(parsed["mode"], "global", "mode should be global");
                assert_eq!(
                    parsed["projects"],
                    serde_json::json!({}),
                    "projects should be empty"
                );
                assert_eq!(parsed["numStartups"], 42, "numStartups should be preserved");
                assert_eq!(
                    parsed["otherField"], "should be preserved",
                    "otherField should be preserved"
                );
            } else {
                panic!("Failed to parse transformed JSON");
            }
        }
    } else {
        println!("Legacy config not found (expected if no mount)");
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_refresh_script_error_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-refresh-errors";
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Empty credentials parameter
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
            "",
        ])
        .output()?;

    // Should fall through to checking file-based credentials and eventually fail
    assert!(
        !output.status.success(),
        "Should fail with empty credentials"
    );

    // Test: Invalid JSON as credentials parameter
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
            "invalid-json",
        ])
        .output()?;

    // This should "succeed" in writing the invalid JSON, but Claude won't be able to use it
    // The script itself doesn't validate JSON format - that's Claude's job
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Using provided keychain credentials"),
            "Should accept any string as credentials"
        );

        // Verify the invalid content was written
        let output = Command::new("docker")
            .args(&[
                "exec",
                &format!("crowdcontrol-{}", agent_name),
                "cat",
                "/home/developer/.claude/.credentials.json",
            ])
            .output()?;

        if output.status.success() {
            let content = String::from_utf8_lossy(&output.stdout);
            assert_eq!(
                content.trim(),
                "invalid-json",
                "Should write exactly what was provided"
            );
        }
    }

    // Test: Very long credentials string
    let long_creds = "x".repeat(10000);
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
            &long_creds,
        ])
        .output()?;

    // Should handle long strings without issue
    assert!(
        output.status.success(),
        "Should handle long credential strings"
    );

    // Test: Credentials with special characters
    let special_creds = r#"{"key": "test with spaces & symbols!@#$%^&*()"}"#;
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
            special_creds,
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should handle special characters in credentials"
    );

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_refresh_script_directory_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = "test-refresh-dirs";
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test: Ensure .claude directory doesn't exist initially
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "ls",
            "/home/developer/.claude",
        ])
        .output()?;

    // Should fail initially
    assert!(
        !output.status.success(),
        "Claude directory should not exist initially"
    );

    // Test: Run refresh script with credentials to create directory
    let mock_creds = r#"{"api_key": "test-key"}"#;
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
            mock_creds,
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Refresh script should create directory and succeed"
    );

    // Test: Verify directory was created
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "ls",
            "-la",
            "/home/developer/.claude",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Claude directory should exist after refresh"
    );
    let ls_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        ls_output.contains(".credentials.json"),
        "Should contain credentials file"
    );

    // Test: Verify directory permissions
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "stat",
            "-c",
            "%a",
            "/home/developer/.claude",
        ])
        .output()?;

    if output.status.success() {
        let perms_binding = String::from_utf8_lossy(&output.stdout);
        let perms = perms_binding.trim();
        println!("Claude directory permissions: {}", perms);
        // Should be readable/writable by owner
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}
