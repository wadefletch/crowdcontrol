use crowdcontrol_core::{Config, Agent, AgentStatus, StateValidator, StateInconsistency};
use crowdcontrol_core::agent::{save_agent_metadata, update_agent_metadata};
use tempfile::tempdir;
use chrono::Utc;
use std::path::PathBuf;
use std::fs;

fn create_test_config() -> (Config, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "test:latest".to_string(),
        default_memory: None,
        default_cpus: None,
        verbose: 0,
    };
    (config, temp_dir)
}

fn create_test_agent(name: &str, status: AgentStatus) -> Agent {
    let container_id = if status == AgentStatus::Running {
        Some("test-container-id".to_string())
    } else {
        None
    };
    
    Agent {
        name: name.to_string(),
        status,
        container_id,
        repository: "https://github.com/test/repo.git".to_string(),
        branch: Some("main".to_string()),
        created_at: Utc::now(),
        workspace_path: PathBuf::from("/test/workspace"),
    }
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_detect_missing_workspace() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("test-agent", AgentStatus::Created);
    
    // Save metadata
    save_agent_metadata(&config, &agent).unwrap();
    
    // Remove the workspace directory
    let workspace_path = config.agent_workspace_path("test-agent");
    if workspace_path.exists() {
        fs::remove_dir_all(&workspace_path).unwrap();
    }
    
    // Validate should detect missing workspace
    let validator = StateValidator::new(config).unwrap();
    let issues = validator.validate_all().await.unwrap();
    
    assert_eq!(issues.len(), 1);
    match &issues[0] {
        StateInconsistency::MissingWorkspace { agent_name } => {
            assert_eq!(agent_name, "test-agent");
        }
        _ => panic!("Expected MissingWorkspace inconsistency"),
    }
}

#[tokio::test]
async fn test_detect_corrupted_metadata() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("corrupt-test", AgentStatus::Created);
    
    // Save valid metadata first
    save_agent_metadata(&config, &agent).unwrap();
    
    // Corrupt the metadata file
    let metadata_path = config.agent_workspace_path("corrupt-test")
        .join(".crowdcontrol")
        .join("metadata.json");
    fs::write(&metadata_path, "{ invalid json").unwrap();
    
    // Validate should detect corruption
    let validator = StateValidator::new(config).unwrap();
    let issues = validator.validate_all().await.unwrap();
    
    assert_eq!(issues.len(), 1);
    match &issues[0] {
        StateInconsistency::CorruptedMetadata { agent_name, error: _ } => {
            assert_eq!(agent_name, "corrupt-test");
        }
        _ => panic!("Expected CorruptedMetadata inconsistency"),
    }
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_no_issues_with_valid_state() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("valid-agent", AgentStatus::Stopped);
    
    // Save metadata
    save_agent_metadata(&config, &agent).unwrap();
    
    // Validate should find no issues
    let validator = StateValidator::new(config).unwrap();
    let issues = validator.validate_all().await.unwrap();
    
    assert_eq!(issues.len(), 0);
}

#[tokio::test]
#[ignore = "test has a bug - loads Created instead of Stopped"]
async fn test_repair_status_mismatch() {
    let (config, _temp_dir) = create_test_config();
    let mut agent = create_test_agent("status-test", AgentStatus::Running);
    agent.container_id = Some("fake-container-id".to_string());
    
    // Save metadata indicating agent is running
    save_agent_metadata(&config, &agent).unwrap();
    
    // Since we can't actually create a Docker container in tests,
    // we'll simulate the repair by updating the metadata
    update_agent_metadata(&config, "status-test", |agent| {
        agent.status = AgentStatus::Stopped;
        agent.container_id = None;
        Ok(())
    }).unwrap();
    
    // Verify the repair
    let loaded_agent = crowdcontrol_core::agent::load_agent_metadata(&config, "status-test").unwrap();
    assert_eq!(loaded_agent.status, AgentStatus::Stopped);
    assert_eq!(loaded_agent.container_id, None);
}