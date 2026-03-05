use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod test_utils;
use test_utils::{generate_test_id, create_config_file, create_config_with_limits};

// Helper to create and start an agent
fn create_and_start_agent(name: &str, config_path: &std::path::Path, repo_path: &std::path::Path, extra_env: Vec<(&str, &str)>) {
    // Create the agent
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    for (key, val) in &extra_env {
        cmd.env(key, val);
    }
    cmd.env("CC_CONFIG_FILE", config_path)
        .args(&[
            "new",
            name,
            &format!("file://{}", repo_path.display()),
            "--skip-verification",
        ])
        .assert()
        .success();
    
    // Start the agent
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    for (key, val) in &extra_env {
        cmd.env(key, val);
    }
    cmd.env("CC_CONFIG_FILE", config_path)
        .args(&["start", name])
        .assert()
        .success()
        .stdout(predicate::str::contains(name).and(predicate::str::contains("started")));
}

#[test]
#[ignore = "requires Docker"]
fn test_start_with_claude_credentials() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create mock .claude directory in HOME
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("credentials.json"),
        r#"{"token": "test-token"}"#,
    )
    .unwrap();
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("README.md"), "# Test Repo").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Create and start an agent with Claude config available
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("claude-test-{}", timestamp);
    create_and_start_agent(&agent_name, &config_path, &test_repo, vec![("HOME", temp_dir.path().to_str().unwrap())]);
    
    // Clean up - stop the agent
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_start_without_claude_credentials() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("README.md"), "# Test Repo").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Create and start an agent without Claude config
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("no-claude-test-{}", timestamp);
    create_and_start_agent(&agent_name, &config_path, &test_repo, vec![("HOME", temp_dir.path().to_str().unwrap())]);
    
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
fn test_start_with_resource_limits() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("README.md"), "# Test Repo").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Create agent with resource limits
    let agent_name = generate_test_id("limited-agent");
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            &agent_name,
            &format!("file://{}", test_repo.display()),
            "--skip-verification",
            "--memory",
            "512m",
            "--cpus",
            "1.0",
        ])
        .assert()
        .success();
    
    // Start the agent
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success()
        .stdout(predicate::str::contains(&agent_name).and(predicate::str::contains("started")));
    
    // Verify the agent is listed
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&agent_name));
    
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
fn test_list_containers_with_label() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("README.md"), "# Test Repo").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Start multiple agents
    for i in 1..=3 {
        Command::cargo_bin("crowdcontrol")
            .unwrap()
            .env("CC_CONFIG_FILE", &config_path)
            .args(&[
                "new",
                &format!("label-test-{}", i),
                &format!("file://{}", test_repo.display()),
                "--skip-verification",
            ])
            .assert()
            .success();
            
        Command::cargo_bin("crowdcontrol")
            .unwrap()
            .env("CC_CONFIG_FILE", &config_path)
            .args(&["start", &format!("label-test-{}", i)])
            .assert()
            .success();
    }
    
    // List should show all agents
    let output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["list"])
        .assert()
        .success();
    
    output
        .stdout(predicate::str::contains("label-test-1"))
        .stdout(predicate::str::contains("label-test-2"))
        .stdout(predicate::str::contains("label-test-3"));
    
    // Clean up all agents
    for i in 1..=3 {
        Command::cargo_bin("crowdcontrol")
            .unwrap()
            .env("CC_CONFIG_FILE", &config_path)
            .args(&["stop", &format!("label-test-{}", i)])
            .assert()
            .success();
    }
}

#[test]
#[ignore = "requires Docker"]
fn test_resource_limits_from_config() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create config with default resource limits
    let config_path = create_config_with_limits(&temp_dir, "1g", "2");
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("README.md"), "# Test Repo").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Create agent without specifying limits (should use config defaults)
    let agent_name = generate_test_id("config-limits");
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", config_path.clone())
        .args(&[
            "new",
            &agent_name,
            &format!("file://{}", test_repo.display()),
            "--skip-verification",
        ])
        .assert()
        .success();
        
    // Start the agent
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", config_path.clone())
        .args(&["start", &agent_name])
        .assert()
        .success()
        .stdout(predicate::str::contains(&agent_name).and(predicate::str::contains("started")));
    
    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}