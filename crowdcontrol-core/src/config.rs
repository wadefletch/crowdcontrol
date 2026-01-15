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

    /// Get agent path using either display name (colon) or filesystem name (hyphen)
    /// e.g., "myapp:main" or "myapp-main" -> "~/.crowdcontrol/myapp/main/"
    pub fn agent_workspace_path(&self, name: &str) -> PathBuf {
        // Try colon format first (display name: "myapp:main")
        if let Some(pos) = name.find(':') {
            let project = &name[..pos];
            let label = &name[pos + 1..];
            let nested_path = self.workspaces_dir.join(project).join(label);
            if nested_path.exists() {
                trace!("Agent path for '{}': {:?} (nested via colon)", name, nested_path);
                return nested_path;
            }
        }

        // Try hyphen format (filesystem name: "myapp-main")
        if let Some(pos) = name.find('-') {
            let project = &name[..pos];
            let label = &name[pos + 1..];
            let nested_path = self.workspaces_dir.join(project).join(label);
            if nested_path.exists() {
                trace!("Agent path for '{}': {:?} (nested via hyphen)", name, nested_path);
                return nested_path;
            }
        }

        // Fallback to flat path (standalone agents)
        let path = self.workspaces_dir.join(name);
        trace!("Agent path for '{}': {:?} (flat)", name, path);
        path
    }

    /// Get agent path with explicit project/label structure
    pub fn agent_path(&self, project_slug: Option<&str>, label: &str) -> PathBuf {
        let path = match project_slug {
            Some(project) => self.workspaces_dir.join(project).join(label),
            None => self.workspaces_dir.join(label),
        };
        trace!(
            "Agent path for project={:?}, label='{}': {:?}",
            project_slug,
            label,
            path
        );
        path
    }
}
