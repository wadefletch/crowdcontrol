#!/bin/bash
# Refresh Claude Code authentication from host mount

set -e

USER_ID=${HOST_UID:-1000}
GROUP_ID=${HOST_GID:-1000}

echo "Refreshing Claude Code authentication..."

# Create target directory
mkdir -p /home/developer/.claude

# Copy credentials.json if it exists
if [ -f "/mnt/claude-config/credentials.json" ]; then
    echo "Copying credentials.json..."
    cp /mnt/claude-config/credentials.json /home/developer/.claude/credentials.json
    chown $USER_ID:$GROUP_ID /home/developer/.claude/credentials.json
    chmod 600 /home/developer/.claude/credentials.json
    echo "✅ Claude Code authentication configured"
    exit 0
fi

echo "⚠️  No Claude Code credentials found"
echo "   Expected: ~/.claude/credentials.json"
exit 1