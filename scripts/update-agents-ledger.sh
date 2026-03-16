#!/bin/bash

# Auto-update AGENTS.md ledger on sprint completion
# Usage: ./scripts/update-agents-ledger.sh

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║     AGENTS.md Ledger Update                               ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

LEDGER_FILE="AGENTS.md"
TEMP_FILE="AGENTS.md.tmp"
DATE=$(date +%Y-%m-%d)

# Check if ledger file exists
if [ ! -f "$LEDGER_FILE" ]; then
    echo "ERROR: Ledger file not found: $LEDGER_FILE"
    exit 1
fi

echo "Updating ledger as of $DATE..."
echo ""

# Copy current ledger to temp file
cp "$LEDGER_FILE" "$TEMP_FILE"

# Track updates
UPDATES=0

# Process each agent's done folder
for agent_dir in planning/agent-*/done; do
    if [ ! -d "$agent_dir" ]; then
        continue
    fi
    
    agent_name=$(basename "$(dirname "$agent_dir")" | sed 's/agent-//')
    agent_upper=$(echo "$agent_name" | tr '[:lower:]' '[:upper:]')
    
    echo "Processing Agent $agent_upper..."
    
    for sprint_file in "$agent_dir"/AGENT-${agent_upper}-SPRINT-*.md; do
        if [ ! -f "$sprint_file" ]; then
            continue
        fi
        
        # Extract sprint number (portable - no grep -P)
        sprint_num=$(basename "$sprint_file" | sed 's/.*SPRINT-\([0-9]*\).*/\1/')
        if [ -z "$sprint_num" ] || [ "$sprint_num" = "$(basename "$sprint_file")" ]; then
            continue
        fi
        
        # Extract sprint title from file
        sprint_title=$(grep -m1 "^#" "$sprint_file" | sed 's/^#.*: //' | cut -d'(' -f1 | xargs 2>/dev/null || echo "Sprint $sprint_num")
        
        # Check if already in ledger
        if grep -q "| *$sprint_num *| *$agent_upper" "$LEDGER_FILE" 2>/dev/null; then
            echo "  ✓ Sprint $sprint_num already in ledger"
            continue
        fi
        
        # Add to sprint history (append to appropriate agent section)
        echo "| $sprint_num | $agent_upper | $sprint_title | ✅ Complete | N/A |" >> "$TEMP_FILE"
        echo "  + Added Sprint $sprint_num: $sprint_title"
        UPDATES=$((UPDATES + 1))
    done
done

echo ""

# Check for recent status files (in progress sprints)
for agent_dir in planning/agent-*; do
    if [ ! -d "$agent_dir" ]; then
        continue
    fi
    
    agent_name=$(basename "$agent_dir" | sed 's/agent-//')
    agent_upper=$(echo "$agent_name" | tr '[:lower:]' '[:upper:]')
    
    status_file="$agent_dir/AGENT-${agent_upper}-STATUS.md"
    if [ -f "$status_file" ]; then
        current_sprint=$(grep -o 'Sprint [0-9]*' "$status_file" 2>/dev/null | head -1 | sed 's/Sprint //' || echo "")
        if [ -n "$current_sprint" ]; then
            echo "Agent $agent_upper: Sprint $current_sprint (In Progress)"
        fi
    fi
done

echo ""
echo "═══════════════════════════════════════════════════════════"
echo ""

# Replace original ledger with updated version
if [ $UPDATES -gt 0 ]; then
    mv "$TEMP_FILE" "$LEDGER_FILE"
    echo "✅ Ledger updated: $UPDATES new sprint(s) added"
else
    rm "$TEMP_FILE"
    echo "✓ Ledger is up to date (no new sprints)"
fi

echo ""
echo "Last Updated: $DATE"
echo ""
