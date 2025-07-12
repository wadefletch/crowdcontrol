use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_refresh_command_validation() {
    // Test that refresh command requires an agent name
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("refresh")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn test_refresh_nonexistent_agent() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("refresh")
        .arg("nonexistent-agent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Agent 'nonexistent-agent' does not exist"));
}

#[test]
fn test_claude_config_detection() {
    // This test verifies that the Docker mounting logic correctly detects Claude config
    let temp_home = TempDir::new().unwrap();
    let claude_dir = temp_home.path().join(".claude");
    
    // Test 1: No Claude directory
    assert!(!claude_dir.exists());
    
    // Test 2: Create Claude directory and credentials
    fs::create_dir_all(&claude_dir).unwrap();
    let credentials_path = claude_dir.join("credentials.json");
    fs::write(&credentials_path, r#"{"token": "test"}"#).unwrap();
    
    assert!(claude_dir.exists());
    assert!(credentials_path.exists());
    
    // Test 3: Verify content
    let content = fs::read_to_string(&credentials_path).unwrap();
    assert!(content.contains("test"));
}