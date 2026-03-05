use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(test)]
mod tests;
pub mod test_helpers;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub workspaces_dir: PathBuf,
    pub image: String,
    pub verbose: u8,
    
    pub default_memory: Option<String>,
    pub default_cpus: Option<String>,
    
    pub docker: Docker,
    pub github: Option<Github>,
    pub logging: Logging,
    
    #[serde(default)]
    pub repos: HashMap<String, RepoConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Docker {
    #[serde(default = "default_docker_host")]
    pub host: Option<String>,
    pub default_memory: Option<String>,
    pub default_cpus: Option<String>,
}

fn default_docker_host() -> Option<String> {
    None
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Github {
    pub installation_token: Option<String>,
    pub app_id: Option<String>,
    pub installation_id: Option<String>,
    pub private_key_path: Option<PathBuf>,
    pub base_url: Option<String>,
    
    #[serde(skip)]
    pub cached_token: Option<CachedToken>,
}

#[derive(Clone, Debug)]
pub struct CachedToken {
    pub token: String,
    pub expires_at: std::time::Instant,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Logging {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub verbosity: u8,
    pub log_dir: Option<PathBuf>,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RepoConfig {
    pub workspaces_dir: Option<PathBuf>,
    pub image: Option<String>,
    pub verbose: Option<u8>,
    pub default_memory: Option<String>,
    pub default_cpus: Option<String>,
    pub docker: Option<Docker>,
    pub github: Option<Github>,
    pub logging: Option<Logging>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EffectiveConfig {
    pub workspaces_dir: PathBuf,
    pub image: String,
    pub verbose: u8,
    pub default_memory: Option<String>,
    pub default_cpus: Option<String>,
    pub docker: Docker,
    pub github: Option<Github>,
    pub logging: Logging,
}

pub struct Loader {
    figment: Figment,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspaces_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("crowdcontrol-workspaces"),
            image: "crowdcontrol:latest".to_string(),
            verbose: 0,
            default_memory: None,
            default_cpus: None,
            docker: Docker::default(),
            github: None,
            logging: Logging::default(),
            repos: HashMap::new(),
        }
    }
}

impl Default for Docker {
    fn default() -> Self {
        Self {
            host: None,
            default_memory: None,
            default_cpus: None,
        }
    }
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            verbosity: 0,
            log_dir: None,
        }
    }
}

impl Loader {
    pub fn new() -> Self {
        let figment = default_figment();
        Self { figment }
    }
    
    pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.figment = self.figment.merge(Env::prefixed(&prefix.into()).split("__"));
        self
    }
    
    pub fn config_file(self, path: impl Into<PathBuf>) -> Self {
        // Replace the default config file with the specified one
        let figment = Figment::new()
            .merge(figment::providers::Serialized::defaults(Config::default()))
            .merge(Toml::file(path.into()))
            .merge(Env::prefixed("CC_").split("__"));
        
        // Apply any previous merges
        Self { figment }
    }
    
    pub fn merge<T: Serialize>(mut self, patch: &T) -> Self {
        self.figment = self.figment.merge(figment::providers::Serialized::defaults(patch));
        self
    }
    
    pub fn load(self) -> Result<Config> {
        let config: Config = self.figment
            .extract()
            .context("Failed to load configuration")?;
        
        Ok(config)
    }
    
    pub fn into_figment(self) -> Figment {
        self.figment
    }
}

impl Github {
    /// Convert to container environment variables for Docker
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
}

impl Config {
    pub fn validate(&mut self) -> Result<()> {
        if !self.workspaces_dir.exists() {
            std::fs::create_dir_all(&self.workspaces_dir)
                .with_context(|| format!("Failed to create workspaces directory: {:?}", self.workspaces_dir))?;
        }
        
        if !self.workspaces_dir.is_dir() {
            anyhow::bail!("Workspaces path is not a directory: {:?}", self.workspaces_dir);
        }
        
        if self.image.is_empty() {
            anyhow::bail!("Docker image cannot be empty");
        }
        
        Ok(())
    }
    
    /// Get the workspace path for a specific agent
    pub fn agent_workspace_path(&self, agent_name: &str) -> PathBuf {
        self.workspaces_dir.join(agent_name)
    }
    
    pub fn effective_for_repo<T: Serialize>(
        &self,
        repo: &str,
        patch: Option<&T>,
    ) -> Result<EffectiveConfig> {
        let mut effective = EffectiveConfig {
            workspaces_dir: self.workspaces_dir.clone(),
            image: self.image.clone(),
            verbose: self.verbose,
            default_memory: self.default_memory.clone(),
            default_cpus: self.default_cpus.clone(),
            docker: self.docker.clone(),
            github: self.github.clone(),
            logging: self.logging.clone(),
        };
        
        if let Some(repo_config) = self.repos.get(repo) {
            if let Some(dir) = &repo_config.workspaces_dir {
                effective.workspaces_dir = dir.clone();
            }
            if let Some(image) = &repo_config.image {
                effective.image = image.clone();
            }
            if let Some(verbose) = repo_config.verbose {
                effective.verbose = verbose;
            }
            if let Some(memory) = &repo_config.default_memory {
                effective.default_memory = Some(memory.clone());
            }
            if let Some(cpus) = &repo_config.default_cpus {
                effective.default_cpus = Some(cpus.clone());
            }
            if let Some(docker) = &repo_config.docker {
                effective.docker = docker.clone();
            }
            if let Some(github) = &repo_config.github {
                effective.github = Some(github.clone());
            }
            if let Some(logging) = &repo_config.logging {
                effective.logging = logging.clone();
            }
        }
        
        if let Some(patch) = patch {
            let patch_json = serde_json::to_value(patch)?;
            let mut effective_json = serde_json::to_value(&effective)?;
            merge_json(&mut effective_json, &patch_json);
            effective = serde_json::from_value(effective_json)?;
        }
        
        Ok(effective)
    }
}

pub fn default_figment() -> Figment {
    let config_file = dirs::config_dir()
        .map(|d| d.join("crowdcontrol").join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("~/.config/crowdcontrol/config.toml"));
    
    Figment::new()
        .merge(figment::providers::Serialized::defaults(Config::default()))
        .merge(Toml::file(config_file))
        .merge(Env::prefixed("CC_").split("__"))
}

fn merge_json(target: &mut serde_json::Value, source: &serde_json::Value) {
    use serde_json::Value;
    
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, value) in source_map {
                match target_map.get_mut(key) {
                    Some(target_value) => merge_json(target_value, value),
                    None => {
                        target_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, source) => {
            *target = source.clone();
        }
    }
}

pub type ConfigArc = Arc<Config>;