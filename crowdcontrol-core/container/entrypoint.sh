#!/bin/bash
# /entrypoint.sh

set -e

echo "Starting crowdcontrol development environment..."

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
echo "Connect with: docker exec -it \$CONTAINER_NAME claude --dangerously-skip-permissions"

# Keep container running
tail -f /dev/null