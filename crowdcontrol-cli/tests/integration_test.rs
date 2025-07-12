use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::sync::Once;
use std::time::Duration;

// NOTE: These tests use Docker and shared workspaces, so they may interfere
// with each other if run in parallel. For reliable results, run them serially:
// cargo test --test integration_test -- --test-threads=1

static INIT: Once = Once::new();

fn get_test_workspaces_dir() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let test_workspaces = PathBuf::from(manifest_dir).join("test-workspaces");

    // Create directory if it doesn't exist
    fs::create_dir_all(&test_workspaces).expect("Failed to create test workspaces directory");

    test_workspaces
}

fn check_docker_running() -> bool {
    let output = StdCommand::new("docker")
        .arg("info")
        .output()
        .expect("Failed to execute docker info");

    output.status.success()
}

fn crowdcontrol_cmd() -> Command {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir").arg(get_test_workspaces_dir());
    cmd
}

fn ensure_docker_image_built() {
    INIT.call_once(|| {
        // First check if Docker is running
        if !check_docker_running() {
            panic!("Docker is not running! Please start Docker and try again.");
        }

        println!("Checking if crowdcontrol:latest image exists...");

        // Check if image exists
        let check = StdCommand::new("docker")
            .args(&["image", "inspect", "crowdcontrol:latest"])
            .output()
            .expect("Failed to check docker image");

        if !check.status.success() {
            println!("Building crowdcontrol:latest image...");

            let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
            let container_dir = PathBuf::from(manifest_dir)
                .parent()
                .unwrap()
                .join("container");

            let output = StdCommand::new("docker")
                .args(&["build", "-t", "crowdcontrol:latest", "."])
                .current_dir(&container_dir)
                .output()
                .expect("Failed to build docker image");

            if !output.status.success() {
                panic!(
                    "Failed to build Docker image: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            println!("Docker image built successfully!");
        } else {
            println!("Docker image already exists.");
        }
    });
}

#[test]
#[ignore = "requires Docker and should be run with --ignored"]
fn test_full_agent_lifecycle() {
    ensure_docker_image_built();

    // This test requires Docker to be running
    let test_agent_name = format!(
        "test-agent-{}",
        uuid::Uuid::new_v4().to_string()[0..8].to_string()
    );

    // Get the absolute path to the fixture repo
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixture_path = PathBuf::from(manifest_dir).join("tests/fixtures/nodejs-test-repo");

    // Ensure fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture not found at {:?}",
        fixture_path
    );

    // Clean up any existing test agent
    cleanup_agent(&test_agent_name);

    // Initialize the fixture as a git repo if it isn't already
    if !fixture_path.join(".git").exists() {
        println!("Initializing fixture as git repo...");
        StdCommand::new("git")
            .current_dir(&fixture_path)
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        StdCommand::new("git")
            .current_dir(&fixture_path)
            .args(&["add", "."])
            .status()
            .expect("Failed to add files");

        StdCommand::new("git")
            .current_dir(&fixture_path)
            .args(&["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit");
    }

    // Format as a file:// URL for local git repo
    let repo_url = format!("file://{}", fixture_path.display());

    // Test 1: Create command should clone the fixture repo
    println!("Testing new command...");
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("new")
        .arg(&test_agent_name)
        .arg(&repo_url)
        .timeout(Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("setup complete!"));

    // Test 2: List command should show the new agent
    println!("Testing list command...");
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("list")
        .arg("--all")
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains(&test_agent_name))
        .stdout(predicate::str::contains("Created"));

    // Test 3: Start command should start the container
    println!("Testing start command...");
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("start")
        .arg(&test_agent_name)
        .timeout(Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("started successfully"));

    // Give container time to fully initialize
    std::thread::sleep(Duration::from_secs(20));

    // Test 4: List should show agent as running
    println!("Verifying agent is running...");
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("list")
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains(&test_agent_name))
        .stdout(predicate::str::contains("Running"));

    // Test 5: Connect command with custom command
    println!("Testing connect command...");
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("connect")
        .arg(&test_agent_name)
        .arg("--command")
        .arg("echo 'Hello from crowdcontrol!'")
        .arg("--detach")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();

    // Test 6: Logs command should show container output
    println!("Testing logs command...");
    // First, just verify logs command works
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("logs")
        .arg(&test_agent_name)
        .arg("--tail")
        .arg("10")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();

    // Test 7: Stop command should stop the container
    println!("Testing stop command...");
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("stop")
        .arg(&test_agent_name)
        .timeout(Duration::from_secs(10)) // Quick stop for dev containers
        .assert()
        .success()
        .stdout(predicate::str::contains("stopped successfully"));

    // Test 8: List should show agent as stopped
    println!("Verifying agent is stopped...");
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("list")
        .arg("--all")
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains(&test_agent_name))
        .stdout(predicate::str::contains("Stopped"));

    // Clean up
    cleanup_agent(&test_agent_name);

    println!("✅ All integration tests passed!");
}

