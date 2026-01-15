// Integration test for the full agent lifecycle with project:label format
// This test captures the bug where:
// - `crowdcontrol new myproject:label` succeeds
// - `crowdcontrol start myproject:label` fails with "not found"
//
// Run with: cargo test --package crowdcontrol-cli --test project_agent_lifecycle_test -- --ignored

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Set up test environment with a registered project
fn setup_test_env() -> (TempDir, TempDir) {
    let temp_workspace = TempDir::new().expect("Failed to create temp workspace");
    let temp_home = TempDir::new().expect("Failed to create temp home");

    // Create .crowdcontrol directory in temp home
    let crowdcontrol_dir = temp_home.path().join(".crowdcontrol");
    fs::create_dir_all(&crowdcontrol_dir).expect("Failed to create .crowdcontrol dir");

    // Register a test project
    fs::write(
        crowdcontrol_dir.join("projects.toml"),
        r#"[projects.testproj]
url = "https://github.com/octocat/Hello-World.git"
"#,
    )
    .expect("Failed to write projects.toml");

    (temp_workspace, temp_home)
}

/// Test that `new` with project:label format creates the correct nested structure
#[test]
fn test_new_creates_nested_structure() {
    let (temp_workspace, temp_home) = setup_test_env();

    // Create agent with project:label format
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp_workspace.path())
        .env("HOME", temp_home.path())
        .env("CROWDCONTROL_IMAGE", "nonexistent:image") // Will fail at container creation
        .arg("new")
        .arg("testproj:myagent")
        .arg("--skip-verification");

    // Command will fail at Docker step, but we can check the structure was created
    let _ = cmd.output();

    // Verify nested structure was attempted (even though it gets cleaned up on failure)
    // The key test is that it doesn't create "testproj:myagent" as a flat directory
    let flat_path = temp_workspace.path().join("testproj:myagent");
    assert!(
        !flat_path.exists(),
        "Should NOT create flat directory with colon in name"
    );
}

/// Test the full lifecycle: new → start → list → stop → remove
/// This test requires Docker to be running with the crowdcontrol:latest image
#[test]
#[ignore] // Requires Docker
fn test_full_lifecycle_with_project_label() {
    let (temp_workspace, temp_home) = setup_test_env();
    let workspace_path = temp_workspace.path();
    let home_path = temp_home.path();

    // 1. Create agent with project:label format
    let mut cmd_new = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_new
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("new")
        .arg("testproj:dev")
        .arg("--skip-verification");

    cmd_new
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent 'testproj:dev' setup complete"));

    // Verify nested directory structure
    let nested_path = workspace_path.join("testproj").join("dev");
    assert!(nested_path.exists(), "Should create nested directory structure");
    assert!(
        nested_path.join("metadata.json").exists(),
        "Should create metadata.json in nested path"
    );

    // 2. Start agent using project:label format (colon)
    let mut cmd_start = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_start
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("start")
        .arg("testproj:dev"); // Using colon format

    cmd_start
        .assert()
        .success()
        .stdout(predicate::str::contains("started successfully"));

    // 3. List agents - should show testproj:dev
    let mut cmd_list = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_list
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("list");

    cmd_list
        .assert()
        .success()
        .stdout(predicate::str::contains("testproj:dev"));

    // 4. Stop agent using hyphen format (should also work)
    let mut cmd_stop = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_stop
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("stop")
        .arg("testproj-dev"); // Using hyphen format

    cmd_stop.assert().success();

    // 5. Remove agent using colon format
    let mut cmd_remove = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_remove
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("remove")
        .arg("testproj:dev")
        .arg("--force");

    cmd_remove.assert().success();

    // Verify cleanup
    assert!(
        !nested_path.exists(),
        "Nested directory should be removed after agent removal"
    );
}

