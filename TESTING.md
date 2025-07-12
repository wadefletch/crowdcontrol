# Testing CrowdControl

This document describes the test suite for CrowdControl, including unit tests, integration tests, and Docker-specific tests.

## Running Tests

### Unit Tests
Run all unit tests:
```bash
cargo test
```

### Integration Tests (Requires Docker)
Run integration tests that require Docker:
```bash
cargo test -- --ignored --test-threads=1
```

### Specific Test Categories

#### Claude Authentication Tests
Tests for Claude Code credential mounting and refresh functionality:

1. **Unit Tests** (`crowdcontrol-core/tests/docker_claude_tests.rs`):
   - `test_claude_config_mount` - Verifies Claude config is mounted when present
   - `test_no_claude_config_no_mount` - Verifies no mount when config absent
   - `test_container_has_crowdcontrol_label` - Verifies container labeling

2. **Integration Tests** (`crowdcontrol-cli/tests/integration_test.rs`):
   - `test_refresh_claude_credentials` - Full refresh command workflow
   - `test_refresh_command_requires_running_agent` - Validation tests

3. **Shell Script Tests** (`container/test-refresh-claude-auth.sh`):
   - Tests the refresh-claude-auth.sh script functionality
   - Run during Docker image build to ensure correctness

#### Docker Container Tests
- `test_full_agent_lifecycle` - Complete create/start/stop/remove cycle
- `test_agent_start_and_connect_issue` - Regression test for container startup
- `test_multiple_agents` - Concurrent agent management
- `test_resource_limits` - Memory and CPU limit configuration

## Test Structure

### Directory Layout
```
crowdcontrol/
├── crowdcontrol-core/
│   └── tests/
│       └── docker_claude_tests.rs    # Docker-specific unit tests
├── crowdcontrol-cli/
│   └── tests/
│       ├── integration_test.rs       # Full integration tests
│       └── claude_auth_test.rs       # CLI validation tests
└── container/
    └── test-refresh-claude-auth.sh   # Shell script tests
```

### Test Fixtures
Integration tests use a Node.js test repository fixture located at:
`crowdcontrol-cli/tests/fixtures/nodejs-test-repo/`

### Test Workspaces
Tests that create agents use a temporary `test-workspaces/` directory that is gitignored to avoid polluting the repository.

## Running Individual Tests

Run a specific test:
```bash
cargo test test_refresh_claude_credentials -- --ignored --nocapture
```

Run tests with detailed output:
```bash
RUST_LOG=debug cargo test -- --ignored --nocapture
```

## Docker Image Testing

The Docker image includes tests that run during build:
```bash
cd container
docker build -t crowdcontrol:latest .
```

If the build succeeds, all container scripts have passed their tests.

## CI/CD Considerations

When setting up CI/CD, ensure:
1. Docker is available in the CI environment
2. Tests are run with `--test-threads=1` to avoid conflicts
3. The Docker image is built before running integration tests
4. Temporary test workspaces are cleaned up after tests

## Troubleshooting

### Common Issues

1. **"Docker not found" errors**
   - Ensure Docker is installed and running
   - Check `docker info` works from command line

2. **Permission errors**
   - Ensure your user has Docker permissions
   - On Linux: `sudo usermod -aG docker $USER`

3. **Container startup failures**
   - Check Docker logs: `docker logs crowdcontrol-<agent-name>`
   - Verify the Docker image is up to date

4. **Claude authentication tests failing**
   - Tests create mock credentials - ensure ~/.claude/ is writable
   - Real credentials are backed up and restored automatically