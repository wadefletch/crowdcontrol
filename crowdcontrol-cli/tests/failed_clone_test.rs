use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_failed_clone_cleanup() {
    // Create a temporary workspace directory
    let temp_workspace = TempDir::new().expect("Failed to create temp directory");
    let workspace_path = temp_workspace.path();

    let agent_name = "test-failed-clone";
    let invalid_repo = "git@github.com:nonexistent/repo.git";
    let invalid_branch = "nonexistent-branch";

    // Attempt to create agent with invalid repo/branch - this should fail
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .arg("new")
        .arg(agent_name)
        .arg(invalid_repo)
        .arg("--branch")
        .arg(invalid_branch)
        .arg("--skip-verification");

    // The command should fail
    cmd.assert().failure();

    // Verify no partial state exists
    let agent_dir = workspace_path.join(agent_name);
    assert!(
        !agent_dir.exists(),
        "Agent directory should not exist after failed clone"
    );

    // Verify we can try to create the agent again (no "already exists" error)
    let mut cmd2 = Command::cargo_bin("crowdcontrol").unwrap();
    cmd2.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .arg("new")
        .arg(agent_name)
        .arg(invalid_repo)
        .arg("--branch")
        .arg(invalid_branch)
        .arg("--skip-verification");

    // Should fail for the same reason (clone failure), not "already exists"
    cmd2.assert()
        .failure()
        .stderr(predicate::str::contains("already exists").not());
}

#[test]
fn test_failed_clone_with_valid_repo_invalid_branch() {
    let temp_workspace = TempDir::new().expect("Failed to create temp directory");
    let workspace_path = temp_workspace.path();

    let agent_name = "test-invalid-branch";
    // Use a real repo but invalid branch
    let valid_repo = "https://github.com/octocat/Hello-World.git";
    let invalid_branch = "definitely-does-not-exist-branch";

    // Attempt to create agent - should fail due to invalid branch
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .arg("new")
        .arg(agent_name)
        .arg(valid_repo)
        .arg("--branch")
        .arg(invalid_branch)
        .arg("--skip-verification");

    cmd.assert().failure();

    // Verify cleanup happened
    let agent_dir = workspace_path.join(agent_name);
    assert!(
        !agent_dir.exists(),
        "Agent directory should be cleaned up after failed clone"
    );

    // Should be able to retry without "already exists" error
    let mut cmd2 = Command::cargo_bin("crowdcontrol").unwrap();
    cmd2.env("CROWDCONTROL_WORKSPACES_DIR", workspace_path)
        .arg("new")
        .arg(agent_name)
        .arg(valid_repo)
        .arg("--branch")
        .arg("main") // Try with valid branch this time
        .arg("--skip-verification");

    // This should work or fail for a different reason (not "already exists")
    cmd2.assert()
        .stderr(predicate::str::contains("already exists").not());
}
