#!/bin/bash
# Refresh Claude Code authentication from host mount

set -e

USER_ID=${HOST_UID:-1000}
GROUP_ID=${HOST_GID:-1000}

echo "Refreshing Claude Code authentication..."

# Create target directory
mkdir -p /home/developer/.claude

# Copy credentials.json if it exists (preferred)
if [ -f "/mnt/host-claude-config/.claude/credentials.json" ]; then
    echo "Copying credentials.json..."
    cp /mnt/host-claude-config/.claude/credentials.json /home/developer/.claude/credentials.json
    chown $USER_ID:$GROUP_ID /home/developer/.claude/credentials.json
    chmod 600 /home/developer/.claude/credentials.json
    echo "✅ Claude Code authentication configured (credentials.json)"
    exit 0
fi

# Copy .claude.json if credentials.json doesn't exist (legacy support)
if [ -f "/mnt/host-claude-config/.claude.json" ]; then
    echo "Copying .claude.json (legacy format)..."
    cp /mnt/host-claude-config/.claude.json /home/developer/.claude.json
    chown $USER_ID:$GROUP_ID /home/developer/.claude.json
    chmod 600 /home/developer/.claude.json
    echo "✅ Claude Code authentication configured (.claude.json)"
    exit 0
fi

echo "⚠️  No Claude Code authentication found in host mount"
echo "   Expected: ~/.claude/credentials.json or ~/.claude.json"
exit 1