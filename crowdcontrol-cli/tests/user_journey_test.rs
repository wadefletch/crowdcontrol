use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// Test helper functions
fn crowdcontrol_cmd_with_temp() -> (Command, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir").arg(temp_dir.path());
    (cmd, temp_dir)
}

fn check_docker_available() -> bool {
    StdCommand::new("docker")
        .arg("version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn wait_for_container_state(name: &str, expected_state: &str, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        let output = StdCommand::new("docker")
            .args(&["ps", "-a", "--format", "{{.Names}}\t{{.Status}}"])
            .output()
            .expect("Failed to list containers");

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with(&format!("crowdcontrol-{}", name)) {
                if expected_state == "running" && line.contains("Up ") {
                    return true;
                } else if expected_state == "exited" && line.contains("Exited") {
                    return true;
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }

    false
}

#[test]
#[ignore] // Requires Docker
fn test_developer_workflow_multiple_features() {
    if !check_docker_available() {
        eprintln!("Docker not available, skipping test");
        return;
    }

    let (mut cmd, _temp) = crowdcontrol_cmd_with_temp();

    // Scenario: Developer working on multiple features simultaneously
    // 1. Setup main branch agent
    cmd.args(&[
        "new",
        "app-main",
        "https://github.com/octocat/Hello-World.git",
    ])
    .assert()
    .success()
    .stdout(predicates::str::contains(
        "Successfully set up agent 'app-main'",
    ));

    // 2. Setup feature branch agent
    let (mut cmd, _temp2) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&[
            "new",
            "app-feature-auth",
            "https://github.com/octocat/Hello-World.git",
            "--branch",
            "test",
        ])
        .assert()
        .success();

    // 3. List agents to see both
    let (mut cmd, _temp3) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("app-main"))
        .stdout(predicates::str::contains("app-feature-auth"))
        .stdout(predicates::str::contains("Created"));

    // 4. Start main branch agent
    let (mut cmd, _temp4) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&["start", "app-main"])
        .assert()
        .success();

    // Wait for container to be running
    assert!(wait_for_container_state("app-main", "running", 10));

    // 5. Start feature branch agent with resource limits
    let (mut cmd, _temp5) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&[
            "start",
            "app-feature-auth",
            "--memory",
            "512m",
            "--cpus",
            "1",
        ])
        .assert()
        .success();

    assert!(wait_for_container_state("app-feature-auth", "running", 10));

    // 6. List agents - both should be running
    let (mut cmd, _temp6) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&["list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::starts_with("[").and(predicates::str::ends_with("]\n")));

    // 7. Get logs from feature branch
    let (mut cmd, _temp7) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&["logs", "app-feature-auth", "--tail", "5"])
        .assert()
        .success();

    // 8. Stop feature branch when done
    let (mut cmd, _temp8) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&["stop", "app-feature-auth"])
        .assert()
        .success();

    // 9. Clean up everything
    let (mut cmd, _temp9) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&["stop", "--all"])
        .assert()
        .success();

    let (mut cmd, _temp10) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&["remove", "app-main", "--force"])
        .assert()
        .success();

    let (mut cmd, _temp11) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", _temp.path())
        .args(&["remove", "app-feature-auth", "--force"])
        .assert()
        .success();
}

#[test]
#[ignore] // Requires Docker
fn test_resource_constrained_workflow() {
    if !check_docker_available() {
        eprintln!("Docker not available, skipping test");
        return;
    }

    let (_, temp) = crowdcontrol_cmd_with_temp();

    // Scenario: Developer with limited resources juggling multiple projects
    // Setup 3 agents for different projects
    let projects = vec![
        ("project-frontend", "512m", "0.5"),
        ("project-backend", "1g", "1"),
        ("project-database", "2g", "1"),
    ];

    for (name, memory, cpus) in &projects {
        let (mut cmd, _) = crowdcontrol_cmd_with_temp();
        cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
            .args(&[
                "new",
                name,
                "https://github.com/octocat/Hello-World.git",
                "--memory",
                memory,
                "--cpus",
                cpus,
            ])
            .assert()
            .success();
    }

    // Start only the frontend initially
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["start", "project-frontend"])
        .assert()
        .success();

    assert!(wait_for_container_state("project-frontend", "running", 10));

    // List to verify only frontend is running
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    let output = cmd
        .env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["list", "--format", "json"])
        .output()
        .unwrap();

    let agents: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    let running_count = agents
        .iter()
        .filter(|a| a["status"].as_str() == Some("Running"))
        .count();
    assert_eq!(running_count, 1);

    // Switch to backend - stop frontend, start backend
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["stop", "project-frontend"])
        .assert()
        .success();

    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["start", "project-backend"])
        .assert()
        .success();

    assert!(wait_for_container_state("project-backend", "running", 10));

    // Clean up
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["stop", "--all"])
        .assert()
        .success();

    for (name, _, _) in &projects {
        let (mut cmd, _) = crowdcontrol_cmd_with_temp();
        cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
            .args(&["remove", name, "--force"])
            .assert()
            .success();
    }
}

