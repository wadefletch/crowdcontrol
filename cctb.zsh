#!/bin/zsh
# CrowdControl Tractorbeam function
# Add this to your ~/.zshrc or source it

cctb() {
    if [[ $# -ne 1 ]]; then
        echo "Usage: cctb <agent-name>"
        echo "This will start and connect to a CrowdControl agent"
        return 1
    fi
    
    local agent_name="$1"
    local cc_cmd="CROWDCONTROL_WORKSPACES_DIR=~/Development/Tractorbeam/cc-workspaces crowdcontrol"
    
    echo "🚀 Starting agent: $agent_name"
    
    # Start the agent
    if eval "$cc_cmd start $agent_name"; then
        echo "✅ Agent started successfully"
        echo "🔌 Connecting to agent..."
        
        # Connect to the agent
        eval "$cc_cmd connect $agent_name"
    else
        echo "❌ Failed to start agent: $agent_name"
        return 1
    fi
}

# Also create the alias for convenience
alias cc="CROWDCONTROL_WORKSPACES_DIR=~/Development/Tractorbeam/cc-workspaces crowdcontrol"