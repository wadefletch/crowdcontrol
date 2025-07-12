use anyhow::{anyhow, Context, Result};
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, ListContainersOptions, LogsOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::{Docker, API_DEFAULT_VERSION};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use tracing::{debug, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use crate::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub status: AgentStatus,
    pub container_id: Option<String>,
    pub repository: String,
    pub branch: Option<String>,
    pub created_at: DateTime<Utc>,
    pub workspace_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Created,
    Running,
    Stopped,
    Error,
}

pub struct DockerClient {
    docker: Docker,
    config: Config,
}

impl DockerClient {
    pub fn new(config: Config) -> Result<Self> {
        debug!("Initializing Docker client");
        
        // Use connect_with_defaults() which respects DOCKER_HOST env var
        // If DOCKER_HOST is not set, it will try default locations
        let docker = if let Ok(docker_host) = env::var("DOCKER_HOST") {
            // If DOCKER_HOST is set, use it
            info!("Connecting to Docker using DOCKER_HOST: {}", docker_host);
            Docker::connect_with_defaults()
                .context("Failed to connect to Docker using DOCKER_HOST")?
        } else {
            // Try to detect the correct socket location
            #[cfg(unix)]
            {
                // Check common socket locations
                let home_socket = format!(
                    "{}/.docker/run/docker.sock",
                    env::var("HOME").unwrap_or_default()
                );
                let socket_locations = vec!["/var/run/docker.sock", home_socket.as_str()];

                let mut connected = None;
                for socket in &socket_locations {
                    trace!("Checking Docker socket at: {}", socket);
                    if std::path::Path::new(socket).exists() {
                        debug!("Found Docker socket at: {}", socket);
                        if let Ok(docker) =
                            Docker::connect_with_unix(socket, 120, API_DEFAULT_VERSION)
                        {
                            info!("Successfully connected to Docker at: {}", socket);
                            connected = Some(docker);
                            break;
                        } else {
                            warn!("Socket exists but failed to connect at: {}", socket);
                        }
                    }
                }

                connected.ok_or_else(|| {
                    anyhow!("Failed to connect to Docker. Docker socket not found at common locations.\n\
                           Try setting DOCKER_HOST environment variable:\n\
                           export DOCKER_HOST=unix://$HOME/.docker/run/docker.sock")
                })?
            }

            #[cfg(windows)]
            {
                Docker::connect_with_local_defaults()
                    .context("Failed to connect to Docker. Is Docker Desktop running?")?
            }
        };

        Ok(Self { docker, config })
    }