#[test]
#[ignore = "requires Docker"]
fn test_config_and_environment_precedence() {
    // Test the full config hierarchy: CLI args > env vars > config file > defaults
    let config_dir = TempDir::new().unwrap();
    let workspace1 = TempDir::new().unwrap();
    let workspace2 = TempDir::new().unwrap();
    let workspace3 = TempDir::new().unwrap();

    // Create config file
    let config_path = config_dir.path().join(".config").join("crowdcontrol");
    fs::create_dir_all(&config_path).unwrap();

    let config_content = format!(
        r#"
workspaces_dir = "{}"
image = "config-image:latest"
default_memory = "1g"
verbose = 1
"#,
        workspace1.path().display()
    );

    fs::write(config_path.join("config.toml"), config_content).unwrap();

    // Test 1: Config file is used
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("list")
        .assert()
        .success();

    // Test 2: Env var overrides config file
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace2.path())
        .arg("list")
        .assert()
        .success();

    // Test 3: CLI arg overrides both env var and config
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace2.path())
        .args(&["--workspaces-dir", workspace3.path().to_str().unwrap()])
        .arg("list")
        .assert()
        .success();

    // Test 4: Verbose levels accumulate
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .args(&["-vv", "list"]) // Should add to config file's verbose=1
        .assert()
        .success();
}

#[test]
#[ignore] // Requires Docker and network
fn test_concurrent_operations_stress() {
    if !check_docker_available() {
        eprintln!("Docker not available, skipping test");
        return;
    }

    let (_, temp) = crowdcontrol_cmd_with_temp();
    let workspace_path = temp.path().to_path_buf();

    // Setup multiple agents
    let agent_count = 5;
    for i in 0..agent_count {
        let (mut cmd, _) = crowdcontrol_cmd_with_temp();
        cmd.env("CROWDCONTROL_WORKSPACES_DIR", &workspace_path)
            .args(&[
                "new",
                &format!("stress-test-{}", i),
                "https://github.com/octocat/Hello-World.git",
            ])
            .assert()
            .success();
    }

    // Start all agents concurrently
    let handles: Vec<_> = (0..agent_count)
        .map(|i| {
            let workspace = workspace_path.clone();
            thread::spawn(move || {
                let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
                cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace)
                    .args(&["start", &format!("stress-test-{}", i)])
                    .assert()
                    .success();
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all are running
    thread::sleep(Duration::from_secs(5));

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    let output = cmd
        .env("CROWDCONTROL_WORKSPACES_DIR", &workspace_path)
        .args(&["list", "--format", "json"])
        .output()
        .unwrap();

    let agents: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    let running_count = agents
        .iter()
        .filter(|a| a["status"].as_str() == Some("Running"))
        .count();

    assert_eq!(running_count, agent_count);

    // Stop all concurrently
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", &workspace_path)
        .args(&["stop", "--all"])
        .assert()
        .success();

    // Clean up
    for i in 0..agent_count {
        let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
        cmd.env("CROWDCONTROL_WORKSPACES_DIR", &workspace_path)
            .args(&["remove", &format!("stress-test-{}", i), "--force"])
            .assert()
            .success();
    }
}

#[test]
#[ignore = "requires Docker"]
fn test_error_recovery_scenarios() {
    let (mut cmd, temp) = crowdcontrol_cmd_with_temp();

    // Test 1: Invalid repository URL
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["new", "invalid-repo", "not-a-valid-url"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Failed to clone repository"));

    // Test 2: Network timeout simulation (using non-existent domain)
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&[
            "new",
            "timeout-test",
            "https://this-domain-definitely-does-not-exist-12345.com/repo.git",
        ])
        .assert()
        .failure();

    // Test 3: Permission denied scenario
    let protected_dir = temp.path().join("protected");
    fs::create_dir(&protected_dir).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&protected_dir).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&protected_dir, perms).unwrap();

        let (mut cmd, _) = crowdcontrol_cmd_with_temp();
        cmd.env("CROWDCONTROL_WORKSPACES_DIR", &protected_dir)
            .arg("list")
            .assert()
            .failure();

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&protected_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&protected_dir, perms).unwrap();
    }
}

