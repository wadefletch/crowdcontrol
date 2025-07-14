use crowdcontrol_core::github::{GitHubConfig, GitHubCredentialManager, GitHubCredentialTemplate, CachedToken};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

#[test]
fn test_cached_token_lifecycle() {
    // Test token creation and expiration
    let token = CachedToken::new("test_token".to_string(), 1); // 1 minute
    
    assert_eq!(token.token, "test_token");
    assert!(!token.is_expired());
    assert!(token.expires_in().as_secs() > 50); // Should be close to 60 seconds
    
    // Test token with immediate expiration
    let expired_token = CachedToken {
        token: "expired".to_string(),
        expires_at: SystemTime::now() - Duration::from_secs(1),
    };
    
    assert!(expired_token.is_expired());
    assert_eq!(expired_token.expires_in(), Duration::ZERO);
}

#[test]
fn test_github_config_with_app_credentials() {
    let temp_dir = TempDir::new().unwrap();
    let key_path = temp_dir.path().join("test-key.pem");
    std::fs::write(&key_path, "fake key content").unwrap();
    
    let config = GitHubConfig::new_with_app_credentials(
        "123456".to_string(),
        "789012".to_string(),
        key_path.clone(),
        Some("https://github.enterprise.com".to_string()),
    );
    
    assert_eq!(config.app_id, Some("123456".to_string()));
    assert_eq!(config.installation_id, Some("789012".to_string()));
    assert_eq!(config.private_key_path, Some(key_path));
    assert_eq!(config.base_url, Some("https://github.enterprise.com".to_string()));
    assert!(config.validate().is_ok());
}

#[test]
fn test_github_enterprise_base_url() {
    let config = GitHubConfig::new_with_app_credentials(
        "123456".to_string(),
        "789012".to_string(),
        PathBuf::from("/fake/path"), // Won't validate file existence in this test
        Some("https://github.enterprise.com".to_string()),
    );
    
    assert_eq!(config.github_base_url(), "https://github.enterprise.com");
    
    // Test default base URL
    let default_config = GitHubConfig::new_with_installation_token("ghs_test".to_string());
    assert_eq!(default_config.github_base_url(), "https://github.com");
}

#[test]
fn test_github_enterprise_git_commands() {
    let config = GitHubConfig::new_with_app_credentials(
        "123456".to_string(),
        "789012".to_string(),
        PathBuf::from("/fake/path"),
        Some("https://github.enterprise.com".to_string()),
    );
    
    let commands = config.get_git_config_commands();
    
    // Should configure for enterprise GitHub
    assert!(commands.iter().any(|cmd| cmd.contains("github.enterprise.com")));
    assert!(commands.iter().any(|cmd| cmd.contains("CrowdControl[bot]")));
}

#[test]
fn test_github_config_validation_enhancements() {
    // Test invalid app ID (non-numeric)
    let mut config = GitHubConfig::new_with_installation_token("".to_string());
    config.installation_token = None;
    config.app_id = Some("not-a-number".to_string());
    config.installation_id = Some("123456".to_string());
    
    assert!(config.validate().is_err());
    
    // Test invalid installation ID (non-numeric)
    config.app_id = Some("123456".to_string());
    config.installation_id = Some("not-a-number".to_string());
    
    assert!(config.validate().is_err());
    
    // Test invalid base URL
    config.app_id = Some("123456".to_string());
    config.installation_id = Some("789012".to_string());
    config.base_url = Some("http://insecure.com".to_string()); // Should be HTTPS
    
    assert!(config.validate().is_err());
}

#[test]
fn test_github_env_vars_with_base_url() {
    std::env::set_var("GITHUB_INSTALLATION_TOKEN", "ghs_test_token");
    std::env::set_var("GITHUB_BASE_URL", "https://github.enterprise.com");
    
    let config = GitHubConfig::from_env().unwrap();
    let env_vars = config.to_container_env_vars();
    
    assert!(env_vars.contains(&"GITHUB_INSTALLATION_TOKEN=ghs_test_token".to_string()));
    assert!(env_vars.contains(&"GITHUB_BASE_URL=https://github.enterprise.com".to_string()));
    
    std::env::remove_var("GITHUB_INSTALLATION_TOKEN");
    std::env::remove_var("GITHUB_BASE_URL");
}

#[test]
fn test_credential_template_creation() {
    let config = GitHubConfig::new_with_installation_token("ghs_test".to_string());
    
    let template = GitHubCredentialTemplate::new_organization_template(
        "acme-corp".to_string(),
        "acme-corp".to_string(),
        config,
        None,
    );
    
    assert_eq!(template.name, "acme-corp");
    assert_eq!(template.url_pattern, "https://github.com/acme-corp/*");
    assert!(template.description.is_some());
}

#[test]
fn test_credential_template_url_matching() {
    let config = GitHubConfig::new_with_installation_token("ghs_test".to_string());
    
    let template = GitHubCredentialTemplate::new_organization_template(
        "acme-corp".to_string(),
        "acme-corp".to_string(),
        config,
        None,
    );
    
    // Should match organization repositories
    assert!(template.matches_url("https://github.com/acme-corp/repo1"));
    assert!(template.matches_url("https://github.com/acme-corp/some-project"));
    
    // Should not match other organizations
    assert!(!template.matches_url("https://github.com/other-org/repo"));
    assert!(!template.matches_url("https://github.com/acme-corp-fake/repo"));
}

#[test]
fn test_credential_manager() {
    let mut manager = GitHubCredentialManager::new();
    
    // Add default config
    let default_config = GitHubConfig::new_with_installation_token("ghs_default".to_string());
    manager.default_config = Some(default_config);
    
    // Add organization template
    let org_config = GitHubConfig::new_with_installation_token("ghs_org".to_string());
    let template = GitHubCredentialTemplate::new_organization_template(
        "acme-corp".to_string(),
        "acme-corp".to_string(),
        org_config,
        None,
    );
    manager.add_template(template);
    
    // Test template matching
    let config = manager.get_config_for_url("https://github.com/acme-corp/repo");
    assert!(config.is_some());
    assert_eq!(config.unwrap().installation_token.as_ref().unwrap(), "ghs_org");
    
    // Test fallback to default
    let config = manager.get_config_for_url("https://github.com/other-org/repo");
    assert!(config.is_some());
    assert_eq!(config.unwrap().installation_token.as_ref().unwrap(), "ghs_default");
    
    // Test no config found
    manager.default_config = None;
    let config = manager.get_config_for_url("https://github.com/other-org/repo");
    assert!(config.is_none());
}

#[test]
fn test_credential_manager_from_env() {
    std::env::set_var("GITHUB_INSTALLATION_TOKEN", "ghs_from_env");
    
    let manager = GitHubCredentialManager::from_env_and_config();
    assert!(manager.default_config.is_some());
    assert_eq!(
        manager.default_config.unwrap().installation_token.unwrap(),
        "ghs_from_env"
    );
    
    std::env::remove_var("GITHUB_INSTALLATION_TOKEN");
}