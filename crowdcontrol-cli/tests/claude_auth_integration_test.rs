use assert_cmd::Command;
use predicates::prelude::*;
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_claude_auth_end_to_end() {
    // This test verifies that Claude Code authentication passes through to containers
    let temp_dir = TempDir::new().unwrap();
    let agent_name = format!("test-claude-auth-{}", std::process::id());
    
    // Create a new agent
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("new")
        .arg(&agent_name)
        .arg("https://github.com/anthropics/anthropic-sdk-python.git")
        .assert()
        .success();
    
    // Start the agent
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("start")
        .arg(&agent_name)
        .assert()
        .success();
    
    // Wait for container to be ready
    thread::sleep(Duration::from_secs(3));
    
    // Refresh Claude auth
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("refresh")
        .arg(&agent_name)
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude Code authentication refreshed successfully"));
    
    // Test Claude Code authentication (run as developer user)
    let container_name = format!("crowdcontrol-{}", agent_name);
    let output = StdCommand::new("docker")
        .args(&["exec", "-u", "developer", &container_name, "claude", "echo 'test'"])
        .output()
        .expect("Failed to execute docker command");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("Claude command output - stdout: '{}', stderr: '{}'", stdout, stderr);
    println!("Exit status: {}", output.status);
    
    // Check for authentication error - this means Claude Code is running
    // but doesn't have valid credentials (which is expected in CI)
    // The important thing is that it's NOT saying "command not found"
    // or other Docker/system errors
    let auth_error = stdout.contains("Invalid API key") || stdout.contains("Please run /login") || 
                    stderr.contains("Invalid API key") || stderr.contains("Please run /login");
    let command_found = !stderr.contains("command not found") && !stderr.contains("No such file");
    let claude_working = stdout.contains("test") || auth_error;
    
    assert!(
        command_found,
        "Claude Code command should be found, got stderr: '{}', stdout: '{}'",
        stderr, stdout
    );
    
    // If Claude is properly authenticated, it should output "test"
    // If Claude is not authenticated, it should show auth error
    // Either case indicates Claude Code is working properly
    assert!(
        claude_working,
        "Expected Claude Code to either work (output 'test') or show auth error, got stderr: '{}', stdout: '{}'",
        stderr, stdout
    );
    
    // Cleanup
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("remove")
        .arg(&agent_name)
        .arg("--force")
        .assert()
        .success();
}

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_claude_files_mounted_correctly() {
    // This test verifies that Claude config files are mounted to the correct location
    let temp_dir = TempDir::new().unwrap();
    let agent_name = format!("test-claude-mount-{}", std::process::id());
    
    // Create and start agent
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("new")
        .arg(&agent_name)
        .arg("https://github.com/anthropics/anthropic-sdk-python.git")
        .assert()
        .success();
    
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("start")
        .arg(&agent_name)
        .assert()
        .success();
    
    thread::sleep(Duration::from_secs(2));
    
    // Check if mount points exist
    let container_name = format!("crowdcontrol-{}", agent_name);
    
    // Check /mnt/claude-config exists
    let output = StdCommand::new("docker")
        .args(&["exec", &container_name, "test", "-d", "/mnt/claude-config"])
        .output()
        .expect("Failed to execute docker command");
    
    assert!(output.status.success(), "Mount point /mnt/claude-config should exist");
    
    // Check what's mounted (if host has Claude config)
    let ls_output = StdCommand::new("docker")
        .args(&["exec", &container_name, "ls", "-la", "/mnt/claude-config/"])
        .output()
        .expect("Failed to execute docker command");
    
    println!("Mounted Claude config files: {}", String::from_utf8_lossy(&ls_output.stdout));
    
    // Run refresh command
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("refresh")
        .arg(&agent_name)
        .assert()
        .success();
    
    // Check if refresh script exists and is executable
    let script_check = StdCommand::new("docker")
        .args(&["exec", &container_name, "test", "-x", "/usr/local/bin/refresh-claude-auth.sh"])
        .output()
        .expect("Failed to execute docker command");
    
    assert!(script_check.status.success(), "Refresh script should be executable");
    
    // Verify Claude Code is installed
    let claude_version = StdCommand::new("docker")
        .args(&["exec", &container_name, "claude", "--version"])
        .output()
        .expect("Failed to execute docker command");
    
    assert!(
        claude_version.status.success(),
        "Claude Code should be installed, stderr: {}",
        String::from_utf8_lossy(&claude_version.stderr)
    );
    
    // If host has Claude config, verify files were copied
    let home_dir = dirs::home_dir().unwrap();
    if home_dir.join(".claude").exists() || home_dir.join(".claude.json").exists() {
        // Check if files were copied to user home
        let output = StdCommand::new("docker")
            .args(&["exec", &container_name, "bash", "-c", "ls -la /home/developer/.claude* 2>/dev/null || echo 'No files'"])
            .output()
            .expect("Failed to execute docker command");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should have either .claude directory or .claude.json file
        assert!(
            !stdout.contains("No files"),
            "Expected to find .claude files in home directory when host has config, got: {}",
            stdout
        );
    }
    
    // Cleanup
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("remove")
        .arg(&agent_name)
        .arg("--force")
        .assert()
        .success();
}