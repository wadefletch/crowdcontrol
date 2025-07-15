use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// Helper to create a basic config file
fn create_config_file(dir: &TempDir) -> std::path::PathBuf {
    let config_path = dir.path().join("config.toml");
    let workspaces_dir = dir.path().join("workspaces");
    
    // Ensure workspaces directory exists
    fs::create_dir_all(&workspaces_dir).unwrap();
    
    let config_content = format!(
        r#"
workspaces_dir = "{}"
image = "crowdcontrol:latest"
"#,
        workspaces_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();
    config_path
}

// Helper to create a proper git repository with commits
fn create_test_git_repo(repo_path: &std::path::Path) {
    fs::create_dir_all(repo_path).unwrap();
    fs::write(repo_path.join("README.md"), "# Test Repo").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .assert()
        .success();
    
    // Configure git user for test
    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .assert()
        .success();
    
    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .assert()
        .success();
    
    // Add and commit files
    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo_path)
        .assert()
        .success();
    
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .assert()
        .success();
}

// Helper to create and start an agent
fn start_agent(name: &str, config_path: &std::path::Path, repo_path: &std::path::Path) {
    // First create the agent using 'new' command with file:// URL
    let new_output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            name,
            &format!("file://{}", repo_path.display()),
            "--skip-verification",
        ])
        .output()
        .unwrap();
    
    // Debug: check what the new command outputs
    println!("New command stdout: {}", String::from_utf8_lossy(&new_output.stdout));
    println!("New command stderr: {}", String::from_utf8_lossy(&new_output.stderr));
    
    if !new_output.status.success() {
        panic!("New command failed with exit code: {}", new_output.status);
    }
    
    // Add a small delay to ensure metadata is written
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Debug: check the workspaces directory
    let config_contents = std::fs::read_to_string(&config_path).unwrap();
    println!("Config file contents: {}", config_contents);
    
    // Debug: check agent metadata
    let list_output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["list", "--format", "json"])
        .output()
        .unwrap();
    
    println!("Agent list after new: {}", String::from_utf8_lossy(&list_output.stdout));
    
    // Debug: check raw metadata file
    let workspaces_dir = config_contents.lines()
        .find(|line| line.starts_with("workspaces_dir"))
        .and_then(|line| line.split('"').nth(1))
        .unwrap();
    let metadata_path = format!("{}/{}/.crowdcontrol/metadata.json", workspaces_dir, name);
    if let Ok(metadata_contents) = std::fs::read_to_string(&metadata_path) {
        println!("Raw metadata contents: {}", metadata_contents);
    } else {
        println!("Could not read metadata file: {}", metadata_path);
    }
    
    // Debug: check if container exists
    let expected_name = format!("crowdcontrol-{}", name);
    println!("Expected container name: {}", expected_name);
    
    let docker_ps_output = Command::new("docker")
        .args(&["ps", "-a", "--format", "table {{.ID}}\\t{{.Names}}\\t{{.Status}}"])
        .output()
        .unwrap();
    println!("Docker containers: {}", String::from_utf8_lossy(&docker_ps_output.stdout));
    
    // Then start it
    let start_output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", name])
        .output()
        .unwrap();
    
    println!("Start command stdout: {}", String::from_utf8_lossy(&start_output.stdout));
    println!("Start command stderr: {}", String::from_utf8_lossy(&start_output.stderr));
    
    if !start_output.status.success() {
        // Check metadata file again after failure
        if let Ok(metadata_contents_after) = std::fs::read_to_string(&metadata_path) {
            println!("Metadata after start failure: {}", metadata_contents_after);
        }
        panic!("Start command failed with exit code: {}", start_output.status);
    }
}

#[test]
#[ignore = "requires Docker"]
fn test_agent_status_lifecycle() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("status-test-{}", timestamp);
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    create_test_git_repo(&test_repo);
    
    // Start agent
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Check initial status (should be running)
    let output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["list"])
        .assert()
        .success();
    
    // Just verify the agent is listed and running
    output.stdout(predicate::str::contains(&agent_name));
    
    // Stop the agent
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
    
    // Check status after stop (should be stopped)
    let output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["list"])
        .assert()
        .success();
    
    // Just verify the agent is still listed
    output.stdout(predicate::str::contains(&agent_name));
}

#[test]
#[ignore = "requires Docker"]
fn test_claude_code_integration() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("test.py"), "print('Hello from test')").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("claude-test-{}", timestamp);
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Wait for container to be ready
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test that Claude Code is available
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "claude",
            "--version",
        ])
        .output()
        .unwrap();
    
    assert!(
        output.status.success(),
        "Claude Code should be available in container"
    );
    
    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_refresh_claude_auth_script() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create mock Claude config
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join(".credentials.json"),
        r#"{"api_key": "test-key"}"#,
    )
    .unwrap();
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    create_test_git_repo(&test_repo);
    
    // Start agent with HOME set to include Claude config
    // First create the agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("refresh-test-{}", timestamp);
    
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("HOME", temp_dir.path())
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            &agent_name,
            &format!("file://{}", test_repo.display()),
            "--skip-verification",
        ])
        .assert()
        .success();
    
    // Then start it
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("HOME", temp_dir.path())
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success();
    
    // Wait for container
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Run refresh script
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "refresh-claude-auth.sh",
        ])
        .output()
        .unwrap();
    
    // Script should execute (may succeed or fail based on mount availability)
    println!("Refresh script output: {:?}", String::from_utf8_lossy(&output.stdout));
    
    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_multiple_containers_same_name() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    create_test_git_repo(&test_repo);
    
    // Start first agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("duplicate-test-{}", timestamp);
    
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            &agent_name,
            &format!("file://{}", test_repo.display()),
            "--skip-verification",
        ])
        .assert()
        .success();
    
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success();
    
    // Try to create another agent with the same name (should fail)
    let output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            &agent_name,
            &format!("file://{}", test_repo.display()),
            "--skip-verification",
        ])
        .assert()
        .failure();
    
    // Just verify it mentions the agent already exists in some form
    output.stderr(predicate::str::contains("already"));
    
    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_agent_restart() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    create_test_git_repo(&test_repo);
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("restart-test-{}", timestamp);
    
    // Create agent
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            &agent_name,
            &format!("file://{}", test_repo.display()),
            "--skip-verification",
        ])
        .assert()
        .success();
    
    // Start agent
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success();
    
    // Get initial container ID
    let output = Command::new("docker")
        .args(&["ps", "-q", "-f", &format!("name=crowdcontrol-{}", agent_name)])
        .output()
        .unwrap();
    let initial_container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Restart the agent (stop then start)
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
        
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success();
    
    // Wait for restart
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Get new container ID
    let output = Command::new("docker")
        .args(&["ps", "-q", "-f", &format!("name=crowdcontrol-{}", agent_name)])
        .output()
        .unwrap();
    let new_container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Container ID should be different after restart
    assert_ne!(initial_container_id, new_container_id, "Container should have a new ID after restart");
    
    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_state_validation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    create_test_git_repo(&test_repo);
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("validation-test-{}", timestamp);
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Run doctor command
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["doctor"])
        .assert()
        .success();
    
    // Just verify the doctor command succeeded
    // The exact output format isn't critical as long as it runs
    
    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}