use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

/// Helper to generate unique test identifiers with 8-character UUID suffix
pub fn generate_test_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4().to_string()[0..8].to_string())
}

/// Helper to create a basic config file with proper workspaces directory
pub fn create_config_file(dir: &TempDir) -> std::path::PathBuf {
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

/// Helper to create a config file with optional GitHub configuration
pub fn create_config_with_github(dir: &TempDir, include_github: bool) -> std::path::PathBuf {
    let config_path = dir.path().join("config.toml");
    let workspaces_dir = dir.path().join("workspaces");
    
    // Ensure workspaces directory exists
    fs::create_dir_all(&workspaces_dir).unwrap();
    
    let mut config_content = format!(
        r#"
workspaces_dir = "{}"
image = "crowdcontrol:latest"
"#,
        workspaces_dir.display()
    );

    if include_github {
        use std::env;
        if let Ok(token) = env::var("GITHUB_INSTALLATION_TOKEN") {
            config_content.push_str(&format!(
                r#"
[github]
installation_token = "{}"
"#,
                token
            ));
        } else if let (Ok(app_id), Ok(private_key)) = (
            env::var("GITHUB_APP_ID"),
            env::var("GITHUB_APP_PRIVATE_KEY"),
        ) {
            config_content.push_str(&format!(
                r#"
[github]
app_id = {}
private_key = "{}"
"#,
                app_id, private_key
            ));
        }
    }

    fs::write(&config_path, config_content).unwrap();
    config_path
}

/// Helper to create a config file with resource limits
pub fn create_config_with_limits(dir: &TempDir, memory: &str, cpus: &str) -> std::path::PathBuf {
    let config_path = dir.path().join("config.toml");
    let workspaces_dir = dir.path().join("workspaces");
    
    // Ensure workspaces directory exists
    fs::create_dir_all(&workspaces_dir).unwrap();
    
    let config_content = format!(
        r#"
workspaces_dir = "{}"
image = "crowdcontrol:latest"
default_memory = "{}"
default_cpus = "{}"
"#,
        workspaces_dir.display(),
        memory,
        cpus
    );
    fs::write(&config_path, config_content).unwrap();
    config_path
}