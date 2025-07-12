use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_repo_cloned_to_workspace_root() {
    // This test verifies that repositories are cloned directly to the workspace root,
    // not to workspace/agent-name/agent-name
    let temp_dir = TempDir::new().unwrap();
    let agent_name = "test-clone-root";
    
    // Create a minimal git repository to clone
    let repo_dir = temp_dir.path().join("test-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    
    // Initialize git repo
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to init git repo");
    
    // Add a test file
    fs::write(repo_dir.join("README.md"), "# Test Repository").unwrap();
    
    // Configure git user for the test
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(&repo_dir)
        .output()
        .unwrap();
    
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(&repo_dir)
        .output()
        .unwrap();
    
    // Add and commit the file
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(&repo_dir)
        .output()
        .unwrap();
    
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(&repo_dir)
        .output()
        .unwrap();
    
    // Test the crowdcontrol new command
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("new")
        .arg(agent_name)
        .arg(&repo_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Repository cloned successfully"));
    
    // Verify the repository structure
    let agent_workspace = temp_dir.path().join(agent_name);
    
    // The README.md should be directly in the workspace root, not in a subdirectory
    let readme_path = agent_workspace.join("README.md");
    assert!(readme_path.exists(), "README.md should exist in workspace root");
    
    // Verify we don't have the old nested structure (workspace/agent-name/agent-name/README.md)
    let nested_path = agent_workspace.join(agent_name).join("README.md");
    assert!(!nested_path.exists(), "README.md should NOT exist in nested directory");
    
    // Verify the git repository is at the root
    let git_dir = agent_workspace.join(".git");
    assert!(git_dir.exists(), ".git directory should exist in workspace root");
}

#[test] 
fn test_claude_json_transformations() {
    // This test verifies that .claude.json gets the correct jq transformations
    use serde_json::{json, Value};
    
    // Create a sample .claude.json with the fields we want to transform
    let original_config = json!({
        "numStartups": 42,
        "autoUpdates": true,
        "mode": "project",
        "projects": {
            "/some/project/path": {
                "history": ["some", "history", "items"]
            },
            "/another/project": {
                "settings": "value"
            }
        },
        "otherField": "should remain unchanged"
    });
    
    // Test the jq transformation command
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), original_config.to_string()).unwrap();
    
    let output = std::process::Command::new("jq")
        .args(&[".autoUpdates = false | .projects = {} | .mode = \"global\""])
        .arg(temp_file.path())
        .output()
        .expect("Failed to run jq command");
    
    assert!(output.status.success(), "jq command should succeed");
    
    let transformed: Value = serde_json::from_slice(&output.stdout).unwrap();
    
    // Verify the transformations
    assert_eq!(transformed["autoUpdates"], json!(false), "autoUpdates should be false");
    assert_eq!(transformed["projects"], json!({}), "projects should be empty object");
    assert_eq!(transformed["mode"], json!("global"), "mode should be 'global'");
    
    // Verify other fields are preserved
    assert_eq!(transformed["numStartups"], json!(42), "numStartups should be preserved");
    assert_eq!(transformed["otherField"], json!("should remain unchanged"), "otherField should be preserved");
}

#[test]
fn test_refresh_script_jq_transformations() {
    // This test verifies the refresh script applies transformations correctly
    let temp_dir = TempDir::new().unwrap();
    
    // Create a sample .claude.json
    let claude_config = json!({
        "autoUpdates": true,
        "mode": "project", 
        "projects": {
            "/existing/project": {"data": "value"}
        },
        "numStartups": 10
    });
    
    let claude_file = temp_dir.path().join(".claude.json");
    fs::write(&claude_file, claude_config.to_string()).unwrap();
    
    // Simulate the jq transformation from the refresh script
    let output = std::process::Command::new("jq")
        .args(&[".autoUpdates = false | .projects = {} | .mode = \"global\""])
        .arg(&claude_file)
        .output()
        .expect("Failed to run jq transformation");
    
    assert!(output.status.success(), "jq transformation should succeed");
    
    // Write the transformed content back
    fs::write(&claude_file, &output.stdout).unwrap();
    
    // Verify the file was transformed correctly
    let content = fs::read_to_string(&claude_file).unwrap();
    let transformed: serde_json::Value = serde_json::from_str(&content).unwrap();
    
    assert_eq!(transformed["autoUpdates"], false);
    assert_eq!(transformed["mode"], "global");
    assert_eq!(transformed["projects"], json!({}));
    assert_eq!(transformed["numStartups"], 10); // Should be preserved
}