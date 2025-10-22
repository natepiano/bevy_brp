#!/bin/bash
# Post-Bash hook: Capture mutation test operation announcements
#
# When it sees: : "Starting operation 5 on port 30001"
# It writes: operation_id=5 to /tmp/mutation_test_op_30001.txt

# Read the JSON input from stdin
INPUT=$(cat)

# Extract the bash command
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Check if command is empty
if [ -z "$COMMAND" ]; then
    echo '{"continue": true}'
    exit 0
fi

# Check if this is a mutation test operation announcement
# Pattern: : "Starting operation N on port P"
if echo "$COMMAND" | grep -qE '^\s*:\s*"Starting operation [0-9]+ on port [0-9]+"'; then
    # Extract operation_id and port
    OPERATION_ID=$(echo "$COMMAND" | sed -n 's/.*Starting operation \([0-9]*\) on port.*/\1/p')
    PORT=$(echo "$COMMAND" | sed -n 's/.*on port \([0-9]*\).*/\1/p')

    # Validate we extracted both values
    if [ -n "$OPERATION_ID" ] && [ -n "$PORT" ]; then
        # Write operation_id to port-specific temp file
        echo "$OPERATION_ID" > "/tmp/mutation_test_op_${PORT}.txt"
    fi
fi

# Always continue
echo '{"continue": true}'
exit 0
