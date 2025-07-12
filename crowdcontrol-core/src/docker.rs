use anyhow::{anyhow, Context, Result};
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, InspectContainerOptions, ListContainersOptions, LogsOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::{Docker, API_DEFAULT_VERSION};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use tracing::{debug, info, trace, warn};

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

impl Agent {
    /// Compute the current live status from Docker.
    /// This is the single source of truth for agent status.
    pub async fn compute_live_status(&self, docker: &DockerClient) -> Result<AgentStatus> {
        match &self.container_id {
            None => Ok(AgentStatus::Created),
            Some(container_id) => {
                // First validate the container ID is still valid for this agent
                if !docker.validate_container_id(&self.name, container_id).await? {
                    // Container ID is stale, agent is effectively Created
                    return Ok(AgentStatus::Created);
                }
                
                // Get live status from Docker
                docker.get_container_status(&self.name).await
            }
        }
    }
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
        let canonical_workspace = workspace_path.canonicalize().with_context(|| {
            format!(
                "Failed to canonicalize workspace path: {:?}",
                workspace_path
            )
        })?;

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

        // Mount Claude config - both new and legacy formats
        let home_dir = dirs::home_dir().unwrap();
        
        // Mount .claude directory if it exists
        let claude_dir = home_dir.join(".claude");
        if claude_dir.exists() {
            mounts.push(Mount {
                target: Some("/mnt/claude-config/.claude".to_string()),
                source: Some(claude_dir.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(true),
                ..Default::default()
            });
        }
        
        // Mount legacy .claude.json if it exists
        let claude_legacy = home_dir.join(".claude.json");
        if claude_legacy.exists() {
            mounts.push(Mount {
                target: Some("/mnt/claude-config/.claude.json".to_string()),
                source: Some(claude_legacy.to_string_lossy().to_string()),
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

        // Get host UID/GID for user mapping
        let user_id = unsafe { libc::getuid() };
        let group_id = unsafe { libc::getgid() };

        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "crowdcontrol".to_string());
        
        let container_config = ContainerConfig {
            image: Some(self.config.image.clone()),
            host_config: Some(host_config),
            env: Some(vec![
                format!("HOST_UID={}", user_id),
                format!("HOST_GID={}", group_id),
            ]),
            labels: Some(labels),
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
            StopContainerOptions { t: 5 } // Quick stop for dev containers
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
        self.exec_in_container_as_user(container_id, cmd, attach, None).await
    }

    pub async fn exec_in_container_as_user(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
        attach: bool,
        user: Option<&str>,
    ) -> Result<()> {
        let exec_config = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(attach),
            attach_stderr: Some(attach),
            attach_stdin: Some(attach),
            tty: Some(attach),
            user,
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
            all: true, // Include stopped containers
            filters,
            ..Default::default()
        };

        self.docker
            .list_containers(Some(options))
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
            img.repo_tags.iter().any(|tag| {
                tag == &self.config.image || tag.starts_with(&format!("{}:", self.config.image))
            })
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

    /// Validate that a container ID actually belongs to the specified agent
    pub async fn validate_container_id(&self, agent_name: &str, container_id: &str) -> Result<bool> {
        let expected_container_name = format!("crowdcontrol-{}", agent_name);
        
        // Get container details
        match self.docker.inspect_container(container_id, None::<InspectContainerOptions>).await {
            Ok(container) => {
                // Check if container name matches expected agent name
                if let Some(name) = container.name {
                    // Docker container names start with "/"
                    let clean_name = name.strip_prefix('/').unwrap_or(&name);
                    Ok(clean_name == expected_container_name)
                } else {
                    Ok(false)
                }
            }
            Err(_) => {
                // Container doesn't exist or can't be inspected
                Ok(false)
            }
        }
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
