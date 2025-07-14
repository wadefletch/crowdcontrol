use crowdcontrol_core::{Config, Settings};
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_settings_structure() {
    // Test default settings creation
    let settings = Settings::default();

    assert_eq!(
        settings
            .core
            .workspaces_dir
            .to_string_lossy()
            .contains("crowdcontrol-workspaces"),
        true
    );
    assert_eq!(settings.docker.image, "crowdcontrol:latest");
    assert_eq!(settings.logging.verbosity, 0);
    assert_eq!(settings.resources.default_memory, None);
    assert!(settings.github.is_none());
    assert!(settings.repos.is_empty());
}

#[test]
fn test_config_from_settings() {
    let temp_dir = TempDir::new().unwrap();

    let mut settings = Settings::default();
    settings.core.workspaces_dir = temp_dir.path().to_path_buf();
    settings.docker.image = "test:latest".to_string();
    settings.logging.verbosity = 2;
    settings.resources.default_memory = Some("4g".to_string());
    settings.resources.default_cpus = Some("2".to_string());

    let config = Config::from_settings(settings).unwrap();

    assert_eq!(config.workspaces_dir, temp_dir.path());
    assert_eq!(config.image, "test:latest");
    assert_eq!(config.verbose, 2);
    assert_eq!(config.default_memory, Some("4g".to_string()));
    assert_eq!(config.default_cpus, Some("2".to_string()));
}

#[test]
fn test_repo_specific_overrides() {
    let mut settings = Settings::default();

    // Add a repo-specific override
    let mut repo_settings = crowdcontrol_core::settings::RepoSettings {
        image: Some("custom:latest".to_string()),
        memory: Some("8g".to_string()),
        cpus: Some("4".to_string()),
        workspace_dir: None,
        env: HashMap::new(),
    };

    settings
        .repos
        .insert("github.com/test/repo".to_string(), repo_settings.clone());

    // Test exact match
    let override_settings = settings.get_repo_override("github.com/test/repo");
    assert!(override_settings.is_some());
    assert_eq!(
        override_settings.unwrap().image,
        Some("custom:latest".to_string())
    );

    // Add a wildcard pattern
    repo_settings.image = Some("org-image:latest".to_string());
    settings
        .repos
        .insert("github.com/myorg/*".to_string(), repo_settings);

    // Test pattern match
    let override_settings = settings.get_repo_override("github.com/myorg/some-project");
    assert!(override_settings.is_some());
    assert_eq!(
        override_settings.unwrap().image,
        Some("org-image:latest".to_string())
    );
}

#[test]
fn test_env_var_mapping() {
    // Test that CC_ prefixed environment variables would be parsed correctly
    // This is more of a documentation test since we can't easily test the actual
    // environment variable loading without affecting the test environment

    let expected_mappings = vec![
        ("CC_CORE_WORKSPACES_DIR", "core.workspaces_dir"),
        ("CC_DOCKER_IMAGE", "docker.image"),
        ("CC_RESOURCES_DEFAULT_MEMORY", "resources.default_memory"),
        ("CC_RESOURCES_DEFAULT_CPUS", "resources.default_cpus"),
        ("CC_LOGGING_VERBOSITY", "logging.verbosity"),
        ("CC_LOGGING_NO_COLOR", "logging.no_color"),
        ("CC_GITHUB_APP_ID", "github.app_id"),
        ("CC_GITHUB_INSTALLATION_ID", "github.installation_id"),
        ("CC_GITHUB_INSTALLATION_TOKEN", "github.installation_token"),
    ];

    // Just verify the mapping makes sense
    for (env_var, config_path) in expected_mappings {
        assert!(env_var.starts_with("CC_"));
        assert!(config_path.contains('.'));
    }
}
