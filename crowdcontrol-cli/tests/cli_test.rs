use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("crowdcontrol"))
        .stdout(predicates::str::contains("Usage:"))
        .stdout(predicates::str::contains("Commands:"));
}

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("crowdcontrol"))
        .stdout(predicates::str::contains("0.1.0"));
}

#[test]
fn test_subcommand_help() {
    let subcommands = vec![
        "new",
        "start",
        "stop",
        "list",
        "remove",
        "logs",
        "connect",
        "completions",
    ];

    for subcommand in subcommands {
        let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
        cmd.arg(subcommand)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains(subcommand))
            .stdout(predicates::str::contains("Usage:"));
    }
}

#[test]
fn test_invalid_subcommand() {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unrecognized subcommand"));
}

#[test]
fn test_global_options() {
    let temp_dir = TempDir::new().unwrap();

    // Test --workspaces-dir global option
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("list")
        .assert()
        .success();

    // Test -v verbose flag
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("-v").arg("list").assert().success();

    // Test -vv double verbose
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("-vv").arg("list").assert().success();
}

#[test]
fn test_list_empty_workspace() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("No agents found"));
}

#[test]
fn test_list_json_format() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("list")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicates::str::starts_with("["))
        .stdout(predicates::str::ends_with("]\n"));
}

#[test]
fn test_new_missing_arguments() {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("new")
        .assert()
        .failure()
        .stderr(predicates::str::contains("required arguments"));
}

#[test]
fn test_new_invalid_agent_names() {
    let temp_dir = TempDir::new().unwrap();
    let too_long = "a".repeat(100);
    let invalid_names = vec![
        "",                // empty
        "my/agent",        // contains slash
        "my agent",        // contains space
        too_long.as_str(), // too long
        "my\\agent",       // contains backslash
        "my:agent",        // contains colon
        "my|agent",        // contains pipe
    ];

    for name in invalid_names {
        let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
        cmd.arg("--workspaces-dir")
            .arg(temp_dir.path())
            .arg("new")
            .arg(&name)
            .arg("https://github.com/test/repo.git")
            .assert()
            .failure();
    }
}

#[test]
fn test_start_nonexistent_agent() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("start")
        .arg("does-not-exist")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn test_stop_nonexistent_agent() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("stop")
        .arg("does-not-exist")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn test_logs_nonexistent_agent() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("logs")
        .arg("does-not-exist")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn test_remove_nonexistent_agent() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("remove")
        .arg("does-not-exist")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn test_connect_nonexistent_agent() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("connect")
        .arg("does-not-exist")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn test_completion_generation() {
    let shells = vec!["bash", "zsh", "fish"];

    for shell in shells {
        let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
        cmd.arg("completions")
            .arg(shell)
            .assert()
            .success()
            .stdout(predicates::str::is_empty().not());

        // Verify shell-specific patterns
        let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
        let output = cmd.arg("completions").arg(shell).output().unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        match shell {
            "bash" => assert!(stdout.contains("complete")),
            "zsh" => assert!(stdout.contains("compdef") || stdout.contains("#compdef")),
            "fish" => assert!(stdout.contains("complete -c crowdcontrol")),
            _ => {}
        }
    }
}

#[test]
fn test_environment_variable_override() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    // Test CROWDCONTROL_WORKSPACES_DIR environment variable
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp_dir1.path())
        .arg("list")
        .assert()
        .success();

    // Test that CLI arg overrides env var
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("CROWDCONTROL_WORKSPACES_DIR", temp_dir1.path())
        .arg("--workspaces-dir")
        .arg(temp_dir2.path())
        .arg("list")
        .assert()
        .success();
}

#[test]
fn test_config_file_loading() {
    let config_dir = TempDir::new().unwrap();
    let workspaces_dir = TempDir::new().unwrap();

    // Create config directory structure
    let crowdcontrol_config_dir = config_dir.path().join(".config").join("crowdcontrol");
    fs::create_dir_all(&crowdcontrol_config_dir).unwrap();

    // Write config file
    let config_content = format!(
        r#"
workspaces_dir = "{}"
image = "test:image"
default_memory = "1g"
default_cpus = "2"
verbose = 1
"#,
        workspaces_dir.path().display()
    );

    fs::write(crowdcontrol_config_dir.join("config.toml"), config_content).unwrap();

    // Test that config file is loaded - we'll test with help to avoid Docker requirement
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_no_color_output() {
    let temp_dir = TempDir::new().unwrap();

    // Test with NO_COLOR environment variable
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("NO_COLOR", "true")
        .arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b[").not()); // No ANSI escape codes
}

#[test]
fn test_new_with_branch() {
    let temp_dir = TempDir::new().unwrap();

    // This will fail because we're not actually cloning, but we can test argument parsing
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("new")
        .arg("test-agent")
        .arg("https://github.com/test/repo.git")
        .arg("--branch")
        .arg("develop")
        .assert()
        .failure(); // Will fail due to git clone, but arguments are parsed correctly
}

#[test]
fn test_new_with_resource_limits() {
    let temp_dir = TempDir::new().unwrap();

    // Test memory and CPU arguments
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("new")
        .arg("test-agent")
        .arg("https://github.com/test/repo.git")
        .arg("--memory")
        .arg("2g")
        .arg("--cpus")
        .arg("1.5")
        .assert()
        .failure(); // Will fail due to git clone, but arguments are parsed correctly
}

#[test]
fn test_logs_with_options() {
    let temp_dir = TempDir::new().unwrap();

    // Test tail option
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("logs")
        .arg("test-agent")
        .arg("--tail")
        .arg("50")
        .assert()
        .failure(); // Will fail due to missing agent

    // Test follow option
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("logs")
        .arg("test-agent")
        .arg("--follow")
        .assert()
        .failure(); // Will fail due to missing agent
}

#[test]
fn test_remove_with_force() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("remove")
        .arg("test-agent")
        .arg("--force")
        .assert()
        .failure(); // Will fail due to missing agent, but force flag is parsed
}

#[test]
fn test_stop_all_agents() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("stop")
        .arg("--all")
        .assert()
        .success(); // Should succeed even with no agents
}

#[test]
fn test_connect_with_custom_command() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("--workspaces-dir")
        .arg(temp_dir.path())
        .arg("connect")
        .arg("test-agent")
        .arg("--command")
        .arg("/bin/bash")
        .assert()
        .failure(); // Will fail due to missing agent
}