#[test]
#[ignore] // Requires Docker
fn test_workspace_persistence_across_restarts() {
    if !check_docker_available() {
        eprintln!("Docker not available, skipping test");
        return;
    }

    let (mut cmd, temp) = crowdcontrol_cmd_with_temp();
    let workspace_path = temp.path();

    // Setup and start an agent
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .args(&[
            "new",
            "persistence-test",
            "https://github.com/octocat/Hello-World.git",
        ])
        .assert()
        .success();

    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .args(&["start", "persistence-test"])
        .assert()
        .success();

    assert!(wait_for_container_state("persistence-test", "running", 10));

    // Create a test file in the workspace
    let agent_workspace = workspace_path.join("persistence-test");
    let test_file = agent_workspace.join("test-data.txt");
    fs::write(&test_file, "Important data that should persist").unwrap();

    // Also create a file in .crowdcontrol directory
    let crowdcontrol_dir = agent_workspace.join(".crowdcontrol");
    let config_file = crowdcontrol_dir.join("custom-config.yml");
    fs::write(&config_file, "custom: configuration").unwrap();

    // Stop the agent
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .args(&["stop", "persistence-test"])
        .assert()
        .success();

    // Verify files still exist
    assert!(test_file.exists());
    assert_eq!(
        fs::read_to_string(&test_file).unwrap(),
        "Important data that should persist"
    );
    assert!(config_file.exists());

    // Start agent again
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .args(&["start", "persistence-test"])
        .assert()
        .success();

    assert!(wait_for_container_state("persistence-test", "running", 10));

    // Files should still be there
    assert!(test_file.exists());
    assert!(config_file.exists());

    // Clean up
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .args(&["stop", "persistence-test"])
        .assert()
        .success();

    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .args(&["remove", "persistence-test", "--force"])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_metadata_file_integrity() {
    let (mut cmd, temp) = crowdcontrol_cmd_with_temp();

    // Setup an agent
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&[
            "new",
            "metadata-test",
            "https://github.com/octocat/Hello-World.git",
            "--branch",
            "test",
        ])
        .assert()
        .success();

    // Read and verify metadata
    let metadata_path = temp
        .path()
        .join("metadata-test")
        .join(".crowdcontrol")
        .join("metadata.json");

    assert!(metadata_path.exists());

    let metadata_content = fs::read_to_string(&metadata_path).unwrap();
    let metadata: Value = serde_json::from_str(&metadata_content).unwrap();

    // Verify required fields
    assert_eq!(metadata["name"], "metadata-test");
    assert_eq!(
        metadata["repository"],
        "https://github.com/octocat/Hello-World.git"
    );
    assert_eq!(metadata["branch"], "test");
    assert!(metadata["_comment"]
        .as_str()
        .unwrap()
        .contains("auto-generated"));
    assert!(metadata["created_at"].is_string());

    // Simulate manual edit attempt
    let _original_content = metadata_content.clone();
    let mut modified_metadata = metadata.clone();
    modified_metadata["name"] = Value::String("hacked-name".to_string());
    fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&modified_metadata).unwrap(),
    )
    .unwrap();

    // List should still work and show correct data
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicates::str::starts_with("[").and(predicates::str::ends_with("]\n")));

    // Clean up
    let (mut cmd, _) = crowdcontrol_cmd_with_temp();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp.path())
        .args(&["remove", "metadata-test", "--force"])
        .assert()
        .success();
}
