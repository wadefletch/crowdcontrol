// TDD: Tests for CLI repo subcommands
// Run with: cargo test --package crowdcontrol-cli --test repo_cli_test

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, TempDir) {
    let workspaces_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();

    // Create config directory structure
    let crowdcontrol_config_dir = config_dir.path().join(".config").join("crowdcontrol");
    fs::create_dir_all(&crowdcontrol_config_dir).unwrap();

    (workspaces_dir, config_dir)
}

#[test]
fn test_repo_help() {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("repo")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("repo"))
        .stdout(predicates::str::contains("Usage:"));
}

#[test]
fn test_repo_add_help() {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("repo")
        .arg("add")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("add"))
        .stdout(predicates::str::contains("slug"));
}

#[test]
fn test_repo_add_success() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("myapp")
        .arg("git@github.com:org/myapp.git")
        .assert()
        .success()
        .stdout(predicates::str::contains("myapp"));
}

#[test]
fn test_repo_add_with_default_branch() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("myapp")
        .arg("git@github.com:org/myapp.git")
        .arg("--default-branch")
        .arg("develop")
        .assert()
        .success();
}

#[test]
fn test_repo_add_missing_arguments() {
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.arg("repo")
        .arg("add")
        .assert()
        .failure()
        .stderr(predicates::str::contains("required"));
}

#[test]
fn test_repo_add_invalid_slug() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    // Slug with colon should fail
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("my:app")
        .arg("git@github.com:org/myapp.git")
        .assert()
        .failure()
        .stderr(predicates::str::contains("colon").or(predicates::str::contains(":")));
}

#[test]
fn test_repo_add_duplicate() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    // Add first repo
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("myapp")
        .arg("git@github.com:org/myapp.git")
        .assert()
        .success();

    // Try to add duplicate
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("myapp")
        .arg("git@github.com:other/myapp.git")
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));
}

#[test]
fn test_repo_list_empty() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("No repos").or(predicates::str::is_empty()));
}

#[test]
fn test_repo_list_shows_repos() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    // Add a repo first
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("myapp")
        .arg("git@github.com:org/myapp.git")
        .assert()
        .success();

    // List repos
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("myapp"))
        .stdout(predicates::str::contains("git@github.com:org/myapp.git"));
}

#[test]
fn test_repo_remove_success() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    // Add a repo first
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("myapp")
        .arg("git@github.com:org/myapp.git")
        .assert()
        .success();

    // Remove it
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("remove")
        .arg("myapp")
        .assert()
        .success();

    // Verify it's gone
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("myapp").not());
}

#[test]
fn test_repo_remove_nonexistent() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("remove")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn test_repo_list_json_format() {
    let (_workspaces_dir, config_dir) = setup_test_env();

    // Add a repo first
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("add")
        .arg("myapp")
        .arg("git@github.com:org/myapp.git")
        .assert()
        .success();

    // List in JSON format
    let mut cmd = Command::cargo_bin("crowdcontrol").unwrap();
    cmd.env("HOME", config_dir.path())
        .arg("repo")
        .arg("list")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicates::str::starts_with("["))
        .stdout(predicates::str::contains("myapp"));
}
