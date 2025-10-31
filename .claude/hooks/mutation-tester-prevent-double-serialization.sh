#!/bin/bash

# Read hook input
input=$(cat)

# Check if we're in the bevy_brp project root
if [ ! -d "${CLAUDE_PROJECT_DIR}/.claude/scripts/mutation_test" ]; then
    # Not in project root, silently allow
    echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}'
    exit 0
fi

# Extract the tool name
tool_name=$(echo "$input" | jq -r '.tool_name')

# Only check MCP query tools
if [[ "$tool_name" == "mcp__brp__world_query" ]]; then
  # Check if filter parameter is a string type (should be object/null)
  filter_type=$(echo "$input" | jq -r '.tool_input.filter | type')

  # If filter is a string type, it's double-serialized
  if [[ "$filter_type" == "string" ]]; then
    # Deny the call
    echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Double-serialized filter parameter detected. The filter parameter must be an OBJECT, not a STRING. You called json.dumps() on the filter dict. Extract filter from operation_manager.py output and pass it directly without json.dumps()."}}'
    exit 0
  fi
fi

# Allow everything else
echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}'