#[test]
#[ignore = "requires Docker and should be run with --ignored"]
fn test_multiple_agents() {
    ensure_docker_image_built();

    let agent1 = format!(
        "test-multi-1-{}",
        uuid::Uuid::new_v4().to_string()[0..8].to_string()
    );
    let agent2 = format!(
        "test-multi-2-{}",
        uuid::Uuid::new_v4().to_string()[0..8].to_string()
    );

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixture_path = PathBuf::from(manifest_dir).join("tests/fixtures/nodejs-test-repo");
    let repo_url = format!("file://{}", fixture_path.display());

    // Clean up
    cleanup_agent(&agent1);
    cleanup_agent(&agent2);

    // Create both agents
    for agent in [&agent1, &agent2] {
        let mut cmd = crowdcontrol_cmd();
        cmd.arg("new")
            .arg(agent)
            .arg(&repo_url)
            .timeout(Duration::from_secs(30))
            .assert()
            .success();
    }

    // Start both agents
    for agent in [&agent1, &agent2] {
        let mut cmd = crowdcontrol_cmd();
        cmd.arg("start")
            .arg(agent)
            .timeout(Duration::from_secs(60))
            .assert()
            .success();
    }

    // Verify both are running - check that our specific agents are running
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("list")
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains(&agent1))
        .stdout(predicate::str::contains(&agent2));

    // Stop all agents
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("stop")
        .arg("--all")
        .timeout(Duration::from_secs(15)) // Quick stop for multiple dev containers
        .assert()
        .success()
        .stdout(predicate::str::contains("agent(s)"));

    // Clean up
    cleanup_agent(&agent1);
    cleanup_agent(&agent2);
}

#[test]
#[ignore = "requires Docker and should be run with --ignored"]
fn test_resource_limits() {
    ensure_docker_image_built();

    let test_agent = format!(
        "test-limits-{}",
        uuid::Uuid::new_v4().to_string()[0..8].to_string()
    );

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixture_path = PathBuf::from(manifest_dir).join("tests/fixtures/nodejs-test-repo");
    let repo_url = format!("file://{}", fixture_path.display());

    cleanup_agent(&test_agent);

    // Create with resource limits
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("new")
        .arg(&test_agent)
        .arg(&repo_url)
        .arg("--memory")
        .arg("512m")
        .arg("--cpus")
        .arg("0.5")
        .timeout(Duration::from_secs(30))
        .assert()
        .success();

    cleanup_agent(&test_agent);
}

