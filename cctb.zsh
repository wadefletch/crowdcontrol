#!/bin/zsh
# CrowdControl Tractorbeam function
# Add this to your ~/.zshrc or source it

cctb() {
    local agent_name="$1"
    local cc_cmd="CROWDCONTROL_WORKSPACES_DIR=~/Development/Tractorbeam/cc-workspaces crowdcontrol"
    local repo="tractorbeamai/monorepo"

    echo "🛸 Creating agent: $agent_name"
    if eval "$cc_cmd new $agent_name git@github.com:$repo"; then

        echo "🚀 Starting agent..."
        if eval "$cc_cmd start $agent_name"; then

            # Automatically refresh Claude auth
            echo "🔐 Setting up Claude authentication..."
            if [[ "$OSTYPE" == "darwin"* ]]; then
                # macOS: use keychain extraction
                if eval "$cc_cmd refresh $agent_name --extract-keychain" >/dev/null 2>&1; then
                    echo "✅ Claude authentication configured"
                else
                    echo "⚠️  Could not auto-configure Claude auth (you may need to run 'claude login' in the container)"
                fi
            else
                # Linux/other: refresh without keychain
                if eval "$cc_cmd refresh $agent_name" >/dev/null 2>&1; then
                    echo "✅ Claude authentication configured"
                else
                    echo "⚠️  Could not auto-configure Claude auth (you may need to run 'claude login' in the container)"
                fi
            fi

            echo "🔌 Connecting to agent..."
            eval "$cc_cmd connect $agent_name"
        else
            echo "❌ Failed to start agent: $agent_name"
            return 1
        fi
    else
        echo "❌ Failed to create agent: $agent_name"
        return 1
    fi
}

# Also create the alias for convenience
alias cc="CROWDCONTROL_WORKSPACES_DIR=~/Development/Tractorbeam/cc-workspaces crowdcontrol"
