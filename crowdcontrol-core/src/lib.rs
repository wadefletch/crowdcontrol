pub mod agent;
pub mod docker;
pub mod github;
pub mod logger;
pub mod state_validator;

// Re-export config types from the new config crate
pub use crowdcontrol_config::{
    Config, ConfigArc, Docker, EffectiveConfig, Github, Loader, Logging, RepoConfig,
    default_figment,
};

// Re-export test helpers
pub use crowdcontrol_config::test_helpers::{test_config, test_config_with_dir};

pub use agent::*;
pub use docker::{Agent, AgentStatus, DockerClient};
pub use github::{CachedToken, GitHubConfig, GitHubCredentialManager, GitHubCredentialTemplate};
pub use logger::init_logger;
pub use state_validator::{StateInconsistency, StateValidator};
