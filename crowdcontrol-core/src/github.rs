use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tracing::{debug, trace, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub installation_token: Option<String>,
    pub app_id: Option<String>,
    pub installation_id: Option<String>,
    pub private_key_path: Option<PathBuf>,
    pub base_url: Option<String>, // Support for GitHub Enterprise
    #[serde(skip)] // Don't serialize cached data
    pub cached_token: Option<CachedToken>,
}

#[derive(Debug, Clone)]
pub struct CachedToken {
    pub token: String,
    pub expires_at: SystemTime,
}

impl CachedToken {
    pub fn new(token: String, lifetime_minutes: u64) -> Self {
        let expires_at = SystemTime::now() + Duration::from_secs(lifetime_minutes * 60);
        Self { token, expires_at }
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now() >= self.expires_at
    }

    pub fn expires_in(&self) -> Duration {
        self.expires_at.duration_since(SystemTime::now()).unwrap_or(Duration::ZERO)
    }
}

impl GitHubConfig {
    /// Create a new GitHub config with an installation token
    pub fn new_with_installation_token(token: String) -> Self {
        Self {
            installation_token: Some(token),
            app_id: None,
            installation_id: None,
            private_key_path: None,
            base_url: None,
            cached_token: None,
        }
    }

    /// Create a new GitHub config for an app (for organization-wide sharing)
    pub fn new_with_app_credentials(
        app_id: String,
        installation_id: String,
        private_key_path: PathBuf,
        base_url: Option<String>,
    ) -> Self {
        Self {
            installation_token: None,
            app_id: Some(app_id),
            installation_id: Some(installation_id),
            private_key_path: Some(private_key_path),
            base_url,
            cached_token: None,
        }
    }

    /// Create GitHub config from environment variables
    pub fn from_env() -> Option<Self> {
        let installation_token = env::var("GITHUB_INSTALLATION_TOKEN").ok();
        let app_id = env::var("GITHUB_APP_ID").ok();
        let installation_id = env::var("GITHUB_INSTALLATION_ID").ok();
        let private_key_path = env::var("GITHUB_PRIVATE_KEY_PATH")
            .ok()
            .map(PathBuf::from);
        let base_url = env::var("GITHUB_BASE_URL").ok();

        // Return Some if any GitHub config is present
        if installation_token.is_some() || app_id.is_some() {
            Some(Self {
                installation_token,
                app_id,
                installation_id,
                private_key_path,
                base_url,
                cached_token: None,
            })
        } else {
            None
        }
    }

    /// Validate the GitHub configuration
    pub fn validate(&self) -> Result<()> {
        // Must have either installation token or app credentials
        match (&self.installation_token, &self.app_id, &self.installation_id) {
            (Some(token), _, _) => {
                if token.is_empty() {
                    return Err(anyhow!("GitHub installation token cannot be empty"));
                }
                if !token.starts_with("ghs_") {
                    return Err(anyhow!("GitHub installation token must start with 'ghs_'"));
                }
            }
            (None, Some(app_id), Some(installation_id)) => {
                if app_id.is_empty() || installation_id.is_empty() {
                    return Err(anyhow!("GitHub app ID and installation ID cannot be empty"));
                }
                // Validate app_id is numeric
                if app_id.parse::<u64>().is_err() {
                    return Err(anyhow!("GitHub app ID must be a number"));
                }
                // Validate installation_id is numeric
                if installation_id.parse::<u64>().is_err() {
                    return Err(anyhow!("GitHub installation ID must be a number"));
                }
                // Check private key file exists if app-based auth
                if let Some(key_path) = &self.private_key_path {
                    if !key_path.exists() {
                        return Err(anyhow!("GitHub private key file not found: {:?}", key_path));
                    }
                }
            }
            _ => {
                return Err(anyhow!(
                    "GitHub config must have either installation_token or (app_id + installation_id)"
                ));
            }
        }

        // Validate base URL if provided
        if let Some(base_url) = &self.base_url {
            if !base_url.starts_with("https://") {
                return Err(anyhow!("GitHub base URL must start with https://"));
            }
        }

        Ok(())
    }

    /// Get the effective GitHub base URL (defaults to github.com)
    pub fn github_base_url(&self) -> String {
        self.base_url.clone().unwrap_or_else(|| "https://github.com".to_string())
    }

    /// Get an installation token, using cached token if available and not expired
    pub fn get_installation_token(&mut self) -> Result<String> {
        // Check if we have a cached token that's not expired
        if let Some(cached) = &self.cached_token {
            if !cached.is_expired() {
                trace!(
                    "Using cached GitHub token, expires in {:?}",
                    cached.expires_in()
                );
                return Ok(cached.token.clone());
            } else {
                debug!("Cached GitHub token expired, will refresh");
                self.cached_token = None;
            }
        }

        // Use direct token if available
        if let Some(token) = &self.installation_token {
            debug!("Using direct installation token");
            return Ok(token.clone());
        }

        // For app-based auth, we'd generate a JWT and fetch installation token here
        // This is a placeholder for future JWT implementation
        if self.app_id.is_some() && self.installation_id.is_some() {
            warn!("JWT-based token generation not yet implemented");
            return Err(anyhow!("JWT-based token generation not yet implemented"));
        }

        Err(anyhow!("No valid GitHub authentication method available"))
    }

