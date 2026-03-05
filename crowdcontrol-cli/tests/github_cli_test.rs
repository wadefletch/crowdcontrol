use assert_cmd::Command;
use predicates::prelude::*;
use std::{env, fs};
use tempfile::TempDir;

mod test_utils;
use test_utils::{generate_test_id, create_config_with_github};


#[test]
#[ignore = "requires Docker"]
fn test_start_without_github_auth() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_with_github(&temp_dir, false);

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

    // Create and start agent without GitHub configuration
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("no-github-test-{}", timestamp);
    
    Command::cargo_bin("crowdcontrol")
        .unwrap()
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
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success()
        .stdout(predicate::str::contains(&agent_name).and(predicate::str::contains("started")));

    // Wait for container to be ready
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Check that git operations work locally
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "git",
            "config",
            "--global",
            "--list",
        ])
        .output()
        .unwrap();

    let git_config = String::from_utf8_lossy(&output.stdout);
    // Should have basic git config but no credentials
    assert!(git_config.contains("user.name"));
    assert!(!git_config.contains("credential.helper=store"));

    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker and GitHub credentials"]
fn test_start_with_github_auth() {
    // Skip if no GitHub credentials
    if env::var("GITHUB_INSTALLATION_TOKEN").is_err() && env::var("GITHUB_APP_ID").is_err() {
        println!("Skipping test - no GitHub credentials configured");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_with_github(&temp_dir, true);

    // Create a test repository
    let test_repo = temp_dir.path().join("test-repo");
    fs::create_dir_all(&test_repo).unwrap();
    fs::write(test_repo.join("README.md"), "# Test Repo with Auth").unwrap();

    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&test_repo)
        .assert()
        .success();

    // Create and start agent with GitHub configuration
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let agent_name = format!("github-auth-test-{}", timestamp);
    
    Command::cargo_bin("crowdcontrol")
        .unwrap()
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
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["start", &agent_name])
        .assert()
        .success()
        .stdout(predicate::str::contains(&agent_name).and(predicate::str::contains("started")));

    // Wait for container to be ready
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Check that GitHub authentication is configured
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "git",
            "config",
            "--global",
            "--list",
        ])
        .output()
        .unwrap();

    let git_config = String::from_utf8_lossy(&output.stdout);
    // Should have GitHub-specific configuration
    assert!(git_config.contains("credential.helper=store"));
    assert!(git_config.contains("user.name=CrowdControl[bot]"));
    assert!(git_config.contains("user.email=crowdcontrol[bot]@users.noreply.github.com"));

    // Check for git credentials file
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", agent_name),
            "test",
            "-f",
            "/home/developer/.git-credentials",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Git credentials file should exist when GitHub auth is configured"
    );

    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", config_path)
        .args(&["stop", &agent_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_github_auth_status_check() {
    let temp_dir = TempDir::new().unwrap();

    // Test without GitHub config
    let config_path = create_config_with_github(&temp_dir, false);
    
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["config", "--show"])
        .assert()
        .success();

    // Test with GitHub config (if available)
    if env::var("GITHUB_INSTALLATION_TOKEN").is_ok() || env::var("GITHUB_APP_ID").is_ok() {
        let config_path_with_github = create_config_with_github(&temp_dir, true);
        
        Command::cargo_bin("crowdcontrol")
            .unwrap()
            .env("CC_CONFIG_FILE", &config_path_with_github)
            .args(&["config", "--show"])
            .assert()
            .success();
    }
}

#[test]
#[ignore = "requires Docker and GitHub credentials and test repository"]
fn test_github_clone_operation() {
    // Skip if no GitHub credentials or test repo
    if env::var("GITHUB_INSTALLATION_TOKEN").is_err() && env::var("GITHUB_APP_ID").is_err() {
        println!("Skipping test - no GitHub credentials configured");
        return;
    }

    let test_repo_url = match env::var("GITHUB_TEST_REPO") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping test - GITHUB_TEST_REPO not set");
            return;
        }
    };

    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_with_github(&temp_dir, true);

    // Extract repo name from URL for the agent name
    let repo_name = test_repo_url
        .split('/')
        .last()
        .unwrap_or("test-repo")
        .trim_end_matches(".git");

    // Start agent from GitHub repository
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&[
            "start",
            repo_name,
            "--repo",
            &test_repo_url,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(repo_name).and(predicate::str::contains("started")));

    // Wait for clone to complete
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Verify repository was cloned
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", repo_name),
            "git",
            "status",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Git repository should be accessible"
    );

    // Check remote configuration
    let output = Command::new("docker")
        .args(&[
            "exec",
            &format!("crowdcontrol-{}", repo_name),
            "git",
            "remote",
            "get-url",
            "origin",
        ])
        .output()
        .unwrap();

    let remote_url = String::from_utf8_lossy(&output.stdout);
    assert!(remote_url.contains("github.com"));

    // Clean up
    Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", config_path)
        .args(&["stop", repo_name])
        .assert()
        .success();
}

#[test]
#[ignore = "requires Docker"]
fn test_github_enterprise_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    // Create config with GitHub Enterprise URL
    let workspaces_dir = temp_dir.path().join("workspaces");
    fs::create_dir_all(&workspaces_dir).unwrap();
    
    let config_content = format!(
        r#"
workspaces_dir = "{}"
image = "crowdcontrol:latest"

[github]
installation_token = "ghs_test_token"
base_url = "https://github.enterprise.example.com"
"#,
        workspaces_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Check that config is loaded correctly
    let output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["config", "--show"])
        .assert()
        .success();

    // Just verify the config contains the enterprise URL somewhere
    output
        .stdout(predicate::str::contains("github.enterprise.example.com"));
}

#[test]
#[ignore = "requires Docker"]
fn test_multiple_agents_with_different_repos() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_config_with_github(&temp_dir, false);

    // Create multiple test repositories
    let repo_configs = vec![
        ("repo1", generate_test_id("repo1")),
        ("repo2", generate_test_id("repo2")),
        ("repo3", generate_test_id("repo3")),
    ];
    
    for (repo_name, agent_name) in &repo_configs {
        let repo_path = temp_dir.path().join(repo_name);
        fs::create_dir_all(&repo_path).unwrap();
        fs::write(repo_path.join("README.md"), format!("# {}", repo_name)).unwrap();
        
        Command::new("git")
            .args(&["init"])
            .current_dir(&repo_path)
            .assert()
            .success();

        // Create and start agent for each repository
        Command::cargo_bin("crowdcontrol")
            .unwrap()
            .env("CC_CONFIG_FILE", &config_path)
            .args(&[
                "new",
                agent_name,
                &format!("file://{}", repo_path.display()),
                "--skip-verification",
            ])
            .assert()
            .success();
            
        Command::cargo_bin("crowdcontrol")
            .unwrap()
            .env("CC_CONFIG_FILE", &config_path)
            .args(&["start", agent_name])
            .assert()
            .success();
    }

    // List all agents and verify all are listed
    let output = Command::cargo_bin("crowdcontrol")
        .unwrap()
        .env("CC_CONFIG_FILE", &config_path)
        .args(&["list"])
        .assert()
        .success();
    
    // Check output contains all agent names
    let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
    for (_, agent_name) in &repo_configs {
        assert!(stdout.contains(agent_name), "List output should contain {}", agent_name);
    }

    // Clean up all agents
    for (_, agent_name) in &repo_configs {
        Command::cargo_bin("crowdcontrol")
            .unwrap()
            .env("CC_CONFIG_FILE", &config_path)
            .args(&["stop", agent_name])
            .assert()
            .success();
    }
}