// TDD: These tests define the expected behavior for the Repo module
// Run with: cargo test --package crowdcontrol-core repo_tests

use anyhow::Result;
use tempfile::TempDir;

// These imports will fail until we implement the module
use crowdcontrol_core::repo::{Repo, RepoStore};

fn create_test_repo_store() -> (RepoStore, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let repos_path = temp_dir.path().join("repos.toml");
    let store = RepoStore::new(repos_path);
    (store, temp_dir)
}

#[test]
fn test_repo_struct_has_required_fields() {
    let repo = Repo {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: Some("main".to_string()),
    };

    assert_eq!(repo.slug, "myapp");
    assert_eq!(repo.url, "git@github.com:org/myapp.git");
    assert_eq!(repo.default_branch, Some("main".to_string()));
}

#[test]
fn test_repo_without_default_branch() {
    let repo = Repo {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: None,
    };

    assert_eq!(repo.default_branch, None);
}

#[test]
fn test_add_repo_to_store() -> Result<()> {
    let (mut store, _temp_dir) = create_test_repo_store();

    let repo = Repo {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: Some("main".to_string()),
    };

    store.add(repo)?;

    // Verify it was added
    let loaded = store.get("myapp")?;
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().url, "git@github.com:org/myapp.git");

    Ok(())
}

#[test]
fn test_get_nonexistent_repo_returns_none() -> Result<()> {
    let (store, _temp_dir) = create_test_repo_store();

    let result = store.get("nonexistent")?;
    assert!(result.is_none());

    Ok(())
}

#[test]
fn test_list_repos_empty() -> Result<()> {
    let (store, _temp_dir) = create_test_repo_store();

    let repos = store.list()?;
    assert!(repos.is_empty());

    Ok(())
}

#[test]
fn test_list_repos_returns_all() -> Result<()> {
    let (mut store, _temp_dir) = create_test_repo_store();

    store.add(Repo {
        slug: "app1".to_string(),
        url: "git@github.com:org/app1.git".to_string(),
        default_branch: None,
    })?;

    store.add(Repo {
        slug: "app2".to_string(),
        url: "git@github.com:org/app2.git".to_string(),
        default_branch: Some("develop".to_string()),
    })?;

    let repos = store.list()?;
    assert_eq!(repos.len(), 2);

    Ok(())
}

#[test]
fn test_remove_repo() -> Result<()> {
    let (mut store, _temp_dir) = create_test_repo_store();

    store.add(Repo {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: None,
    })?;

    // Verify it exists
    assert!(store.get("myapp")?.is_some());

    // Remove it
    store.remove("myapp")?;

    // Verify it's gone
    assert!(store.get("myapp")?.is_none());

    Ok(())
}

#[test]
fn test_remove_nonexistent_repo_returns_error() {
    let (mut store, _temp_dir) = create_test_repo_store();

    let result = store.remove("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_add_duplicate_slug_returns_error() -> Result<()> {
    let (mut store, _temp_dir) = create_test_repo_store();

    store.add(Repo {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: None,
    })?;

    // Adding another repo with the same slug should fail
    let result = store.add(Repo {
        slug: "myapp".to_string(),
        url: "git@github.com:other/myapp.git".to_string(),
        default_branch: None,
    });

    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_repo_persists_across_store_instances() -> Result<()> {
    let temp_dir = tempfile::tempdir().unwrap();
    let repos_path = temp_dir.path().join("repos.toml");

    // Create first store and add repo
    {
        let mut store = RepoStore::new(repos_path.clone());
        store.add(Repo {
            slug: "myapp".to_string(),
            url: "git@github.com:org/myapp.git".to_string(),
            default_branch: Some("main".to_string()),
        })?;
    }

    // Create new store instance and verify repo is still there
    {
        let store = RepoStore::new(repos_path);
        let repo = store.get("myapp")?;
        assert!(repo.is_some());
        assert_eq!(repo.unwrap().url, "git@github.com:org/myapp.git");
    }

    Ok(())
}

#[test]
fn test_validate_slug_valid() {
    use crowdcontrol_core::repo::validate_slug;

    assert!(validate_slug("myapp").is_ok());
    assert!(validate_slug("my-app").is_ok());
    assert!(validate_slug("my_app").is_ok());
    assert!(validate_slug("app123").is_ok());
}

#[test]
fn test_validate_slug_invalid() {
    use crowdcontrol_core::repo::validate_slug;

    // Cannot contain colon (reserved for repo:label separator)
    assert!(validate_slug("my:app").is_err());

    // Cannot be empty
    assert!(validate_slug("").is_err());

    // Cannot contain spaces
    assert!(validate_slug("my app").is_err());

    // Cannot contain slashes
    assert!(validate_slug("my/app").is_err());
}

#[test]
fn test_parse_agent_identifier_with_repo() {
    use crowdcontrol_core::repo::parse_agent_identifier;

    let result = parse_agent_identifier("myapp:main").unwrap();

    assert_eq!(result.repo_slug, Some("myapp".to_string()));
    assert_eq!(result.label, "main".to_string());
}

#[test]
fn test_parse_agent_identifier_without_repo() {
    use crowdcontrol_core::repo::parse_agent_identifier;

    // Plain name without colon should work (backwards compat)
    let result = parse_agent_identifier("my-agent").unwrap();

    assert_eq!(result.repo_slug, None);
    assert_eq!(result.label, "my-agent".to_string());
}

#[test]
fn test_parse_agent_identifier_invalid() {
    use crowdcontrol_core::repo::parse_agent_identifier;

    // Empty string
    assert!(parse_agent_identifier("").is_err());

    // Multiple colons
    assert!(parse_agent_identifier("a:b:c").is_err());

    // Empty repo slug
    assert!(parse_agent_identifier(":label").is_err());

    // Empty label
    assert!(parse_agent_identifier("repo:").is_err());
}

#[test]
fn test_agent_identifier_to_filesystem_name() {
    use crowdcontrol_core::repo::AgentIdentifier;

    let id = AgentIdentifier {
        repo_slug: Some("myapp".to_string()),
        label: "main".to_string(),
    };

    // Filesystem name uses hyphen instead of colon
    assert_eq!(id.to_filesystem_name(), "myapp-main");
}

#[test]
fn test_agent_identifier_display_name() {
    use crowdcontrol_core::repo::AgentIdentifier;

    let id = AgentIdentifier {
        repo_slug: Some("myapp".to_string()),
        label: "main".to_string(),
    };

    // Display name uses colon
    assert_eq!(id.to_display_name(), "myapp:main");
}

#[test]
fn test_agent_identifier_without_repo_names() {
    use crowdcontrol_core::repo::AgentIdentifier;

    let id = AgentIdentifier {
        repo_slug: None,
        label: "standalone-agent".to_string(),
    };

    // Without repo, both names are just the label
    assert_eq!(id.to_filesystem_name(), "standalone-agent");
    assert_eq!(id.to_display_name(), "standalone-agent");
}
