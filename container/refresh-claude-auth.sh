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

# Check if credentials were already copied by entrypoint
if [ -f "/home/developer/.claude/.credentials.json" ]; then
    echo "✅ Claude Code authentication already configured"
    exit 0
fi

# Try to copy from mount (fallback if entrypoint didn't run)
if [ -f "/mnt/host-claude/.credentials.json" ]; then
    echo "Copying .credentials.json from mount..."
    mkdir -p /home/developer/.claude
    cp -L /mnt/host-claude/.credentials.json /home/developer/.claude/.credentials.json
    chown $USER_ID:$GROUP_ID /home/developer/.claude/.credentials.json
    chmod 600 /home/developer/.claude/.credentials.json
    echo "✅ Claude Code authentication configured (.credentials.json)"
    exit 0
fi

echo "⚠️  No Claude Code credentials found"
echo "   Expected: ~/.claude/.credentials.json, ~/.claude.json, or keychain credentials"
echo "   "
echo "   To authenticate Claude Code in the container:"
echo "   1. On macOS: crowdcontrol refresh <agent-name> --extract-keychain"
echo "   2. On Linux: Ensure ~/.claude/.credentials.json exists on host"
echo "   3. Or run 'claude login' inside the container manually"
echo "   "
echo "   Note: CLAUDE_CONFIG_DIR is set to: ${CLAUDE_CONFIG_DIR:-/home/developer}"
# Don't exit with error - allow container to continue
exit 0