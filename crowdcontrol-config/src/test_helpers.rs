use crate::*;
use std::path::{Path, PathBuf};

/// Create a test config with minimal defaults
pub fn test_config() -> Config {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    Config {
        workspaces_dir: home.join(".crowdcontrol").join("workspaces"),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
        docker: Docker::default(),
        github: None,
        logging: Logging::default(),
        repos: Default::default(),
    }
}

/// Create a test config with custom workspaces directory
pub fn test_config_with_dir<P: AsRef<Path>>(workspaces_dir: P) -> Config {
    Config {
        workspaces_dir: workspaces_dir.as_ref().to_path_buf(),
        image: "crowdcontrol:latest".to_string(),
        verbose: 0,
        default_memory: None,
        default_cpus: None,
        docker: Docker::default(),
        github: None,
        logging: Logging::default(),
        repos: Default::default(),
    }
}