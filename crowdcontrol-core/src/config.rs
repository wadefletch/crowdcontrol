use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, trace};

use crate::Settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspaces_dir: PathBuf,
    pub image: String,
    pub verbose: u8,
    pub default_memory: Option<String>,
    pub default_cpus: Option<String>,
}

impl Config {
    /// Create config from settings
    pub fn from_settings(settings: Settings) -> Result<Self> {
        debug!("Creating config from settings");
        trace!("Settings: {:?}", settings);

        // Ensure workspaces directory exists
        debug!(
            "Creating workspaces directory: {:?}",
            settings.workspaces_dir
        );
        fs::create_dir_all(&settings.workspaces_dir).with_context(|| {
            format!(
                "Failed to create workspaces directory: {:?}",
                settings.workspaces_dir
            )
        })?;

        Ok(Config {
            workspaces_dir: settings.workspaces_dir,
            image: settings.image,
            verbose: settings.verbose,
            default_memory: settings.default_memory,
            default_cpus: settings.default_cpus,
        })
    }

    pub fn agent_workspace_path(&self, name: &str) -> PathBuf {
        let path = self.workspaces_dir.join(name);
        trace!("Agent workspace path for '{}': {:?}", name, path);
        path
    }
}
