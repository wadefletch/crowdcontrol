# Release Setup

This document outlines the one-time setup required to enable automated releases.

## Prerequisites

You need to configure authentication tokens for automated publishing to work.

## Required GitHub Secrets

Add these secrets at: https://github.com/wadefletch/crowdcontrol/settings/secrets/actions

### 1. CARGO_REGISTRY_TOKEN

Get your crates.io API token:

```bash
# Method 1: CLI login (will prompt for token)
cargo login

# Method 2: Get token from website
# Visit: https://crates.io/settings/tokens
# Create a new token with "Publish new crates" permission
```

### 2. NPM_TOKEN  

Get your npm authentication token:

```bash
# Login to npm
npm login

# Create a publish token
npm token create
# Choose: "Publish" when prompted for token type
```

## How Releases Work

Once secrets are configured, releases are **fully automated**:

- **Automatic**: Every push to `main` with `feat`/`fix`/`perf` commits triggers a release
- **Manual**: Go to Actions tab > Release workflow > "Run workflow" 
- **Versioning**: Follows semantic versioning based on conventional commits

## What Gets Published

- **Cargo**: `crowdcontrol-cli` crate → users install with `cargo install crowdcontrol-cli`
- **NPM**: `@wadefletch/crowdcontrol` package → users install with `npm install -g @wadefletch/crowdcontrol`
- **GitHub**: Cross-platform binaries attached to releases

Both provide the same `crowdcontrol` binary.

## That's It!

After adding those two GitHub secrets, the entire release process is automated. Just use conventional commits and push to main.