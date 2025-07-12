#!/bin/bash
# Refresh Claude Code authentication from host mount or provided credentials

set -e

USER_ID=${HOST_UID:-1000}
GROUP_ID=${HOST_GID:-1000}

# If credentials are provided as first parameter, use them directly
KEYCHAIN_CREDENTIALS="$1"

echo "Refreshing Claude Code authentication..."

# Create target directory
mkdir -p /home/developer/.claude

# If keychain credentials were provided as parameter, use them (highest priority)
if [ -n "$KEYCHAIN_CREDENTIALS" ]; then
    echo "Using provided keychain credentials..."
    echo "$KEYCHAIN_CREDENTIALS" > /home/developer/.claude/.credentials.json
    chown $USER_ID:$GROUP_ID /home/developer/.claude/.credentials.json
    chmod 600 /home/developer/.claude/.credentials.json
    echo "✅ Claude Code authentication configured (keychain credentials)"
    exit 0
fi

# Otherwise, try to copy from mounted files (for Linux/file-based auth)
# Copy .credentials.json if it exists
if [ -f "/mnt/claude-config/.claude/.credentials.json" ]; then
    echo "Copying .credentials.json from mount..."
    cp /mnt/claude-config/.claude/.credentials.json /home/developer/.claude/.credentials.json
    chown $USER_ID:$GROUP_ID /home/developer/.claude/.credentials.json
    chmod 600 /home/developer/.claude/.credentials.json
    echo "✅ Claude Code authentication configured (.credentials.json)"
    exit 0
fi

# Copy .claude.json if it exists (legacy format)
if [ -f "/mnt/claude-config/.claude.json" ]; then
    echo "Copying .claude.json from mount (legacy format)..."
    cp /mnt/claude-config/.claude.json /home/developer/.claude.json
    
    # Apply jq transformations: disable autoUpdates, reset projects, set mode to global
    echo "Applying transformations to .claude.json..."
    jq '.autoUpdates = false | .projects = {} | .mode = "global"' /home/developer/.claude.json > /home/developer/.claude.json.tmp
    mv /home/developer/.claude.json.tmp /home/developer/.claude.json
    
    chown $USER_ID:$GROUP_ID /home/developer/.claude.json
    chmod 600 /home/developer/.claude.json
    echo "✅ Claude Code authentication configured (.claude.json with transformations)"
    exit 0
fi

echo "⚠️  No Claude Code credentials found"
echo "   Expected: ~/.claude/credentials.json, ~/.claude.json, or keychain credentials"
echo "   On macOS, run: crowdcontrol refresh <agent-name> --extract-keychain"
exit 1