    /// Cache an installation token for future use
    pub fn cache_token(&mut self, token: String, lifetime_minutes: u64) {
        debug!("Caching GitHub token for {} minutes", lifetime_minutes);
        self.cached_token = Some(CachedToken::new(token, lifetime_minutes));
    }

    /// Convert to container environment variables
    pub fn to_container_env_vars(&self) -> Vec<String> {
        let mut env_vars = Vec::new();

        if let Some(token) = &self.installation_token {
            env_vars.push(format!("GITHUB_INSTALLATION_TOKEN={}", token));
        }

        if let Some(app_id) = &self.app_id {
            env_vars.push(format!("GITHUB_APP_ID={}", app_id));
        }

        if let Some(installation_id) = &self.installation_id {
            env_vars.push(format!("GITHUB_INSTALLATION_ID={}", installation_id));
        }

        if let Some(base_url) = &self.base_url {
            env_vars.push(format!("GITHUB_BASE_URL={}", base_url));
        }

        // Set CrowdControl as the git committer
        env_vars.push("GITHUB_USER_NAME=CrowdControl[bot]".to_string());
        env_vars.push("GITHUB_USER_EMAIL=crowdcontrol[bot]@users.noreply.github.com".to_string());

        env_vars
    }

    /// Get git configuration commands for container setup
    pub fn get_git_config_commands(&self) -> Vec<String> {
        let base_url = self.github_base_url();
        let base_host = base_url.trim_start_matches("https://");
        
        vec![
            // Configure git to use HTTPS instead of SSH (supports GitHub Enterprise)
            format!("git config --global url.\"{}//\".insteadOf \"git@{}:\"", base_url, base_host),
            // Set up credential helper
            "git config --global credential.helper store".to_string(),
            // Set CrowdControl as committer
            "git config --global user.name \"CrowdControl[bot]\"".to_string(),
            "git config --global user.email \"crowdcontrol[bot]@users.noreply.github.com\"".to_string(),
        ]
    }
}

/// Organization-wide GitHub credential template
/// Inspired by ArgoCD's repository credential templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCredentialTemplate {
    pub name: String,
    pub description: Option<String>,
    pub config: GitHubConfig,
    /// URL pattern this template applies to (e.g., "https://github.com/myorg/*")
    pub url_pattern: String,
}

impl GitHubCredentialTemplate {
    /// Create a new credential template for an organization
    pub fn new_organization_template(
        name: String,
        organization: String,
        config: GitHubConfig,
        base_url: Option<String>,
    ) -> Self {
        let base = base_url.unwrap_or_else(|| "https://github.com".to_string());
        let url_pattern = format!("{}/{}/*", base, organization);
        
        Self {
            name: name.clone(),
            description: Some(format!("GitHub App credentials for {} organization", organization)),
            config,
            url_pattern,
        }
    }

    /// Check if this template applies to a given repository URL
    pub fn matches_url(&self, url: &str) -> bool {
        // Simple pattern matching for now - could be enhanced with regex
        let pattern = self.url_pattern.replace("*", "");
        url.starts_with(&pattern)
    }
}

/// Configuration manager for GitHub credentials
/// Supports both individual configs and organization-wide templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCredentialManager {
    pub templates: Vec<GitHubCredentialTemplate>,
    pub default_config: Option<GitHubConfig>,
}

impl GitHubCredentialManager {
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
            default_config: None,
        }
    }

    /// Add a credential template
    pub fn add_template(&mut self, template: GitHubCredentialTemplate) {
        debug!("Adding GitHub credential template: {}", template.name);
        self.templates.push(template);
    }

    /// Find the best matching credential config for a repository URL
    pub fn get_config_for_url(&self, url: &str) -> Option<&GitHubConfig> {
        // First, try to find a matching template
        for template in &self.templates {
            if template.matches_url(url) {
                debug!("Using template '{}' for URL: {}", template.name, url);
                return Some(&template.config);
            }
        }

        // Fall back to default config
        if let Some(default) = &self.default_config {
            debug!("Using default GitHub config for URL: {}", url);
            Some(default)
        } else {
            debug!("No GitHub config found for URL: {}", url);
            None
        }
    }

    /// Load credential manager from environment and config files
    pub fn from_env_and_config() -> Self {
        let mut manager = Self::new();

        // Load default config from environment
        if let Some(config) = GitHubConfig::from_env() {
            manager.default_config = Some(config);
        }

        // TODO: Load templates from config files
        // This could read from ~/.config/crowdcontrol/github-templates.toml

        manager
    }
}