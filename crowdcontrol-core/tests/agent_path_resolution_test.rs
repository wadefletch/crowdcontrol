// TDD: Tests for agent path resolution with both colon and hyphen formats
// Run with: cargo test --package crowdcontrol-core --test agent_path_resolution_test

use crowdcontrol_core::Config;
use std::fs;
use tempfile::TempDir;

fn create_test_config() -> (Config, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "test:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
    };
    (config, temp_dir)
}

/// Test that agent_workspace_path resolves "project:label" format (colon)
/// to the nested path "project/label/"
#[test]
fn test_agent_workspace_path_with_colon_format() {
    let (config, _temp_dir) = create_test_config();

    // Create nested directory structure
    let nested_path = config.workspaces_dir.join("myproject").join("main");
    fs::create_dir_all(&nested_path).unwrap();

    // Should resolve "myproject:main" to "myproject/main/"
    let resolved = config.agent_workspace_path("myproject:main");
    assert_eq!(
        resolved, nested_path,
        "agent_workspace_path should resolve colon format to nested path"
    );
}

/// Test that agent_workspace_path resolves "project-label" format (hyphen)
/// to the nested path "project/label/"
#[test]
fn test_agent_workspace_path_with_hyphen_format() {
    let (config, _temp_dir) = create_test_config();

    // Create nested directory structure
    let nested_path = config.workspaces_dir.join("myproject").join("main");
    fs::create_dir_all(&nested_path).unwrap();

    // Should resolve "myproject-main" to "myproject/main/"
    let resolved = config.agent_workspace_path("myproject-main");
    assert_eq!(
        resolved, nested_path,
        "agent_workspace_path should resolve hyphen format to nested path"
    );
}

/// Test that standalone agents (no project) resolve correctly
#[test]
fn test_agent_workspace_path_standalone() {
    let (config, _temp_dir) = create_test_config();

    // Create standalone agent directory
    let standalone_path = config.workspaces_dir.join("standalone-agent");
    fs::create_dir_all(&standalone_path).unwrap();

    // Should resolve to flat path
    let resolved = config.agent_workspace_path("standalone-agent");
    assert_eq!(
        resolved, standalone_path,
        "agent_workspace_path should resolve standalone agents to flat path"
    );
}

/// Test that colon and hyphen formats resolve to the same path
/// This is critical for the CLI to work correctly when users use either format
#[test]
fn test_colon_and_hyphen_resolve_to_same_path() {
    let (config, _temp_dir) = create_test_config();

    // Create nested directory structure
    let nested_path = config.workspaces_dir.join("carlyle").join("test");
    fs::create_dir_all(&nested_path).unwrap();

    let colon_path = config.agent_workspace_path("carlyle:test");
    let hyphen_path = config.agent_workspace_path("carlyle-test");

    assert_eq!(
        colon_path, hyphen_path,
        "Both 'carlyle:test' and 'carlyle-test' should resolve to the same nested path"
    );
    assert_eq!(
        colon_path, nested_path,
        "Both formats should resolve to ~/.crowdcontrol/carlyle/test/"
    );
}

/// Test load_agent_metadata works with colon format
#[test]
fn test_load_agent_metadata_with_colon_format() {
    use chrono::Utc;
    use crowdcontrol_core::agent::{load_agent_metadata, save_agent_metadata};
    use crowdcontrol_core::{Agent, AgentStatus};

    let (config, _temp_dir) = create_test_config();

    // Create an agent with project:label format
    let workspace_path = config.workspaces_dir.join("myapp").join("main");
    fs::create_dir_all(&workspace_path).unwrap();

    let agent = Agent {
        name: "myapp-main".to_string(), // filesystem name
        status: AgentStatus::Created,
        container_id: Some("abc123".to_string()),
        repository: "git@github.com:org/myapp.git".to_string(),
        branch: Some("main".to_string()),
        created_at: Utc::now(),
        workspace_path: workspace_path.clone(),
        project_slug: Some("myapp".to_string()),
    };

    save_agent_metadata(&config, &agent).unwrap();

    // Should be able to load using colon format
    let loaded = load_agent_metadata(&config, "myapp:main").unwrap();
    assert_eq!(loaded.name, "myapp-main");
    assert_eq!(loaded.project_slug, Some("myapp".to_string()));
}

/// Test load_agent_metadata works with hyphen format
#[test]
fn test_load_agent_metadata_with_hyphen_format() {
    use chrono::Utc;
    use crowdcontrol_core::agent::{load_agent_metadata, save_agent_metadata};
    use crowdcontrol_core::{Agent, AgentStatus};

    let (config, _temp_dir) = create_test_config();

    // Create an agent with project:label format
    let workspace_path = config.workspaces_dir.join("myapp").join("main");
    fs::create_dir_all(&workspace_path).unwrap();

    let agent = Agent {
        name: "myapp-main".to_string(), // filesystem name
        status: AgentStatus::Created,
        container_id: Some("abc123".to_string()),
        repository: "git@github.com:org/myapp.git".to_string(),
        branch: Some("main".to_string()),
        created_at: Utc::now(),
        workspace_path: workspace_path.clone(),
        project_slug: Some("myapp".to_string()),
    };

    save_agent_metadata(&config, &agent).unwrap();

    // Should also be able to load using hyphen format
    let loaded = load_agent_metadata(&config, "myapp-main").unwrap();
    assert_eq!(loaded.name, "myapp-main");
    assert_eq!(loaded.project_slug, Some("myapp".to_string()));
}
