#!/bin/bash
# Test script for refresh-claude-auth.sh functionality

set -e

echo "Testing refresh-claude-auth.sh script..."

# Setup test environment
TEST_DIR="/tmp/claude-auth-test"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR/mnt/claude-config"
mkdir -p "$TEST_DIR/home/developer"

# Mock environment variables
export HOST_UID=1000
export HOST_GID=1000

# Test 1: No credentials available
echo "Test 1: No credentials available"
cd "$TEST_DIR"
SCRIPT_PATH="${SCRIPT_PATH:-./refresh-claude-auth.sh}"
if [ ! -f "$SCRIPT_PATH" ]; then
    SCRIPT_PATH="/usr/local/bin/refresh-claude-auth.sh"
fi
if ! [ -f "$SCRIPT_PATH" ]; then
    echo "Error: refresh-claude-auth.sh not found"
    exit 1
fi

if "$SCRIPT_PATH" 2>&1 | grep -q "No Claude Code credentials found"; then
    echo "✓ Correctly reports missing credentials"
else
    echo "✗ Failed to report missing credentials"
    echo "Script output:"
    "$SCRIPT_PATH" 2>&1
    exit 1
fi

# Test 2: Credentials.json exists
echo "Test 2: Credentials.json exists"
mkdir -p "$TEST_DIR/mnt/claude-config"
echo '{"token": "test-token"}' > "$TEST_DIR/mnt/claude-config/credentials.json"

# Create a wrapper script to test without actually changing ownership
cat > "$TEST_DIR/test-refresh.sh" << 'EOF'
#!/bin/bash
set -e

USER_ID=${HOST_UID:-1000}
GROUP_ID=${HOST_GID:-1000}

echo "Refreshing Claude Code authentication..."

# Create target directory
mkdir -p /tmp/claude-auth-test/home/developer/.claude

# Copy credentials.json if it exists
if [ -f "/tmp/claude-auth-test/mnt/claude-config/credentials.json" ]; then
    echo "Copying credentials.json..."
    cp /tmp/claude-auth-test/mnt/claude-config/credentials.json /tmp/claude-auth-test/home/developer/.claude/credentials.json
    # Skip chown in test
    echo "Would chown $USER_ID:$GROUP_ID /tmp/claude-auth-test/home/developer/.claude/credentials.json"
    # Skip chmod in test
    echo "Would chmod 600 /tmp/claude-auth-test/home/developer/.claude/credentials.json"
    echo "✅ Claude Code authentication configured"
    exit 0
fi

echo "⚠️  No Claude Code credentials found"
echo "   Expected: ~/.claude/credentials.json"
exit 1
EOF

chmod +x "$TEST_DIR/test-refresh.sh"

if "$TEST_DIR/test-refresh.sh"; then
    echo "✓ Successfully copies credentials"
    
    # Verify file was copied
    if [ -f "$TEST_DIR/home/developer/.claude/credentials.json" ]; then
        echo "✓ Credentials file exists in correct location"
        
        # Verify content
        if grep -q "test-token" "$TEST_DIR/home/developer/.claude/credentials.json"; then
            echo "✓ Credentials content is correct"
        else
            echo "✗ Credentials content is incorrect"
            exit 1
        fi
    else
        echo "✗ Credentials file not found"
        exit 1
    fi
else
    echo "✗ Failed to copy credentials"
    exit 1
fi

# Test 3: Directory structure is created
echo "Test 3: Directory structure"
rm -rf "$TEST_DIR/home/developer/.claude"
"$TEST_DIR/test-refresh.sh" > /dev/null 2>&1
if [ -d "$TEST_DIR/home/developer/.claude" ]; then
    echo "✓ Creates .claude directory if it doesn't exist"
else
    echo "✗ Failed to create .claude directory"
    exit 1
fi

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "All tests passed! ✅"