    pub async fn container_exists(&self, name: &str) -> Result<bool> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![name.to_string()]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;
        Ok(!containers.is_empty())
    }

    pub async fn create_container(
        &self,
        name: &str,
        workspace_path: &PathBuf,
        memory: Option<String>,
        cpus: Option<String>,
    ) -> Result<String> {
        let container_name = format!("crowdcontrol-{}", name);
        
        info!(
            "Creating container '{}' with workspace: {:?}, memory: {:?}, cpus: {:?}", 
            container_name, workspace_path, memory, cpus
        );

        // Set up mounts - canonicalize path to avoid Docker Desktop issues
        let canonical_workspace = workspace_path.canonicalize()
            .with_context(|| format!("Failed to canonicalize workspace path: {:?}", workspace_path))?;
        
        let mut mounts = vec![
            // Mount workspace
            Mount {
                target: Some("/workspace".to_string()),
                source: Some(canonical_workspace.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            },
        ];

        // Add SSH keys if available
        let ssh_dir = dirs::home_dir().unwrap().join(".ssh");
        if ssh_dir.exists() {
            mounts.push(Mount {
                target: Some("/root/.ssh".to_string()),
                source: Some(ssh_dir.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(true),
                ..Default::default()
            });
        }

        // Add git config file if available
        let git_config_file = dirs::home_dir().unwrap().join(".gitconfig");
        if git_config_file.exists() {
            mounts.push(Mount {
                target: Some("/root/.gitconfig".to_string()),
                source: Some(git_config_file.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(true),
                ..Default::default()
            });
        }

        // Add git config directory if available
        let git_config_dir = dirs::home_dir().unwrap().join(".config/git");
        if git_config_dir.exists() {
            mounts.push(Mount {
                target: Some("/root/.config/git".to_string()),
                source: Some(git_config_dir.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(true),
                ..Default::default()
            });
        }

        // Add Claude Code config if available
        let claude_config = dirs::home_dir().unwrap().join(".claude");
        if claude_config.exists() {
            mounts.push(Mount {
                target: Some("/root/.claude".to_string()),
                source: Some(claude_config.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(true),
                ..Default::default()
            });
        }

        let mut host_config = HostConfig {
            privileged: Some(true),
            mounts: Some(mounts),
            ..Default::default()
        };

        // Set resource limits if provided
        if let Some(memory_limit) = memory {
            let memory_bytes = parse_memory_limit(&memory_limit)?;
            host_config.memory = Some(memory_bytes);
        }

        if let Some(cpu_limit) = cpus {
            let cpu_quota = (cpu_limit.parse::<f64>()? * 100000.0) as i64;
            host_config.cpu_quota = Some(cpu_quota);
            host_config.cpu_period = Some(100000);
        }

        let container_config = ContainerConfig {
            image: Some(self.config.image.clone()),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name.clone(),
            platform: None,
        };

        let container = self
            .docker
            .create_container(Some(options), container_config)
            .await
            .context("Failed to create container")?;

        Ok(container.id)
    }

    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        info!("Starting container: {}", container_id);
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;
        debug!("Container {} started successfully", container_id);
        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str, force: bool) -> Result<()> {
        info!("Stopping container: {} (force: {})", container_id, force);
        let options = if force {
            StopContainerOptions { t: 0 }
        } else {
            StopContainerOptions { t: 5 }  // Quick stop for dev containers
        };

        self.docker
            .stop_container(container_id, Some(options))
            .await
            .context("Failed to stop container")?;
        debug!("Container {} stopped successfully", container_id);
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<()> {
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        self.docker
            .remove_container(container_id, Some(options))
            .await
            .context("Failed to remove container")?;
        Ok(())
    }

    pub async fn exec_in_container(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
        attach: bool,
    ) -> Result<()> {
        let exec_config = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(attach),
            attach_stderr: Some(attach),
            attach_stdin: Some(attach),
            tty: Some(attach),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(container_id, exec_config)
            .await
            .context("Failed to create exec")?;

        if attach {
            match self.docker.start_exec(&exec.id, None).await? {
                StartExecResults::Attached { mut output, .. } => {
                    while let Some(msg) = output.next().await {
                        print!("{}", msg?);
                    }
                }
                StartExecResults::Detached => {}
            }
        } else {
            self.docker.start_exec(&exec.id, None).await?;
        }

        Ok(())
    }

    pub async fn get_container_logs(
        &self,
        container_id: &str,
        follow: bool,
        tail: Option<String>,
        timestamps: bool,
    ) -> Result<()> {
        let options = LogsOptions {
            follow,
            stdout: true,
            stderr: true,
            tail: tail.unwrap_or_else(|| "50".to_string()),
            timestamps,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(output) => print!("{}", output),
                Err(e) => eprintln!("Error reading logs: {}", e),
            }
        }

        Ok(())
    }

    /// List all CrowdControl containers (running and stopped)
    pub async fn list_all_containers(&self) -> Result<Vec<bollard::models::ContainerSummary>> {
        let mut filters = HashMap::new();
        filters.insert("label".to_string(), vec!["app=crowdcontrol".to_string()]);
        
        let options = ListContainersOptions {
            all: true,  // Include stopped containers
            filters,
            ..Default::default()
        };
        
        self.docker.list_containers(Some(options))
            .await
            .context("Failed to list containers")
    }

    pub async fn get_container_status(&self, name: &str) -> Result<AgentStatus> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("crowdcontrol-{}", name)]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        if containers.is_empty() {
            return Ok(AgentStatus::Created);
        }

        let container = &containers[0];
        let state = container
            .state
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unknown");

        match state {
            "created" => Ok(AgentStatus::Created),
            "running" => Ok(AgentStatus::Running),
            "exited" => Ok(AgentStatus::Stopped),
            "dead" | "removing" => Ok(AgentStatus::Error),
            _ => Ok(AgentStatus::Stopped),
        }
    }

    pub async fn pull_image(&self) -> Result<()> {
        // First check if the image exists locally
        let images = self.docker.list_images::<String>(None).await?;
        let image_exists = images.iter().any(|img| {
            img.repo_tags.iter().any(|tag| tag == &self.config.image || tag.starts_with(&format!("{}:", self.config.image)))
        });

        if image_exists {
            println!("Docker image {} already exists locally", self.config.image);
            return Ok(());
        }

        println!("Pulling Docker image: {}", self.config.image);

        let options = CreateImageOptions {
            from_image: self.config.image.clone(),
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(info) => {
                    if let Some(status) = info.status {
                        print!("\r{}", status);
                    }
                }
                Err(e) => eprintln!("Error pulling image: {}", e),
            }
        }
        println!();

        Ok(())
    }
}

fn parse_memory_limit(memory: &str) -> Result<i64> {
    let memory_lower = memory.to_lowercase();
    let multiplier = if memory_lower.ends_with("g") {
        1_073_741_824
    } else if memory_lower.ends_with("m") {
        1_048_576
    } else if memory_lower.ends_with("k") {
        1_024
    } else {
        return Err(anyhow!(
            "Invalid memory format. Use format like: 2g, 1024m, or 512k"
        ));
    };

    let number_part = &memory_lower[..memory_lower.len() - 1];
    let number: i64 = number_part.parse().context("Invalid memory value")?;

    Ok(number * multiplier)
}