/// Test that start works with both colon and hyphen formats
#[test]
#[ignore] // Requires Docker
fn test_start_accepts_both_formats() {
    let (temp_workspace, temp_home) = setup_test_env();
    let workspace_path = temp_workspace.path();
    let home_path = temp_home.path();

    // Create agent
    let mut cmd_new = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_new
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("new")
        .arg("testproj:feature")
        .arg("--skip-verification");
    cmd_new.assert().success();

    // Start with colon format
    let mut cmd_start_colon = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_start_colon
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("start")
        .arg("testproj:feature");
    cmd_start_colon.assert().success();

    // Stop it
    let mut cmd_stop = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_stop
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("stop")
        .arg("testproj:feature");
    cmd_stop.assert().success();

    // Start with hyphen format (should also work)
    let mut cmd_start_hyphen = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_start_hyphen
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("start")
        .arg("testproj-feature"); // Hyphen format
    cmd_start_hyphen.assert().success();

    // Cleanup
    let mut cmd_remove = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_remove
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("stop")
        .arg("--all");
    let _ = cmd_remove.output();

    let mut cmd_remove = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_remove
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", home_path)
        .arg("remove")
        .arg("testproj:feature")
        .arg("--force");
    let _ = cmd_remove.output();
}

/// Test that verifies the specific bug scenario:
/// 1. new carlyle:test succeeds
/// 2. start carlyle:test fails with "not found"
///
/// This is a unit test that doesn't require Docker - it tests path resolution
#[test]
fn test_path_resolution_colon_format() {
    let (temp_workspace, temp_home) = setup_test_env();
    let workspace_path = temp_workspace.path();

    // Simulate what `new` does: create nested structure with metadata
    let nested_path = workspace_path.join("carlyle").join("test");
    fs::create_dir_all(&nested_path).expect("Failed to create nested dir");

    // Write metadata like `new` would
    let metadata = r#"{
        "_comment": "This file is auto-generated by CrowdControl.",
        "name": "carlyle-test",
        "repository": "git@github.com:org/carlyle.git",
        "branch": null,
        "created_at": "2024-01-01T00:00:00Z",
        "container_id": "abc123",
        "project_slug": "carlyle"
    }"#;
    fs::write(nested_path.join("metadata.json"), metadata).expect("Failed to write metadata");

    // Now try to start with colon format - this should find the agent
    let mut cmd_start = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_start
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", temp_home.path())
        .arg("start")
        .arg("carlyle:test");

    // Should NOT fail with "Agent not found" - it should fail for a different reason
    // (like Docker connection or container not existing)
    // Note: Docker errors may contain "not found" (e.g., "socket not found"), so we
    // specifically check for the agent not found error message
    cmd_start
        .assert()
        .failure()
        .stderr(predicate::str::contains("Agent 'carlyle:test' not found").not());
}

/// Test that standalone agents (no project) still work
#[test]
fn test_standalone_agent_path_resolution() {
    let temp_workspace = TempDir::new().expect("Failed to create temp workspace");
    let temp_home = TempDir::new().expect("Failed to create temp home");
    let workspace_path = temp_workspace.path();

    // Create standalone agent directory with metadata
    let standalone_path = workspace_path.join("my-standalone-agent");
    fs::create_dir_all(&standalone_path).expect("Failed to create standalone dir");

    let metadata = r#"{
        "_comment": "This file is auto-generated by CrowdControl.",
        "name": "my-standalone-agent",
        "repository": "git@github.com:org/repo.git",
        "branch": null,
        "created_at": "2024-01-01T00:00:00Z",
        "container_id": "def456",
        "project_slug": null
    }"#;
    fs::write(standalone_path.join("metadata.json"), metadata).expect("Failed to write metadata");

    // Should be able to find standalone agent
    let mut cmd_start = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_start
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", temp_home.path())
        .arg("start")
        .arg("my-standalone-agent");

    // Should NOT fail with "Agent not found" - may fail for Docker reasons
    cmd_start
        .assert()
        .failure()
        .stderr(predicate::str::contains("Agent 'my-standalone-agent' not found").not());
}
