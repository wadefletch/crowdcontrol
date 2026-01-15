use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{debug, trace};

/// A project definition with a short slug mapping to a git URL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub slug: String,
    pub url: String,
    pub default_branch: Option<String>,
}

/// Parsed agent identifier from "project:label" or plain "name" format
#[derive(Debug, Clone, PartialEq)]
pub struct AgentIdentifier {
    /// The project slug, if using project:label format
    pub project_slug: Option<String>,
    /// The agent label (or full name if no project)
    pub label: String,
}

impl AgentIdentifier {
    /// Convert to filesystem-safe name (uses hyphen instead of colon)
    pub fn to_filesystem_name(&self) -> String {
        match &self.project_slug {
            Some(slug) => format!("{}-{}", slug, self.label),
            None => self.label.clone(),
        }
    }

    /// Convert to display name (uses colon for project:label)
    pub fn to_display_name(&self) -> String {
        match &self.project_slug {
            Some(slug) => format!("{}:{}", slug, self.label),
            None => self.label.clone(),
        }
    }
}

/// Validate a project slug
pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        return Err(anyhow!("Project slug cannot be empty"));
    }

    if slug.contains(':') {
        return Err(anyhow!(
            "Project slug cannot contain ':' (reserved for project:label separator)"
        ));
    }

    if slug.contains('/') {
        return Err(anyhow!("Project slug cannot contain '/'"));
    }

    if slug.contains(' ') {
        return Err(anyhow!("Project slug cannot contain spaces"));
    }

    if !slug
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "Project slug can only contain alphanumeric characters, hyphens, and underscores"
        ));
    }

    Ok(())
}

/// Parse an agent identifier string into its components
pub fn parse_agent_identifier(input: &str) -> Result<AgentIdentifier> {
    if input.is_empty() {
        return Err(anyhow!("Agent identifier cannot be empty"));
    }

    let colon_count = input.chars().filter(|&c| c == ':').count();

    if colon_count > 1 {
        return Err(anyhow!(
            "Agent identifier cannot contain multiple colons: '{}'",
            input
        ));
    }

    if colon_count == 1 {
        let parts: Vec<&str> = input.split(':').collect();
        let project_slug = parts[0];
        let label = parts[1];

        if project_slug.is_empty() {
            return Err(anyhow!("Project slug cannot be empty in '{}'", input));
        }

        if label.is_empty() {
            return Err(anyhow!("Agent label cannot be empty in '{}'", input));
        }

        validate_slug(project_slug)?;

        Ok(AgentIdentifier {
            project_slug: Some(project_slug.to_string()),
            label: label.to_string(),
        })
    } else {
        // Plain name without project
        Ok(AgentIdentifier {
            project_slug: None,
            label: input.to_string(),
        })
    }
}

/// Storage format for projects.toml
#[derive(Debug, Default, Serialize, Deserialize)]
struct ProjectsFile {
    #[serde(default)]
    projects: HashMap<String, ProjectEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectEntry {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_branch: Option<String>,
}

/// Store for managing project definitions
pub struct ProjectStore {
    path: PathBuf,
}

impl ProjectStore {
    /// Create a new ProjectStore with the given path
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load projects from file
    fn load(&self) -> Result<ProjectsFile> {
        if !self.path.exists() {
            trace!("Projects file does not exist, returning empty: {:?}", self.path);
            return Ok(ProjectsFile::default());
        }

        let mut file = OpenOptions::new()
            .read(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open projects file: {:?}", self.path))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("Failed to read projects file: {:?}", self.path))?;

        let projects_file: ProjectsFile = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse projects file: {:?}", self.path))?;

        trace!("Loaded {} projects from {:?}", projects_file.projects.len(), self.path);
        Ok(projects_file)
    }

    /// Save projects to file
    fn save(&self, projects_file: &ProjectsFile) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create projects directory: {:?}", parent))?;
        }

        let contents = toml::to_string_pretty(projects_file)
            .with_context(|| "Failed to serialize projects")?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open projects file for writing: {:?}", self.path))?;

        file.write_all(contents.as_bytes())
            .with_context(|| format!("Failed to write projects file: {:?}", self.path))?;

        debug!("Saved {} projects to {:?}", projects_file.projects.len(), self.path);
        Ok(())
    }

    /// Add a new project
    pub fn add(&mut self, project: Project) -> Result<()> {
        validate_slug(&project.slug)?;

        let mut projects_file = self.load()?;

        if projects_file.projects.contains_key(&project.slug) {
            return Err(anyhow!("Project '{}' already exists", project.slug));
        }

        debug!("Adding project '{}' -> {}", project.slug, project.url);
        projects_file.projects.insert(
            project.slug,
            ProjectEntry {
                url: project.url,
                default_branch: project.default_branch,
            },
        );

        self.save(&projects_file)
    }

    /// Get a project by slug
    pub fn get(&self, slug: &str) -> Result<Option<Project>> {
        let projects_file = self.load()?;

        Ok(projects_file.projects.get(slug).map(|entry| Project {
            slug: slug.to_string(),
            url: entry.url.clone(),
            default_branch: entry.default_branch.clone(),
        }))
    }

    /// List all projects
    pub fn list(&self) -> Result<Vec<Project>> {
        let projects_file = self.load()?;

        let projects: Vec<Project> = projects_file
            .projects
            .into_iter()
            .map(|(slug, entry)| Project {
                slug,
                url: entry.url,
                default_branch: entry.default_branch,
            })
            .collect();

        Ok(projects)
    }

    /// Remove a project by slug
    pub fn remove(&mut self, slug: &str) -> Result<()> {
        let mut projects_file = self.load()?;

        if !projects_file.projects.contains_key(slug) {
            return Err(anyhow!("Project '{}' not found", slug));
        }

        debug!("Removing project '{}'", slug);
        projects_file.projects.remove(slug);

        self.save(&projects_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_identifier_with_project() {
        let id = AgentIdentifier {
            project_slug: Some("myapp".to_string()),
            label: "main".to_string(),
        };

        assert_eq!(id.to_filesystem_name(), "myapp-main");
        assert_eq!(id.to_display_name(), "myapp:main");
    }

    #[test]
    fn test_agent_identifier_without_project() {
        let id = AgentIdentifier {
            project_slug: None,
            label: "standalone".to_string(),
        };

        assert_eq!(id.to_filesystem_name(), "standalone");
        assert_eq!(id.to_display_name(), "standalone");
    }
}
