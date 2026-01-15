// TDD: These tests define the expected behavior for the Project module
// Run with: cargo test --package crowdcontrol-core project_tests

use anyhow::Result;
use tempfile::TempDir;

// These imports will fail until we implement the module
use crowdcontrol_core::project::{Project, ProjectStore};

fn create_test_project_store() -> (ProjectStore, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let projects_path = temp_dir.path().join("projects.toml");
    let store = ProjectStore::new(projects_path);
    (store, temp_dir)
}

#[test]
fn test_project_struct_has_required_fields() {
    let project = Project {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: Some("main".to_string()),
    };

    assert_eq!(project.slug, "myapp");
    assert_eq!(project.url, "git@github.com:org/myapp.git");
    assert_eq!(project.default_branch, Some("main".to_string()));
}

#[test]
fn test_project_without_default_branch() {
    let project = Project {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: None,
    };

    assert_eq!(project.default_branch, None);
}

#[test]
fn test_add_project_to_store() -> Result<()> {
    let (mut store, _temp_dir) = create_test_project_store();

    let project = Project {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: Some("main".to_string()),
    };

    store.add(project)?;

    // Verify it was added
    let loaded = store.get("myapp")?;
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().url, "git@github.com:org/myapp.git");

    Ok(())
}

#[test]
fn test_get_nonexistent_project_returns_none() -> Result<()> {
    let (store, _temp_dir) = create_test_project_store();

    let result = store.get("nonexistent")?;
    assert!(result.is_none());

    Ok(())
}

#[test]
fn test_list_projects_empty() -> Result<()> {
    let (store, _temp_dir) = create_test_project_store();

    let projects = store.list()?;
    assert!(projects.is_empty());

    Ok(())
}

#[test]
fn test_list_projects_returns_all() -> Result<()> {
    let (mut store, _temp_dir) = create_test_project_store();

    store.add(Project {
        slug: "app1".to_string(),
        url: "git@github.com:org/app1.git".to_string(),
        default_branch: None,
    })?;

    store.add(Project {
        slug: "app2".to_string(),
        url: "git@github.com:org/app2.git".to_string(),
        default_branch: Some("develop".to_string()),
    })?;

    let projects = store.list()?;
    assert_eq!(projects.len(), 2);

    Ok(())
}

#[test]
fn test_remove_project() -> Result<()> {
    let (mut store, _temp_dir) = create_test_project_store();

    store.add(Project {
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
fn test_remove_nonexistent_project_returns_error() {
    let (mut store, _temp_dir) = create_test_project_store();

    let result = store.remove("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_add_duplicate_slug_returns_error() -> Result<()> {
    let (mut store, _temp_dir) = create_test_project_store();

    store.add(Project {
        slug: "myapp".to_string(),
        url: "git@github.com:org/myapp.git".to_string(),
        default_branch: None,
    })?;

    // Adding another project with the same slug should fail
    let result = store.add(Project {
        slug: "myapp".to_string(),
        url: "git@github.com:other/myapp.git".to_string(),
        default_branch: None,
    });

    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_project_persists_across_store_instances() -> Result<()> {
    let temp_dir = tempfile::tempdir().unwrap();
    let projects_path = temp_dir.path().join("projects.toml");

    // Create first store and add project
    {
        let mut store = ProjectStore::new(projects_path.clone());
        store.add(Project {
            slug: "myapp".to_string(),
            url: "git@github.com:org/myapp.git".to_string(),
            default_branch: Some("main".to_string()),
        })?;
    }

    // Create new store instance and verify project is still there
    {
        let store = ProjectStore::new(projects_path);
        let project = store.get("myapp")?;
        assert!(project.is_some());
        assert_eq!(project.unwrap().url, "git@github.com:org/myapp.git");
    }

    Ok(())
}

#[test]
fn test_validate_slug_valid() {
    use crowdcontrol_core::project::validate_slug;

    assert!(validate_slug("myapp").is_ok());
    assert!(validate_slug("my-app").is_ok());
    assert!(validate_slug("my_app").is_ok());
    assert!(validate_slug("app123").is_ok());
}

#[test]
fn test_validate_slug_invalid() {
    use crowdcontrol_core::project::validate_slug;

    // Cannot contain colon (reserved for project:label separator)
    assert!(validate_slug("my:app").is_err());

    // Cannot be empty
    assert!(validate_slug("").is_err());

    // Cannot contain spaces
    assert!(validate_slug("my app").is_err());

    // Cannot contain slashes
    assert!(validate_slug("my/app").is_err());
}

#[test]
fn test_parse_agent_identifier_with_project() {
    use crowdcontrol_core::project::parse_agent_identifier;

    let result = parse_agent_identifier("myapp:main").unwrap();

    assert_eq!(result.project_slug, Some("myapp".to_string()));
    assert_eq!(result.label, "main".to_string());
}

#[test]
fn test_parse_agent_identifier_without_project() {
    use crowdcontrol_core::project::parse_agent_identifier;

    // Plain name without colon should work (backwards compat)
    let result = parse_agent_identifier("my-agent").unwrap();

    assert_eq!(result.project_slug, None);
    assert_eq!(result.label, "my-agent".to_string());
}

#[test]
fn test_parse_agent_identifier_invalid() {
    use crowdcontrol_core::project::parse_agent_identifier;

    // Empty string
    assert!(parse_agent_identifier("").is_err());

    // Multiple colons
    assert!(parse_agent_identifier("a:b:c").is_err());

    // Empty project slug
    assert!(parse_agent_identifier(":label").is_err());

    // Empty label
    assert!(parse_agent_identifier("project:").is_err());
}

#[test]
fn test_agent_identifier_to_filesystem_name() {
    use crowdcontrol_core::project::AgentIdentifier;

    let id = AgentIdentifier {
        project_slug: Some("myapp".to_string()),
        label: "main".to_string(),
    };

    // Filesystem name uses hyphen instead of colon
    assert_eq!(id.to_filesystem_name(), "myapp-main");
}

#[test]
fn test_agent_identifier_display_name() {
    use crowdcontrol_core::project::AgentIdentifier;

    let id = AgentIdentifier {
        project_slug: Some("myapp".to_string()),
        label: "main".to_string(),
    };

    // Display name uses colon
    assert_eq!(id.to_display_name(), "myapp:main");
}

#[test]
fn test_agent_identifier_without_project_names() {
    use crowdcontrol_core::project::AgentIdentifier;

    let id = AgentIdentifier {
        project_slug: None,
        label: "standalone-agent".to_string(),
    };

    // Without project, both names are just the label
    assert_eq!(id.to_filesystem_name(), "standalone-agent");
    assert_eq!(id.to_display_name(), "standalone-agent");
}
