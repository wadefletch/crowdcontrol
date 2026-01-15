use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Test that captures the scenario where:
/// 1. Repository clone succeeds
/// 2. Container creation fails (e.g., image doesn't exist)
/// 3. System should clean up the workspace directory
/// 4. User should be able to retry `new` without "already exists" error
///
/// This test simulates the failure by using a non-existent Docker image.
#[test]
#[ignore] // Requires Docker to be running
fn test_failed_container_creation_cleanup() {
    let temp_workspace = TempDir::new().expect("Failed to create temp directory");
    let workspace_path = temp_workspace.path();

    let agent_name = "test-container-fail";
    let valid_repo = "https://github.com/octocat/Hello-World.git";
    // Use a non-existent image to trigger container creation failure
    let fake_image = "nonexistent-image-that-does-not-exist:latest";

    // Attempt to create agent - clone will succeed but container creation will fail
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("CROWDCONTROL_IMAGE", fake_image)
        .arg("new")
        .arg(agent_name)
        .arg(valid_repo)
        .arg("--skip-verification");

    // The command should fail due to container creation failure
    cmd.assert().failure();

    // CRITICAL: Verify the workspace directory was cleaned up
    let agent_dir = workspace_path.join(agent_name);
    assert!(
        !agent_dir.exists(),
        "Agent directory should be cleaned up after container creation failure. \
         Found leftover directory at {:?}",
        agent_dir
    );

    // Verify no metadata.json exists
    let metadata_path = agent_dir.join("metadata.json");
    assert!(
        !metadata_path.exists(),
        "metadata.json should not exist after failed container creation"
    );

    // Should be able to retry without "already exists" error
    let mut cmd2 = Command::cargo_bin("crowdcontrol").unwrap();
    cmd2.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("CROWDCONTROL_IMAGE", fake_image)
        .arg("new")
        .arg(agent_name)
        .arg(valid_repo)
        .arg("--skip-verification");

    // Should fail again for the same reason (container failure), not "already exists"
    cmd2.assert()
        .failure()
        .stderr(predicate::str::contains("already exists").not());
}

/// Test that verifies the inconsistent state we want to prevent:
/// - Workspace exists (clone succeeded)
/// - No metadata.json (container failed before save)
///
/// In this state:
/// - `new` fails with "already exists"
/// - `start` fails with "not found"
///
/// This test creates the inconsistent state manually to verify detection.
#[test]
fn test_detect_inconsistent_state_no_metadata() {
    let temp_workspace = TempDir::new().expect("Failed to create temp directory");
    let workspace_path = temp_workspace.path();

    let agent_name = "orphaned-workspace";

    // Manually create the inconsistent state: directory exists but no metadata
    let agent_dir = workspace_path.join(agent_name);
    fs::create_dir_all(&agent_dir).expect("Failed to create agent directory");

    // Create some files to simulate a cloned repo
    fs::write(agent_dir.join("README.md"), "# Test").expect("Failed to write file");

    // Try to create a new agent - should fail with "already exists"
    let mut cmd_new = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_new
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .arg("new")
        .arg(agent_name)
        .arg("https://github.com/octocat/Hello-World.git")
        .arg("--skip-verification");

    cmd_new
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Try to start the agent - should fail with "not found"
    let mut cmd_start = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_start
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .arg("start")
        .arg(agent_name);

    cmd_start
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test the same inconsistent state with project:label format (nested paths)
#[test]
fn test_detect_inconsistent_state_nested_path() {
    let temp_workspace = TempDir::new().expect("Failed to create temp directory");
    let workspace_path = temp_workspace.path();

    // Set up a project first
    let temp_home = TempDir::new().expect("Failed to create temp home");
    let projects_path = temp_home.path().join(".crowdcontrol");
    fs::create_dir_all(&projects_path).expect("Failed to create .crowdcontrol dir");
    fs::write(
        projects_path.join("projects.toml"),
        r#"[projects.myproject]
url = "https://github.com/octocat/Hello-World.git"
"#,
    )
    .expect("Failed to write projects.toml");

    // Manually create the inconsistent state with nested structure
    // myproject:test -> ~/.crowdcontrol/myproject/test/
    let agent_dir = workspace_path.join("myproject").join("test");
    fs::create_dir_all(&agent_dir).expect("Failed to create nested agent directory");
    fs::write(agent_dir.join("README.md"), "# Test").expect("Failed to write file");

    // The filesystem name for myproject:test is myproject-test
    // Try to create - should fail with "already exists"
    let mut cmd_new = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_new
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", temp_home.path())
        .arg("new")
        .arg("myproject:test");

    cmd_new
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Try to start - should fail with "not found"
    let mut cmd_start = Command::cargo_bin("crowdcontrol").unwrap();
    cmd_start
        .env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .env("HOME", temp_home.path())
        .arg("start")
        .arg("myproject:test");

    cmd_start
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
