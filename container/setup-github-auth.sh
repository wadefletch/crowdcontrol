#!/bin/bash
# /usr/local/bin/setup-github-auth.sh
# Set up GitHub authentication for CrowdControl containers

set -e

USER_ID=${HOST_UID:-1000}
GROUP_ID=${HOST_GID:-1000}

# Function to run command as developer user
run_as_developer() {
    if [ "$(id -u)" = "0" ]; then
        su developer -c "$1"
    else
        bash -c "$1"
    fi
}

echo "Setting up GitHub authentication..."

# Check if GitHub configuration is available
if [ -z "$GITHUB_INSTALLATION_TOKEN" ] && [ -z "$GITHUB_APP_ID" ]; then
    echo "No GitHub configuration found (GITHUB_INSTALLATION_TOKEN or GITHUB_APP_ID not set)"
    exit 1
fi

# Configure git globally to use HTTPS instead of SSH (supports GitHub Enterprise)
GITHUB_BASE_URL=${GITHUB_BASE_URL:-"https://github.com"}
GITHUB_HOST=$(echo "$GITHUB_BASE_URL" | sed 's|https://||')
run_as_developer "git config --global url.\"$GITHUB_BASE_URL/\".insteadOf \"git@$GITHUB_HOST:\""

# Set up credential helper
run_as_developer 'git config --global credential.helper store'

# Configure CrowdControl as the git user
if [ -n "$GITHUB_USER_NAME" ]; then
    run_as_developer "git config --global user.name \"$GITHUB_USER_NAME\""
else
    run_as_developer 'git config --global user.name "CrowdControl[bot]"'
fi

if [ -n "$GITHUB_USER_EMAIL" ]; then
    run_as_developer "git config --global user.email \"$GITHUB_USER_EMAIL\""
else
    run_as_developer 'git config --global user.email "crowdcontrol[bot]@users.noreply.github.com"'
fi

# Set up GitHub credentials
if [ -n "$GITHUB_INSTALLATION_TOKEN" ]; then
    echo "Configuring GitHub with installation token for $GITHUB_BASE_URL..."
    
    # Create credentials file with proper ownership
    CREDS_FILE="/home/developer/.git-credentials"
    echo "https://x-access-token:$GITHUB_INSTALLATION_TOKEN@$GITHUB_HOST" > "$CREDS_FILE"
    
    # Set proper ownership and permissions
    chown "$USER_ID:$GROUP_ID" "$CREDS_FILE"
    chmod 600 "$CREDS_FILE"
    
    echo "GitHub authentication configured successfully with installation token for $GITHUB_BASE_URL"
    
elif [ -n "$GITHUB_APP_ID" ] && [ -n "$GITHUB_INSTALLATION_ID" ]; then
    echo "GitHub App configuration detected (app_id: $GITHUB_APP_ID, installation_id: $GITHUB_INSTALLATION_ID)"
    echo "Note: Token refresh will be handled by the application as needed"
    
    # For now, we'll implement this later when we add JWT token generation
    echo "JWT-based token refresh not yet implemented"
    
else
    echo "Invalid GitHub configuration"
    exit 1
fi

echo "GitHub authentication setup complete"