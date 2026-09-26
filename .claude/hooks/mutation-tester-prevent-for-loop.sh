#!/bin/bash

# Read hook input
input=$(cat)

# STEP 0: Check if we're in the bevy_brp project root
# Look for the presence of the mutation test script directory
if [ ! -d "${CLAUDE_PROJECT_DIR}/.claude/scripts/mutation_test" ]; then
    # Not in project root, silently exit
    echo '{"continue": true}'
    exit 0
fi

# Extract the bash command
command=$(echo "$input" | jq -r '.tool_input.command')

# Check if command contains jq operating on mutation test plan files
if echo "$command" | grep -q 'jq' && echo "$command" | grep -qE 'mutation_test_port_[0-9]+\.json'; then
  # Deny jq commands on mutation test plan files
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Direct jq operations on mutation_test_port_*.json files are not allowed. Use operation_manager.py --action get-next to retrieve operations, not direct file access with jq."}}'
# Allow operation_manager.py only as the exact single get-next call; loops, pipes and wrappers
# would consume operations without executing them
elif echo "$command" | grep -qE 'python3?[^|;&]*operation_manager\.py' && ! echo "$command" | grep -qE '^python3 \.claude/scripts/mutation_test/operation_manager\.py --port [0-9]+ --action get-next$'; then
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Run exactly: python3 .claude/scripts/mutation_test/operation_manager.py --port PORT --action get-next. No loops, pipes, or wrappers: every get-next call hands out an operation, and you must execute that operation with its MCP tool before calling get-next again."}}'
else
  # Allow everything else
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}'
fi