#[test]
#[ignore = "requires Docker and should be run with --ignored"]
fn test_agent_start_and_connect_issue() {
    ensure_docker_image_built();
    
    // This test reproduces the issue where agent starts but shows as not running
    let test_agent_name = format!(
        "test-issue-{}",
        uuid::Uuid::new_v4().to_string()[0..8].to_string()
    );
    
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixture_path = PathBuf::from(manifest_dir).join("tests/fixtures/nodejs-test-repo");
    
    // Ensure fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture not found at {:?}",
        fixture_path
    );
    
    // Clean up any existing test agent
    cleanup_agent(&test_agent_name);
    
    let repo_url = format!("file://{}", fixture_path.display());
    
    // Create agent
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("new")
        .arg(&test_agent_name)
        .arg(&repo_url)
        .timeout(Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("setup complete!"));
    
    // Start agent
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("start")
        .arg(&test_agent_name)
        .timeout(Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("started successfully"));
    
    // Give container time to fail if it's going to
    std::thread::sleep(Duration::from_secs(5));
    
    // Check container logs to debug any startup issues
    let output = StdCommand::new("docker")
        .args(&["logs", &format!("crowdcontrol-{}", test_agent_name)])
        .output()
        .expect("Failed to get container logs");
    
    if !output.status.success() || !String::from_utf8_lossy(&output.stderr).is_empty() {
        println!("Container logs (stdout):\n{}", String::from_utf8_lossy(&output.stdout));
        println!("Container logs (stderr):\n{}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Try to connect - this should work if the container is actually running
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("connect")
        .arg(&test_agent_name)
        .arg("--command")
        .arg("echo 'test'")
        .arg("--detach")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();
    
    // Clean up
    cleanup_agent(&test_agent_name);
}

#[test]
fn test_json_output_format() {
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("list")
        .arg("--all")
        .arg("--format")
        .arg("json")
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["))
        .stdout(predicate::str::ends_with("]\n"));
}

#[test]
#[ignore = "requires Docker and should be run with --ignored"]
fn test_refresh_claude_credentials() {
    ensure_docker_image_built();
    
    let test_agent_name = format!(
        "test-refresh-{}",
        uuid::Uuid::new_v4().to_string()[0..8].to_string()
    );
    
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixture_path = PathBuf::from(manifest_dir).join("tests/fixtures/nodejs-test-repo");
    
    // Clean up any existing test agent
    cleanup_agent(&test_agent_name);
    
    let repo_url = format!("file://{}", fixture_path.display());
    
    // Create agent
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("new")
        .arg(&test_agent_name)
        .arg(&repo_url)
        .timeout(Duration::from_secs(30))
        .assert()
        .success();
    
    // Start agent
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("start")
        .arg(&test_agent_name)
        .timeout(Duration::from_secs(60))
        .assert()
        .success();
    
    // Give container time to initialize
    std::thread::sleep(Duration::from_secs(5));
    
    // Test refresh command when Claude credentials don't exist
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("refresh")
        .arg(&test_agent_name)
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains("authentication refreshed successfully"));
    
    // Create a mock Claude credentials file
    let home_dir = dirs::home_dir().unwrap();
    let claude_dir = home_dir.join(".claude");
    let credentials_backup = claude_dir.join("credentials.json.backup");
    let credentials_path = claude_dir.join("credentials.json");
    
    // Backup existing credentials if they exist
    let had_credentials = if credentials_path.exists() {
        fs::rename(&credentials_path, &credentials_backup).ok();
        true
    } else {
        fs::create_dir_all(&claude_dir).ok();
        false
    };
    
    // Create mock credentials
    fs::write(&credentials_path, r#"{"token": "test-token-12345"}"#)
        .expect("Failed to create mock credentials");
    
    // Test refresh with credentials present
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("refresh")
        .arg(&test_agent_name)
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains("authentication refreshed successfully"));
    
    // Verify credentials were copied into container
    let output = StdCommand::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", test_agent_name),
            "cat",
            "/home/developer/.claude/credentials.json"
        ])
        .output()
        .expect("Failed to check credentials in container");
    
    let container_creds = String::from_utf8_lossy(&output.stdout);
    assert!(container_creds.contains("test-token-12345"), "Credentials not found in container");
    
    // Clean up mock credentials
    fs::remove_file(&credentials_path).ok();
    
    // Restore original credentials if they existed
    if had_credentials {
        fs::rename(&credentials_backup, &credentials_path).ok();
    }
    
    // Clean up
    cleanup_agent(&test_agent_name);
}

#[test]
#[ignore = "requires Docker and should be run with --ignored"]
fn test_refresh_command_requires_running_agent() {
    ensure_docker_image_built();
    
    let test_agent_name = format!(
        "test-refresh-stopped-{}",
        uuid::Uuid::new_v4().to_string()[0..8].to_string()
    );
    
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixture_path = PathBuf::from(manifest_dir).join("tests/fixtures/nodejs-test-repo");
    let repo_url = format!("file://{}", fixture_path.display());
    
    // Clean up any existing test agent
    cleanup_agent(&test_agent_name);
    
    // Create agent but don't start it
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("new")
        .arg(&test_agent_name)
        .arg(&repo_url)
        .timeout(Duration::from_secs(30))
        .assert()
        .success();
    
    // Try to refresh - should fail
    let mut cmd = crowdcontrol_cmd();
    cmd.arg("refresh")
        .arg(&test_agent_name)
        .timeout(Duration::from_secs(10))
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be running"));
    
    // Clean up
    cleanup_agent(&test_agent_name);
}

fn cleanup_agent(name: &str) {
    // Try to remove the agent if it exists
    let mut cmd = crowdcontrol_cmd();
    let _ = cmd
        .arg("remove")
        .arg(name)
        .arg("--force")
        .timeout(Duration::from_secs(30))
        .output();
}
