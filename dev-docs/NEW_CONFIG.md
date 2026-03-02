# New Figment-Based Configuration System

This document defines the migration plan for replacing the current *settings / config* duo with a single, façade-agnostic configuration crate powered by **Figment**.  The change removes the legacy `CROWDCONTROL_*` variables and prepares the codebase for additional façades such as gRPC and REST.

## Motivation

The existing design spreads configuration logic across `settings.rs` (defaults & env), `config.rs` (validation), and each binary’s `main.rs` (CLI overrides).  That approach has started to leak internal details and will not scale when we introduce new client façades.  A central loader that owns the full merging pipeline eliminates duplication, guarantees consistent behaviour, and keeps Figment hidden behind a small public API.

## Crate & Directory Layout

```
crowdcontrol/
  crowdcontrol-config/     <-- new crate, houses Config + Loader
  crowdcontrol-core/       <-- now receives Arc<Config>
  crowdcontrol-cli/
  crowdcontrol-grpc/       <-- future façade
  crowdcontrol-rest/       <-- future façade
```

The new crate is added to the workspace in the root `Cargo.toml` and re-exported in `crowdcontrol-core` for external consumers.

:steps
### 1 — Create the `crowdcontrol-config` crate
Implement a `Loader` that *uses* Figment under the hood **but also re-exports** `figment::Figment` so advanced callers can compose their own sources when necessary.  The crate therefore exposes three public items: `Config`, `Loader`, and a helper `default_figment()` that returns the pre-seeded `Figment` instance.

### 2 — Establish default layers inside `Loader::new()`
Compile-time defaults → `$XDG_CONFIG_HOME/crowdcontrol/config.toml` (optional) → environment vars prefixed with `CC_`.

### 3 — Provide a façade-override hook
`pub fn merge<T: Serialize>(self, patch: &T) -> Self` allows any façade to inject its own overrides.  Power users can instead call `Figment::merge()` directly after `default_figment()` if they prefer to stay at the Figment level.

### 4 — Finalise loading
`pub fn load(self) -> Result<Config>` performs validation (e.g. ensure workspace dir exists) and returns an owned `Config`.

### 5 — Inject `Arc<Config>` everywhere
All entry points (`crowdcontrol-cli`, future gRPC, REST) construct a config via `Loader::new().merge(&overrides).load()?` and pass `Arc::new(cfg)` to `crowdcontrol-core` APIs.
:steps

## Public API Contract

````rust
// crowdcontrol-config/src/lib.rs

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    // — Global fields —
    pub workspaces_dir: PathBuf,
    pub image:          String,
    pub verbose:        u8,

    // Optional resource limits
    pub default_memory: Option<String>,
    pub default_cpus:   Option<String>,

    // — Namespaced groups —
    pub docker:    Docker,
    pub github:    Option<Github>,
    pub logging:   Logging,

    // Repo-specific overrides
    #[serde(default)]
    pub repos: std::collections::HashMap<String, RepoConfig>,
}

pub struct Loader { /* wraps a figment::Figment */ }

impl Loader {
    pub fn new() -> Self;                               // defaults + file + CC_* env
    pub fn env_prefix(self, p: impl Into<String>) -> Self;
    pub fn config_file(self, path: impl Into<PathBuf>) -> Self;
    pub fn merge<T: Serialize>(self, patch: &T) -> Self;
    pub fn load(self) -> Result<Config>;

    /// Access the inner Figment if a caller needs fine-grained control.
    pub fn into_figment(self) -> figment::Figment;
}
````


### Field Layout
* **Global keys** – `workspaces_dir`, `image`, `verbose`, `default_memory`, `default_cpus`
* **Nested tables** – `docker`, `github`, `logging`

This mix keeps common values easy to set while grouping advanced knobs.

### Repo-Specific Overrides

Real-world teams often spin up multiple agents for distinct repositories.  A *repo-specific* override layer lets users pin per-repository workspace paths, GitHub credentials, or resource limits.  The configuration file supports this via a `repos` table:

```toml
# $XDG_CONFIG_HOME/crowdcontrol/config.toml

workspaces_dir = "/Users/alice/dev/workspaces"
image          = "crowdcontrol:latest"

[docker]
default_memory = "4g"

[repos."github.com/org/first-repo"]
workspaces_dir = "/mnt/ssd/first-repo-workspaces"
github.token   = "env:CC_FIRST_REPO_TOKEN"   # environment indirection

[repos."github.com/org/second-repo"]
image = "crowdcontrol:cuda"
```

`Config` gains a map of these overrides:

```rust
pub struct Config {
    // global fields …
    #[serde(default)]
    pub repos: std::collections::HashMap<String, RepoConfig>,
}

pub struct RepoConfig { /* same shape as Config but all fields Optional */ }

impl Config {
    /// Returns the effective configuration for `repo`, merging the global
    /// settings with any repo-specific overrides and an optional caller patch.
    pub fn effective_for_repo<T: serde::Serialize>(
        &self,
        repo: &str,
        patch: Option<&T>,
    ) -> anyhow::Result<EffectiveConfig> { /* … */ }
}
```

Environment variables follow the pattern `CC_REPO_<SLUG>__<KEYS>`, for instance:

```
CC_REPO_GITHUB_COM_ORG_FIRST_REPO__WORKSPACES_DIR=/tmp/first
```

CLI façades pass the repository slug they are operating on so that `effective_for_repo()` can be called before launching Docker containers.

## Override Pattern in a Façade

```rust
#[derive(Serialize)]
struct CliOverrides {
    workspaces_dir: Option<PathBuf>,
    image:          Option<String>,
    verbose:        u8,
}

let overrides = CliOverrides { /* populate from clap */ };
let config = crowdcontrol_config::Loader::new()
    .merge(&overrides)
    .load()?;
```

Other façades follow the same pattern with their own override structs.

## Migration Checklist

* Remove `settings.rs` and `config.rs`; re-implement validation inside `crowdcontrol-config`.
* Delete or rename all uses of `CROWDCONTROL_*`; adopt `CC_*` with double underscores for nested fields.
* Refactor each existing command executor to accept `Arc<Config>` in place of the old `Config` value.
* Update tests to construct configs with the new loader and to feed fixture files or env vars via Figment.
* Update documentation and examples in `docs/` to show the new env var names and configuration file location.
* Document the `repos` table and `CC_REPO_*` environment variable convention.

## Testing Strategy

1. Unit tests inside `crowdcontrol-config` verify that the merging order is deterministic and that invalid files produce helpful errors.
2. Integration tests in `crowdcontrol-core` spin up a temporary directory, create a `config.toml`, and assert that all sub-systems behave the same as before.

## Roll-out Notes

Because we have no external users yet, we will land this migration under a single pull request and bump the workspace version to **`0.2.0`** to signal the breaking change. 