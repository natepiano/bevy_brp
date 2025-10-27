#!/bin/bash

# Read hook input
input=$(cat)

# STEP 0: Check if we're in the bevy_brp project root
# Look for the presence of the mutation test script directory
if [ ! -d "${CLAUDE_PROJECT_DIR}.claude/scripts/mutation_test" ]; then
    # Not in project root, silently exit
    echo '{"continue": true}'
    exit 0
fi

# Extract the bash command
command=$(echo "$input" | jq -r '.tool_input.command')

# Check if command contains jq operating on mutation test plan files
if echo "$command" | grep -q 'jq' && echo "$command" | grep -qE '/tmp/mutation_test_[0-9]+\.json'; then
  # Deny jq commands on mutation test plan files
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Direct jq operations on /tmp/mutation_test_*.json files are not allowed. Use operation_manager.py --action get-next to retrieve operations, not direct file access with jq."}}'
# Check if command contains for loop with operation_manager.py
elif echo "$command" | grep -qE '^\s*for\s+' && echo "$command" | grep -q 'operation_manager.py'; then
  # Deny for loops calling operation_manager.py
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"For loops calling operation_manager.py are not allowed. The operation_manager.py script should be called once per operation by the mutation test subagent, not in a batch loop. Each operation must be processed individually with proper error handling."}}'
else
  # Allow everything else
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}'
fi
