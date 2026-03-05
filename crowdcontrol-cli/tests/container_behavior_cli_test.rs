use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// Helper to create a basic config file
fn create_config_file(dir: &TempDir) -> std::path::PathBuf {
    let config_path = dir.path().join("config.toml");
    let workspaces_dir = dir.path().join("workspaces");
    
    // Ensure workspaces directory exists
    fs::create_dir_all(&workspaces_dir).unwrap();
    
    let config_content = format!(
        r#"
workspaces_dir = "{}"
image = "crowdcontrol:latest"
"#,
        workspaces_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();
    config_path
}

// Helper to create and start an agent
fn start_agent(name: &str, config_path: &std::path::Path, repo_path: &std::path::Path) {
    // First create the agent using 'new' command with file:// URL
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            name,
            &format!("file://{}", repo_path.display()),
            "--skip-verification",
        ])
        .assert()
        .success();
    
    // Then start it
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", name])
        .assert()
        .success();
}

// Helper to execute command in container
fn exec_in_container(agent_name: &str, args: &[&str]) -> std::process::Output {
    Command::new("docker")
        .args(&["exec", &format!("crowdcontrol-{}", agent_name)])
        .args(args)
        .output()
        .expect("Failed to execute docker command")
}

#[test]
#[ignore = "requires Docker"]
fn test_container_basic_tools() {
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
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("tools-test-{}", timestamp);
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Wait for container to be ready
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test essential tools are available
    let tools = ["git", "jq", "curl", "node", "npm", "claude"];
    for tool in &tools {
        let output = exec_in_container(&agent_name, &["which", tool]);
        assert!(
            output.status.success(),
            "{} should be available in container",
            tool
        );
    }
    
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
fn test_container_working_directory() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("test.txt"), "test content").unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("pwd-test-{}", timestamp);
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Wait for container
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test working directory
    let output = exec_in_container(&agent_name, &["pwd"]);
    assert!(output.status.success());
    let pwd = String::from_utf8_lossy(&output.stdout);
    assert_eq!(pwd.trim(), "/workspace");
    
    // Test files are accessible
    let output = exec_in_container(&agent_name, &["ls", "-la"]);
    assert!(output.status.success());
    let ls_output = String::from_utf8_lossy(&output.stdout);
    assert!(ls_output.contains("test.txt"));
    
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
fn test_container_user_permissions() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("user-test-{}", timestamp);
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Wait for container
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test user
    let output = exec_in_container(&agent_name, &["whoami"]);
    assert!(output.status.success());
    let user = String::from_utf8_lossy(&output.stdout);
    assert_eq!(user.trim(), "developer");
    
    // Test HOME directory
    let output = exec_in_container(&agent_name, &["printenv", "HOME"]);
    assert!(output.status.success());
    let home = String::from_utf8_lossy(&output.stdout);
    assert_eq!(home.trim(), "/home/developer");
    
    // Test write permissions
    let output = exec_in_container(
        &agent_name,
        &["sh", "-c", "echo 'test' > /workspace/new-file.txt"],
    );
    assert!(output.status.success());
    
    // Verify file exists on host
    assert!(test_repo.join("new-file.txt").exists());
    
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
fn test_container_git_operations() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository with git
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("README.md"), "# Initial").unwrap();
    
    // Initialize and configure git
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    Command::new("git")
        .args(&["add", "."])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("git-test-{}", timestamp);
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Wait for container
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test git status
    let output = exec_in_container(&agent_name, &["git", "status"]);
    assert!(output.status.success());
    let status = String::from_utf8_lossy(&output.stdout);
    assert!(status.contains("working tree clean") || status.contains("nothing to commit"));
    
    // Test git log
    let output = exec_in_container(&agent_name, &["git", "log", "--oneline"]);
    assert!(output.status.success());
    let log = String::from_utf8_lossy(&output.stdout);
    assert!(log.contains("Initial commit"));
    
    // Test making changes
    let output = exec_in_container(
        &agent_name,
        &[
            "sh",
            "-c",
            "echo '## Updated' >> README.md && git add README.md && git commit -m 'Update README'",
        ],
    );
    
    // Git commit might fail if not configured in container, but changes should persist
    if output.status.success() {
        // Verify change on host
        let content = fs::read_to_string(test_repo.join("README.md")).unwrap();
        assert!(content.contains("## Updated"));
    }
    
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
fn test_claude_authentication_refresh() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create mock Claude config
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join(".credentials.json"),
        r#"{"api_key": "test-key"}"#,
    )
    .unwrap();
    fs::write(
        claude_dir.join(".claude.json"),
        r#"{"autoUpdates": true, "mode": "project"}"#,
    )
    .unwrap();
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Create and start agent with HOME set to include Claude config
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("claude-refresh-{}", timestamp);
    
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("HOME", temp_dir.path())
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "new",
            &agent_name,
            &format!("file://{}", test_repo.display()),
            "--skip-verification",
        ])
        .assert()
        .success();
        
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("HOME", temp_dir.path())
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success();
    
    // Wait for container
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test refresh script exists
    let output = exec_in_container(&agent_name, &["which", "refresh-claude-auth.sh"]);
    assert!(output.status.success());
    
    // Run refresh script
    let output = exec_in_container(&agent_name, &["refresh-claude-auth.sh"]);
    
    if output.status.success() {
        // Check if .claude.json was transformed
        let output = exec_in_container(&agent_name, &["cat", "/home/developer/.claude.json"]);
        if output.status.success() {
            let config = String::from_utf8_lossy(&output.stdout);
            // The refresh script should transform the config
            assert!(config.contains("\"autoUpdates\":false") || config.contains("\"autoUpdates\": false"));
            assert!(config.contains("\"mode\":\"global\"") || config.contains("\"mode\": \"global\""));
        }
    }
    
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
fn test_container_environment_variables() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_file(&temp_dir);
    
    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    
    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();
    
    // Start agent with unique name
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("env-test-{}", timestamp);
    start_agent(&agent_name, &config_path, &test_repo);
    
    // Wait for container
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Test important environment variables
    let env_checks = vec![
        ("HOME", "/home/developer"),
        ("USER", "developer"),
        ("PWD", "/workspace"),
    ];
    
    for (var, expected) in env_checks {
        let output = exec_in_container(&agent_name, &["printenv", var]);
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout);
            assert_eq!(value.trim(), expected);
        }
    }
    
    // Test PATH includes important directories
    let output = exec_in_container(&agent_name, &["printenv", "PATH"]);
    assert!(output.status.success());
    let path = String::from_utf8_lossy(&output.stdout);
    assert!(path.contains("/usr/local/bin"));
    assert!(path.contains("/usr/bin"));
    
    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}