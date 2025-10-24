#!/bin/bash

# Read hook input
input=$(cat)

# STEP 0: Check if we're in the bevy_brp project root
# Look for the presence of the mutation test script directory
if [ ! -d ".claude/scripts/mutation_test" ]; then
    # Not in project root, silently exit
    echo '{"continue": true}'
    exit 0
fi

# Extract the bash command
command=$(echo "$input" | jq -r '.tool_input.command')

# Check if command contains BOTH a for loop AND mutation_test_operation_update.py
if echo "$command" | grep -qE '\bfor\s+\w+\s+in\b' && echo "$command" | grep -q 'mutation_test_operation_update.py'; then
  # Deny for loops with mutation_test_operation_update.py
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"For loops with mutation_test_operation_update.py are not allowed. You must execute mutation_test_operation_update.py commands one at a time. Do not use for loops or any batching mechanism. Call the script individually for each operation."}}'
else
  # Allow everything else
  echo '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}'
fi
