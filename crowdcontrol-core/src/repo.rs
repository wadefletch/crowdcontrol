use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{debug, trace};

/// A repository definition with a short slug mapping to a git URL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Repo {
    pub slug: String,
    pub url: String,
    pub default_branch: Option<String>,
}

/// Parsed agent identifier from "repo:label" or plain "name" format
#[derive(Debug, Clone, PartialEq)]
pub struct AgentIdentifier {
    /// The repo slug, if using repo:label format
    pub repo_slug: Option<String>,
    /// The agent label (or full name if no repo)
    pub label: String,
}

impl AgentIdentifier {
    /// Convert to filesystem-safe name (uses hyphen instead of colon)
    pub fn to_filesystem_name(&self) -> String {
        match &self.repo_slug {
            Some(slug) => format!("{}-{}", slug, self.label),
            None => self.label.clone(),
        }
    }

    /// Convert to display name (uses colon for repo:label)
    pub fn to_display_name(&self) -> String {
        match &self.repo_slug {
            Some(slug) => format!("{}:{}", slug, self.label),
            None => self.label.clone(),
        }
    }
}

/// Validate a repo slug
pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        return Err(anyhow!("Repo slug cannot be empty"));
    }

    if slug.contains(':') {
        return Err(anyhow!(
            "Repo slug cannot contain ':' (reserved for repo:label separator)"
        ));
    }

    if slug.contains('/') {
        return Err(anyhow!("Repo slug cannot contain '/'"));
    }

    if slug.contains(' ') {
        return Err(anyhow!("Repo slug cannot contain spaces"));
    }

    if !slug
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "Repo slug can only contain alphanumeric characters, hyphens, and underscores"
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
        let repo_slug = parts[0];
        let label = parts[1];

        if repo_slug.is_empty() {
            return Err(anyhow!("Repo slug cannot be empty in '{}'", input));
        }

        if label.is_empty() {
            return Err(anyhow!("Agent label cannot be empty in '{}'", input));
        }

        validate_slug(repo_slug)?;

        Ok(AgentIdentifier {
            repo_slug: Some(repo_slug.to_string()),
            label: label.to_string(),
        })
    } else {
        // Plain name without repo
        Ok(AgentIdentifier {
            repo_slug: None,
            label: input.to_string(),
        })
    }
}

/// Storage format for repos.toml
#[derive(Debug, Default, Serialize, Deserialize)]
struct ReposFile {
    #[serde(default)]
    repos: HashMap<String, RepoEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RepoEntry {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_branch: Option<String>,
}

/// Store for managing repo definitions
pub struct RepoStore {
    path: PathBuf,
}

impl RepoStore {
    /// Create a new RepoStore with the given path
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load repos from file
    fn load(&self) -> Result<ReposFile> {
        if !self.path.exists() {
            trace!("Repos file does not exist, returning empty: {:?}", self.path);
            return Ok(ReposFile::default());
        }

        let mut file = OpenOptions::new()
            .read(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open repos file: {:?}", self.path))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("Failed to read repos file: {:?}", self.path))?;

        let repos_file: ReposFile = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse repos file: {:?}", self.path))?;

        trace!("Loaded {} repos from {:?}", repos_file.repos.len(), self.path);
        Ok(repos_file)
    }

    /// Save repos to file
    fn save(&self, repos_file: &ReposFile) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create repos directory: {:?}", parent))?;
        }

        let contents = toml::to_string_pretty(repos_file)
            .with_context(|| "Failed to serialize repos")?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open repos file for writing: {:?}", self.path))?;

        file.write_all(contents.as_bytes())
            .with_context(|| format!("Failed to write repos file: {:?}", self.path))?;

        debug!("Saved {} repos to {:?}", repos_file.repos.len(), self.path);
        Ok(())
    }

    /// Add a new repo
    pub fn add(&mut self, repo: Repo) -> Result<()> {
        validate_slug(&repo.slug)?;

        let mut repos_file = self.load()?;

        if repos_file.repos.contains_key(&repo.slug) {
            return Err(anyhow!("Repo '{}' already exists", repo.slug));
        }

        debug!("Adding repo '{}' -> {}", repo.slug, repo.url);
        repos_file.repos.insert(
            repo.slug,
            RepoEntry {
                url: repo.url,
                default_branch: repo.default_branch,
            },
        );

        self.save(&repos_file)
    }

    /// Get a repo by slug
    pub fn get(&self, slug: &str) -> Result<Option<Repo>> {
        let repos_file = self.load()?;

        Ok(repos_file.repos.get(slug).map(|entry| Repo {
            slug: slug.to_string(),
            url: entry.url.clone(),
            default_branch: entry.default_branch.clone(),
        }))
    }

    /// List all repos
    pub fn list(&self) -> Result<Vec<Repo>> {
        let repos_file = self.load()?;

        let repos: Vec<Repo> = repos_file
            .repos
            .into_iter()
            .map(|(slug, entry)| Repo {
                slug,
                url: entry.url,
                default_branch: entry.default_branch,
            })
            .collect();

        Ok(repos)
    }

    /// Remove a repo by slug
    pub fn remove(&mut self, slug: &str) -> Result<()> {
        let mut repos_file = self.load()?;

        if !repos_file.repos.contains_key(slug) {
            return Err(anyhow!("Repo '{}' not found", slug));
        }

        debug!("Removing repo '{}'", slug);
        repos_file.repos.remove(slug);

        self.save(&repos_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_identifier_with_repo() {
        let id = AgentIdentifier {
            repo_slug: Some("myapp".to_string()),
            label: "main".to_string(),
        };

        assert_eq!(id.to_filesystem_name(), "myapp-main");
        assert_eq!(id.to_display_name(), "myapp:main");
    }

    #[test]
    fn test_agent_identifier_without_repo() {
        let id = AgentIdentifier {
            repo_slug: None,
            label: "standalone".to_string(),
        };

        assert_eq!(id.to_filesystem_name(), "standalone");
        assert_eq!(id.to_display_name(), "standalone");
    }
}
