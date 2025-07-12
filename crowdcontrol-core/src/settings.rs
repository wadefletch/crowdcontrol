use anyhow::{Context, Result};
use config::{Config as ConfigBuilder, Environment, File};
use tracing::{debug, info, trace};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Directory for storing agent workspaces
    #[serde(default = "default_workspaces_dir")]
    pub workspaces_dir: PathBuf,
    
    /// Docker image to use for agents
    #[serde(default = "default_image")]
    pub image: String,
    
    /// Default memory limit for agents
    #[serde(default)]
    pub default_memory: Option<String>,
    
    /// Default CPU limit for agents
    #[serde(default)]
    pub default_cpus: Option<String>,
    
    /// Verbosity level
    #[serde(default)]
    pub verbose: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            workspaces_dir: default_workspaces_dir(),
            image: default_image(),
            default_memory: None,
            default_cpus: None,
            verbose: 0,
        }
    }
}

impl Settings {
    /// Load settings from config file and environment variables
    /// 
    /// Priority order (highest to lowest):
    /// 1. CLI arguments (handled by caller)
    /// 2. Environment variables (CROWDCONTROL_*)
    /// 3. Config file (~/.config/crowdcontrol/config.toml)
    /// 4. Default values
    pub fn load() -> Result<Self> {
        debug!("Loading settings from configuration sources");
        
        let config_path = dirs::config_dir()
            .map(|p| p.join("crowdcontrol/config.toml"))
            .filter(|p| p.exists());
        
        if let Some(ref path) = config_path {
            info!("Found config file at: {:?}", path);
        } else {
            debug!("No config file found, using defaults and environment variables");
        }

        let mut builder = ConfigBuilder::builder()
            .set_default("workspaces_dir", default_workspaces_dir().to_string_lossy().to_string())?
            .set_default("image", default_image())?;

        // Add config file if it exists
        if let Some(path) = config_path {
            builder = builder.add_source(File::from(path));
        }

        // Add environment variables with CROWDCONTROL_ prefix
        builder = builder.add_source(
            Environment::with_prefix("CROWDCONTROL")
                .separator("_")
                .try_parsing(true)
        );

        let settings = builder
            .build()
            .context("Failed to load configuration")?
            .try_deserialize()
            .context("Failed to parse configuration")?;

        trace!("Loaded settings: {:?}", settings);
        Ok(settings)
    }

    /// Create settings with CLI overrides
    pub fn with_overrides(
        workspaces_dir: Option<PathBuf>,
        image: Option<String>,
        verbose: u8,
    ) -> Result<Self> {
        let mut settings = Self::load()?;

        // Apply CLI overrides
        if let Some(dir) = workspaces_dir {
            debug!("Overriding workspaces_dir from CLI: {:?}", dir);
            settings.workspaces_dir = dir;
        }
        if let Some(img) = image {
            debug!("Overriding image from CLI: {}", img);
            settings.image = img;
        }
        if verbose > 0 {
            debug!("Setting verbosity level from CLI: {}", verbose);
            settings.verbose = verbose;
        }

        Ok(settings)
    }
}

fn default_workspaces_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Unable to determine home directory")
        .join("crowdcontrol-workspaces")
}

fn default_image() -> String {
    "crowdcontrol:latest".to_string()
}