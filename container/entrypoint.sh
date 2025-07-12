#!/bin/bash
# /entrypoint.sh

set -e

echo "Starting crowdcontrol development environment..."

# Setup user with host UID/GID if running as root
if [ "$(id -u)" = "0" ]; then
    # Use environment variables for UID/GID if provided, fallback to defaults
    USER_ID=${HOST_UID:-1000}
    GROUP_ID=${HOST_GID:-1000}
    
    # Update developer user with host UID/GID
    groupmod -g $GROUP_ID developer 2>/dev/null || true
    usermod -u $USER_ID -g $GROUP_ID developer 2>/dev/null || true
    
    # Fix ownership of home directory
    find /home/developer -mindepth 1 -maxdepth 1 \
        -exec chown -R $USER_ID:$GROUP_ID {} \; 2>/dev/null || true
    
    # Ensure home directory itself has correct ownership
    chown $USER_ID:$GROUP_ID /home/developer
    
    # Setup Claude Code authentication using the refresh script
    /usr/local/bin/refresh-claude-auth.sh || echo "   (This is normal if Claude Code isn't configured on the host)"
    
    # Setup GitHub authentication if configured
    /usr/local/bin/setup-github-auth.sh || echo "   (This is normal if GitHub isn't configured)"
    
    # Start docker daemon in background
    dockerd &
    
    # Wait for docker to be ready
    echo "Waiting for Docker daemon..."
    timeout=30
    while ! docker info >/dev/null 2>&1; do
        sleep 1
        timeout=$((timeout - 1))
        if [ $timeout -eq 0 ]; then
            echo "Docker daemon failed to start"
            exit 1
        fi
    done
    
    echo "Docker daemon ready"
    
    # Switch to developer user for the rest of the script
    exec su developer "$0" "$@"
fi

# Now running as developer user

# Find the repository directory (should be only subdirectory in /workspace)
REPO_DIR=$(find /workspace -maxdepth 1 -type d ! -path /workspace | head -1)

if [ -z "$REPO_DIR" ]; then
    echo "No repository directory found in /workspace"
    echo "Container ready for manual setup"
    tail -f /dev/null
fi

cd "$REPO_DIR"
REPO_NAME=$(basename "$REPO_DIR")
echo "Working in: $REPO_DIR"

# Export environment variables for scripts
export CROWDCONTROL_REPO_PATH="$REPO_DIR"
export CROWDCONTROL_REPO_NAME="$REPO_NAME"
export CROWDCONTROL_WORKSPACE="/workspace"

# Set Claude config directory to home
export CLAUDE_CONFIG_DIR="/home/developer"

# Run repository-specific setup if it exists and hasn't been run
if [ -f ".crowdcontrol/setup.sh" ] && [ ! -f ".crowdcontrol/.setup-complete" ]; then
    echo "Running repository setup for $REPO_NAME..."
    chmod +x .crowdcontrol/setup.sh
    ./.crowdcontrol/setup.sh
    if [ $? -eq 0 ]; then
        touch .crowdcontrol/.setup-complete
        echo "Setup completed successfully"
    else
        echo "Setup failed"
        exit 1
    fi
fi

# Run repository-specific start script if it exists
if [ -f ".crowdcontrol/start.sh" ]; then
    echo "Running repository start script for $REPO_NAME..."
    chmod +x .crowdcontrol/start.sh
    ./.crowdcontrol/start.sh
    if [ $? -eq 0 ]; then
        echo "Repository services started successfully"
    else
        echo "Start script failed"
        exit 1
    fi
fi

echo "Container ready for development"
echo "Connect with: docker exec -it \$CONTAINER_NAME claude"

# Keep container running
tail -f /dev/null