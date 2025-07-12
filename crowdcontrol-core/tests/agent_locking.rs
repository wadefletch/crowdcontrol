use crowdcontrol_core::{agent::*, Config, Agent, AgentStatus};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::tempdir;

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

fn create_test_agent(name: &str) -> Agent {
    Agent {
        name: name.to_string(),
        status: AgentStatus::Created,
        container_id: Some("test-container-id".to_string()),
        repository: "https://github.com/test/repo.git".to_string(),
        branch: Some("main".to_string()),
        created_at: Utc::now(),
        workspace_path: PathBuf::from("/test/workspace"),
    }
}

#[test]
fn test_save_and_load_metadata() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("test-agent");

    // Save metadata
    save_agent_metadata(&config, &agent).unwrap();

    // Load metadata
    let loaded_agent = load_agent_metadata(&config, "test-agent").unwrap();

    assert_eq!(loaded_agent.name, agent.name);
    assert_eq!(loaded_agent.repository, agent.repository);
    assert_eq!(loaded_agent.branch, agent.branch);
    assert_eq!(loaded_agent.container_id, agent.container_id);
}

#[test]
fn test_metadata_has_comment() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("test-agent");

    // Save metadata
    save_agent_metadata(&config, &agent).unwrap();

    // Read the raw JSON to verify comment field
    let metadata_path = config.agent_workspace_path("test-agent")
        .join(".crowdcontrol")
        .join("metadata.json");
    
    let content = std::fs::read_to_string(metadata_path).unwrap();
    assert!(content.contains("\"_comment\":"));
    assert!(content.contains("auto-generated"));
}

#[test]
fn test_concurrent_reads() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("concurrent-read-test");
    
    // Save initial metadata
    save_agent_metadata(&config, &agent).unwrap();

    let config = Arc::new(config);
    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    // Spawn multiple threads to read simultaneously
    for i in 0..5 {
        let config = Arc::clone(&config);
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            barrier.wait();
            
            // All threads try to read at the same time
            let result = load_agent_metadata(&config, "concurrent-read-test");
            
            // Verify the read was successful
            let agent = result.expect(&format!("Thread {} failed to read", i));
            assert_eq!(agent.name, "concurrent-read-test");
            assert_eq!(agent.repository, "https://github.com/test/repo.git");
        });
        
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_writes_are_serialized() {
    let (config, _temp_dir) = create_test_config();
    let config = Arc::new(config);
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    // Create different agents to write
    for i in 0..3 {
        let config = Arc::clone(&config);
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            let agent = Agent {
                name: format!("write-test-{}", i),
                status: AgentStatus::Created,
                container_id: Some(format!("container-{}", i)),
                repository: format!("https://github.com/test/repo{}.git", i),
                branch: Some("main".to_string()),
                created_at: Utc::now(),
                workspace_path: PathBuf::from(format!("/test/workspace{}", i)),
            };
            
            barrier.wait();
            
            // All threads try to write at the same time
            save_agent_metadata(&config, &agent).unwrap();
        });
        
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all agents were saved correctly
    for i in 0..3 {
        let agent = load_agent_metadata(&config, &format!("write-test-{}", i)).unwrap();
        assert_eq!(agent.repository, format!("https://github.com/test/repo{}.git", i));
        assert_eq!(agent.container_id, Some(format!("container-{}", i)));
    }
}

#[test]
fn test_atomic_update() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("update-test");
    
    // Save initial metadata
    save_agent_metadata(&config, &agent).unwrap();

    // Update metadata atomically
    update_agent_metadata(&config, "update-test", |agent| {
        agent.container_id = Some("new-container-id".to_string());
        agent.status = AgentStatus::Running;
        Ok(())
    }).unwrap();

    // Verify the update
    let updated_agent = load_agent_metadata(&config, "update-test").unwrap();
    assert_eq!(updated_agent.container_id, Some("new-container-id".to_string()));
}

#[test]
fn test_concurrent_updates() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("concurrent-update-test");
    
    // Save initial metadata
    save_agent_metadata(&config, &agent).unwrap();

    let config = Arc::new(config);
    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];

    // Spawn multiple threads to update simultaneously
    for i in 0..5 {
        let config = Arc::clone(&config);
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            barrier.wait();
            
            // Each thread tries to update the container_id
            update_agent_metadata(&config, "concurrent-update-test", |agent| {
                agent.container_id = Some(format!("container-from-thread-{}", i));
                Ok(())
            }).unwrap();
        });
        
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify that one of the updates succeeded and the data is consistent
    let final_agent = load_agent_metadata(&config, "concurrent-update-test").unwrap();
    assert!(final_agent.container_id.is_some());
    assert!(final_agent.container_id.unwrap().starts_with("container-from-thread-"));
}

#[test]
fn test_update_nonexistent_agent() {
    let (config, _temp_dir) = create_test_config();
    
    // Try to update a non-existent agent
    let result = update_agent_metadata(&config, "nonexistent", |agent| {
        agent.container_id = Some("should-not-work".to_string());
        Ok(())
    });
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_list_all_agents() {
    let (config, _temp_dir) = create_test_config();
    
    // Create multiple agents
    for i in 0..3 {
        let agent = create_test_agent(&format!("list-test-{}", i));
        save_agent_metadata(&config, &agent).unwrap();
    }
    
    // List all agents
    let agents = list_all_agents(&config).unwrap();
    
    assert_eq!(agents.len(), 3);
    assert!(agents.contains(&"list-test-0".to_string()));
    assert!(agents.contains(&"list-test-1".to_string()));
    assert!(agents.contains(&"list-test-2".to_string()));
}

#[test]
fn test_mixed_concurrent_operations() {
    let (config, _temp_dir) = create_test_config();
    let agent = create_test_agent("mixed-ops-test");
    
    // Save initial metadata
    save_agent_metadata(&config, &agent).unwrap();

    let config = Arc::new(config);
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    // Mix of readers and writers
    for i in 0..10 {
        let config = Arc::clone(&config);
        let barrier = Arc::clone(&barrier);
        
        let handle = if i % 2 == 0 {
            // Reader thread
            thread::spawn(move || {
                barrier.wait();
                
                let agent = load_agent_metadata(&config, "mixed-ops-test").unwrap();
                assert_eq!(agent.name, "mixed-ops-test");
            })
        } else {
            // Writer thread
            thread::spawn(move || {
                barrier.wait();
                
                update_agent_metadata(&config, "mixed-ops-test", |agent| {
                    agent.container_id = Some(format!("updated-by-thread-{}", i));
                    Ok(())
                }).unwrap();
            })
        };
        
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify final state is consistent
    let final_agent = load_agent_metadata(&config, "mixed-ops-test").unwrap();
    assert!(final_agent.container_id.is_some());
}