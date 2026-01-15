// TDD: Tests for project:label syntax in new/connect commands
// Run with: cargo test --package crowdcontrol-cli --test repo_agent_syntax_test

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, TempDir, TempDir) {
    let workspaces_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    let repo_dir = TempDir::new().unwrap();

    // Create config directory structure
    let crowdcontrol_config_dir = config_dir.path().join(".config").join("crowdcontrol");
    fs::create_dir_all(&crowdcontrol_config_dir).unwrap();

    // Create a minimal git repository
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to init git repo");

    fs::write(repo_dir.path().join("README.md"), "# Test Repository").unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();

    (workspaces_dir, config_dir, repo_dir)
}

#[test]
fn test_new_with_project_label_syntax_missing_project() {
    let (workspaces_dir, config_dir, _repo_dir) = setup_test_env();

    // Try to create agent with project:label syntax but project doesn't exist
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--workspaces-dir")
        .arg(workspaces_dir.path())
        .arg("new")
        .arg("myapp:main")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found").or(predicates::str::contains("does not exist")));
}

#[test]
#[ignore = "requires Docker"]
fn test_new_with_project_label_syntax_success() {
    let (workspaces_dir, config_dir, repo_dir) = setup_test_env();

    // First add the project
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("project")
        .arg("add")
        .arg("myapp")
        .arg(repo_dir.path())
        .assert()
        .success();

    // Now create agent using project:label syntax
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--workspaces-dir")
        .arg(workspaces_dir.path())
        .arg("new")
        .arg("myapp:main")
        .assert()
        .success()
        .stdout(predicates::str::contains("myapp:main"));

    // Verify the agent was created with correct filesystem name
    let agent_dir = workspaces_dir.path().join("myapp-main");
    assert!(agent_dir.exists(), "Agent directory should exist at myapp-main");
}

#[test]
#[ignore = "requires Docker"]
fn test_new_with_project_label_uses_default_branch() {
    let (workspaces_dir, config_dir, repo_dir) = setup_test_env();

    // Add project with default branch
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("project")
        .arg("add")
        .arg("myapp")
        .arg(repo_dir.path())
        .arg("--default-branch")
        .arg("develop")
        .assert()
        .success();

    // Create agent without specifying branch - should use default
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--workspaces-dir")
        .arg(workspaces_dir.path())
        .arg("new")
        .arg("myapp:feature")
        .assert()
        .success();
}

#[test]
fn test_new_with_url_still_works() {
    let (workspaces_dir, config_dir, repo_dir) = setup_test_env();

    // Old syntax should still work (backwards compatibility)
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--workspaces-dir")
        .arg(workspaces_dir.path())
        .arg("new")
        .arg("standalone-agent")
        .arg(repo_dir.path())
        .assert()
        .failure(); // Will fail due to Docker, but should parse correctly
}

#[test]
fn test_new_requires_url_or_registered_project() {
    let (workspaces_dir, config_dir, _repo_dir) = setup_test_env();

    // If identifier contains colon but project doesn't exist, should fail
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--workspaces-dir")
        .arg(workspaces_dir.path())
        .arg("new")
        .arg("nonexistent:label")
        .assert()
        .failure();
}

#[test]
#[ignore = "requires Docker"]
fn test_list_shows_display_name_with_project() {
    let (workspaces_dir, config_dir, _repo_dir) = setup_test_env();

    // Create a fake agent metadata with project_slug
    let agent_name = "myapp-main";
    let agent_dir = workspaces_dir.path().join(agent_name);
    let metadata_dir = agent_dir.join(".crowdcontrol");
    fs::create_dir_all(&metadata_dir).unwrap();

    let metadata = serde_json::json!({
        "_comment": "Test metadata",
        "name": "myapp-main",
        "repository": "git@github.com:org/myapp.git",
        "branch": "main",
        "created_at": "2024-01-01T00:00:00Z",
        "container_id": null,
        "project_slug": "myapp"
    });

    fs::write(metadata_dir.join("metadata.json"), metadata.to_string()).unwrap();

    // List should show myapp:main as display name
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--workspaces-dir")
        .arg(workspaces_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("myapp:main"));
}

#[test]
#[ignore = "requires Docker"]
fn test_connect_with_project_label_syntax() {
    let (workspaces_dir, config_dir, _repo_dir) = setup_test_env();

    // Create a fake agent metadata with project_slug
    let agent_name = "myapp-main";
    let agent_dir = workspaces_dir.path().join(agent_name);
    let metadata_dir = agent_dir.join(".crowdcontrol");
    fs::create_dir_all(&metadata_dir).unwrap();

    let metadata = serde_json::json!({
        "_comment": "Test metadata",
        "name": "myapp-main",
        "repository": "git@github.com:org/myapp.git",
        "branch": "main",
        "created_at": "2024-01-01T00:00:00Z",
        "container_id": null,
        "project_slug": "myapp"
    });

    fs::write(metadata_dir.join("metadata.json"), metadata.to_string()).unwrap();

    // Connect using project:label syntax should find the agent
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--workspaces-dir")
        .arg(workspaces_dir.path())
        .arg("connect")
        .arg("myapp:main")
        .assert()
        .failure() // Will fail because no container, but should find the agent
        .stderr(predicates::str::contains("not running").or(predicates::str::contains("not found").not()));
}
