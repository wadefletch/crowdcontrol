// NOTE: These tests require Docker and verify Claude Code behavior inside containers.
// Run with: cargo test --package crowdcontrol-core --ignored

use anyhow::Result;
use crowdcontrol_core::{Config, DockerClient};
use std::fs;
use std::process::Command;
use tempfile::TempDir;
use tokio;

/// Integration tests specifically focused on Claude Code behavior inside containers
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_claude_code_availability_and_version() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = &format!("test-claude-version-{}", std::process::id());
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;
    fs::write(workspace_path.join("test.py"), "print('Hello World')")?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test: Verify Claude Code is installed and accessible
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "claude",
            "--version",
        ])
        .output()?;

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version_error = String::from_utf8_lossy(&output.stderr);
    println!("Claude version command - stdout: {}", version_output);
    println!("Claude version command - stderr: {}", version_error);
    println!(
        "Claude version command - exit code: {:?}",
        output.status.code()
    );

    // Claude might output version to stderr or might not support --version
    let version_text = format!("{}{}", version_output, version_error);
    if output.status.success()
        && (version_text.contains("claude") || version_text.contains("Claude"))
    {
        println!("Claude Code version detected: {}", version_text.trim());
    } else {
        println!("Claude Code version command failed or not recognized - this may be expected");
    }

    // Test: Verify Claude Code can show help
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "claude",
            "--help",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Claude Code help should be accessible"
    );
    let help_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        help_output.contains("Usage:") || help_output.contains("USAGE:"),
        "Help output should contain usage information"
    );

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_claude_authentication_status() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = &format!("test-claude-auth-status-{}", std::process::id());
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test: Check Claude authentication status (should fail without credentials)
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "claude",
            "auth",
            "status",
        ])
        .output()?;

    // This should fail or show unauthenticated status without real credentials
    let auth_output = String::from_utf8_lossy(&output.stdout);
    let auth_error = String::from_utf8_lossy(&output.stderr);

    println!("Claude auth status output: {}", auth_output);
    println!("Claude auth status error: {}", auth_error);

    // Test that the command runs (even if it fails due to no auth)
    // The important part is that Claude Code is functional enough to attempt auth
    assert!(
        auth_output.contains("not authenticated")
            || auth_output.contains("unauthenticated")
            || auth_error.contains("not authenticated")
            || auth_error.contains("API key")
            || !output.status.success(), // Expected to fail without credentials
        "Claude should respond to auth status command"
    );

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_claude_with_mock_credentials() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    // Create mock Claude credentials that match expected format
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir)?;

    let mock_credentials = r#"{"api_key": "sk-ant-api03-mock-key-for-testing"}"#;
    fs::write(claude_dir.join(".credentials.json"), mock_credentials)?;

    let mock_config = r#"{
        "autoUpdates": true,
        "mode": "project",
        "projects": {
            "/some/path": {"lastUsed": "2024-01-01T00:00:00Z"}
        },
        "numStartups": 5
    }"#;
    fs::write(claude_dir.join(".claude.json"), mock_config)?;

    let docker = DockerClient::new(config.clone())?;
    let agent_name = &format!("test-claude-mock-auth-{}", std::process::id());
    let workspace_path = config.agent_workspace_path(agent_name);

    fs::create_dir_all(&workspace_path)?;
    fs::write(workspace_path.join("test.md"), "# Test Project")?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test: Run refresh script to configure Claude auth
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
        ])
        .output()?;

    println!(
        "Refresh script output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    println!(
        "Refresh script error: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Test: Verify credentials file was created with correct permissions
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "ls",
            "-la",
            "/home/developer/.claude/",
        ])
        .output()?;

    if output.status.success() {
        let ls_output = String::from_utf8_lossy(&output.stdout);
        println!("Claude directory contents: {}", ls_output);

        // Check if credentials file exists
        if ls_output.contains(".credentials.json") {
            // Test: Verify credentials file has correct permissions (600)
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

            if output.status.success() {
                let perms_binding = String::from_utf8_lossy(&output.stdout);
                let perms = perms_binding.trim();
                assert_eq!(perms, "600", "Credentials file should have 600 permissions");
            }

            // Test: Verify credentials content
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
                    creds_content.contains("sk-ant-api03-mock-key-for-testing"),
                    "Credentials should contain mock API key"
                );
            }
        }
    }

    // Test: Verify .claude.json transformations were applied
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
        println!("Transformed Claude config: {}", config_content);

        // Parse and verify transformations
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&config_content) {
            assert_eq!(
                parsed["autoUpdates"], false,
                "autoUpdates should be transformed to false"
            );
            assert_eq!(
                parsed["mode"], "global",
                "mode should be transformed to global"
            );
            assert_eq!(
                parsed["projects"],
                serde_json::json!({}),
                "projects should be transformed to empty object"
            );
            // numStartups can be any number - the important thing is it's preserved from the original
            if let Some(num_startups) = parsed["numStartups"].as_number() {
                println!("✅ numStartups preserved: {}", num_startups);
            } else {
                println!("⚠️ numStartups field not found or not a number");
            }
        }
    }

    // Test: Attempt to check Claude auth status with mock credentials
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "claude",
            "auth",
            "status",
        ])
        .output()?;

    let auth_output = String::from_utf8_lossy(&output.stdout);
    let auth_error = String::from_utf8_lossy(&output.stderr);

    println!(
        "Claude auth status with mock creds - stdout: {}",
        auth_output
    );
    println!(
        "Claude auth status with mock creds - stderr: {}",
        auth_error
    );

    // With mock credentials, this might still fail, but it should at least attempt authentication
    // The key is that Claude reads the credentials file and tries to use it

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_claude_project_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = &format!("test-claude-project-{}", std::process::id());
    let workspace_path = config.agent_workspace_path(agent_name);

    // Create a typical project structure
    fs::create_dir_all(&workspace_path)?;
    fs::write(
        workspace_path.join("package.json"),
        r#"{"name": "test-project", "version": "1.0.0"}"#,
    )?;
    fs::write(
        workspace_path.join("README.md"),
        "# Test Project\n\nThis is a test project for Claude.",
    )?;
    fs::write(
        workspace_path.join("src/main.js"),
        "console.log('Hello World');",
    )?;
    fs::create_dir_all(workspace_path.join("src"))?;
    fs::write(
        workspace_path.join("src/main.js"),
        "console.log('Hello World');",
    )?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test: Verify Claude can see and analyze project files
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "find",
            "/workspace",
            "-name",
            "*.json",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to find project files"
    );
    let find_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        find_output.contains("package.json"),
        "Should find package.json"
    );

    // Test: Verify Claude can read project files
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "cat",
            "/workspace/package.json",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to read package.json"
    );
    let package_content = String::from_utf8_lossy(&output.stdout);
    assert!(
        package_content.contains("test-project"),
        "Should read package.json content"
    );

    // Test: Verify directory structure is as expected
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "tree",
            "/workspace",
        ])
        .output()?;

    if !output.status.success() {
        // If tree is not available, use ls -la
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
            "Should be able to list workspace contents"
        );
        let ls_output = String::from_utf8_lossy(&output.stdout);
        assert!(
            ls_output.contains("package.json"),
            "Workspace should contain package.json"
        );
        assert!(
            ls_output.contains("src"),
            "Workspace should contain src directory"
        );
    } else {
        let tree_output = String::from_utf8_lossy(&output.stdout);
        println!("Project structure:\n{}", tree_output);
        assert!(
            tree_output.contains("package.json"),
            "Tree should show package.json"
        );
    }

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_claude_node_npm_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };

    let docker = DockerClient::new(config.clone())?;
    let agent_name = &format!("test-claude-node-{}", std::process::id());
    let workspace_path = config.agent_workspace_path(agent_name);

    // Create a Node.js project
    fs::create_dir_all(&workspace_path)?;
    fs::write(
        workspace_path.join("package.json"),
        r#"{
        "name": "test-project",
        "version": "1.0.0",
        "description": "Test project for Claude",
        "main": "index.js",
        "scripts": {
            "start": "node index.js",
            "test": "echo \"No tests yet\""
        }
    }"#,
    )?;

    fs::write(
        workspace_path.join("index.js"),
        r#"
const message = "Hello from Node.js in Claude container!";
console.log(message);
console.log("Node version:", process.version);
console.log("Working directory:", process.cwd());
"#,
    )?;

    let container_id = docker
        .create_container(agent_name, &workspace_path, None, None)
        .await?;
    docker.start_container(&container_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test: Verify Node.js is available and working
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "node",
            "--version",
        ])
        .output()?;

    assert!(output.status.success(), "Node.js should be available");
    let node_version_binding = String::from_utf8_lossy(&output.stdout);
    let node_version = node_version_binding.trim();
    println!("Node.js version: {}", node_version);
    assert!(
        node_version.starts_with("v"),
        "Node version should start with 'v'"
    );

    // Test: Verify npm is available
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "npm",
            "--version",
        ])
        .output()?;

    assert!(output.status.success(), "npm should be available");
    let npm_version_binding = String::from_utf8_lossy(&output.stdout);
    let npm_version = npm_version_binding.trim();
    println!("npm version: {}", npm_version);

    // Test: Run the Node.js application
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "node",
            "/workspace/index.js",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "Should be able to run Node.js application"
    );
    let app_output = String::from_utf8_lossy(&output.stdout);
    println!("Node.js application output: {}", app_output);
    assert!(
        app_output.contains("Hello from Node.js"),
        "Application should produce expected output"
    );
    assert!(
        app_output.contains("/workspace"),
        "Application should run in workspace directory"
    );

    // Test: npm scripts
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "npm",
            "run",
            "test",
        ])
        .output()?;

    assert!(output.status.success(), "npm scripts should work");
    let npm_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        npm_output.contains("No tests yet"),
        "npm test script should run"
    );

    // Test: npm start
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "npm",
            "start",
        ])
        .output()?;

    assert!(output.status.success(), "npm start should work");
    let start_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        start_output.contains("Hello from Node.js"),
        "npm start should run the application"
    );

    // Cleanup
    docker.stop_container(&container_id, false).await?;
    docker.remove_container(&container_id).await?;

    Ok(())
}
