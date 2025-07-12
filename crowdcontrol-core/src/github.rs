use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub installation_token: Option<String>,
    pub app_id: Option<String>,
    pub installation_id: Option<String>,
    pub private_key_path: Option<PathBuf>,
}

impl GitHubConfig {
    /// Create a new GitHub config with an installation token
    pub fn new_with_installation_token(token: String) -> Self {
        Self {
            installation_token: Some(token),
            app_id: None,
            installation_id: None,
            private_key_path: None,
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

        // Return Some if any GitHub config is present
        if installation_token.is_some() || app_id.is_some() {
            Some(Self {
                installation_token,
                app_id,
                installation_id,
                private_key_path,
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
            }
            _ => {
                return Err(anyhow!(
                    "GitHub config must have either installation_token or (app_id + installation_id)"
                ));
            }
        }

        Ok(())
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

        // Set CrowdControl as the git committer
        env_vars.push("GITHUB_USER_NAME=CrowdControl[bot]".to_string());
        env_vars.push("GITHUB_USER_EMAIL=crowdcontrol[bot]@users.noreply.github.com".to_string());

        env_vars
    }

    /// Get git configuration commands for container setup
    pub fn get_git_config_commands(&self) -> Vec<String> {
        vec![
            // Configure git to use HTTPS instead of SSH
            "git config --global url.\"https://github.com/\".insteadOf \"git@github.com:\"".to_string(),
            // Set up credential helper
            "git config --global credential.helper store".to_string(),
            // Set CrowdControl as committer
            "git config --global user.name \"CrowdControl[bot]\"".to_string(),
            "git config --global user.email \"crowdcontrol[bot]@users.noreply.github.com\"".to_string(),
        ]
    }
}