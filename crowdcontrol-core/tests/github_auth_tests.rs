use crowdcontrol_core::config::Config;
use crowdcontrol_core::github::GitHubConfig;
use tempfile::TempDir;

#[test]
fn test_github_config_creation() {
    // Test creating GitHub config with installation token
    let config = GitHubConfig::new_with_installation_token("ghs_test_token".to_string());
    
    assert_eq!(config.installation_token, Some("ghs_test_token".to_string()));
    assert_eq!(config.app_id, None);
    assert_eq!(config.installation_id, None);
    assert_eq!(config.private_key_path, None);
}

#[test]
fn test_github_config_validation() {
    // Test that GitHub config validates required fields
    let valid_config = GitHubConfig::new_with_installation_token("ghs_valid_token".to_string());
    assert!(valid_config.validate().is_ok());
    
    // Test empty token fails validation
    let invalid_config = GitHubConfig::new_with_installation_token("".to_string());
    assert!(invalid_config.validate().is_err());
    
    // Test invalid token format fails validation
    let invalid_format = GitHubConfig::new_with_installation_token("invalid_token".to_string());
    assert!(invalid_format.validate().is_err());
}

#[test]
fn test_github_config_environment_variables() {
    // Test that GitHub config can be created from environment variables
    std::env::set_var("GITHUB_INSTALLATION_TOKEN", "ghs_env_token");
    
    let config = GitHubConfig::from_env().unwrap();
    assert_eq!(config.installation_token, Some("ghs_env_token".to_string()));
    
    std::env::remove_var("GITHUB_INSTALLATION_TOKEN");
}

#[test]
fn test_github_config_missing_env_vars() {
    // Test that missing environment variables return None
    std::env::remove_var("GITHUB_INSTALLATION_TOKEN");
    std::env::remove_var("GITHUB_APP_ID");
    std::env::remove_var("GITHUB_INSTALLATION_ID");
    
    let config = GitHubConfig::from_env();
    assert!(config.is_none());
}

#[test]
fn test_github_config_container_env_vars() {
    let config = GitHubConfig::new_with_installation_token("ghs_test_token".to_string());
    
    let env_vars = config.to_container_env_vars();
    
    assert!(env_vars.contains(&"GITHUB_INSTALLATION_TOKEN=ghs_test_token".to_string()));
    assert!(env_vars.iter().any(|var| var.starts_with("GITHUB_USER_NAME=")));
    assert!(env_vars.iter().any(|var| var.starts_with("GITHUB_USER_EMAIL=")));
}

#[test]
fn test_config_with_github_settings() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test that Config can include GitHub settings
    let github_config = GitHubConfig::new_with_installation_token("ghs_test_token".to_string());
    
    let config = Config {
        workspaces_dir: temp_dir.path().to_path_buf(),
        image: "test:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
        github: Some(github_config),
    };
    
    assert!(config.github.is_some());
    assert_eq!(
        config.github.unwrap().installation_token,
        Some("ghs_test_token".to_string())
    );
}