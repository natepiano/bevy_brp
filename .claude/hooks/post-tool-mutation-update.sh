#!/bin/bash
# Post-Tool hook: Automatically update mutation test operation status

# Read the JSON input from stdin
INPUT=$(cat)

# Check if we're in the bevy_brp project root
if [ ! -d ".claude/scripts/mutation_test" ]; then
    echo '{"continue": true}'
    exit 0
fi

# Extract port and tool name from tool_input
PORT=$(echo "$INPUT" | jq -r '.tool_input.port // empty')
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // "unknown"')

# Check if port is in mutation test range (30001-30010)
if [ -z "$PORT" ] || [ "$PORT" -lt 30001 ] || [ "$PORT" -gt 30010 ]; then
    MESSAGE="Hook: Port ${PORT} not in test range, skipping"
    echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
    exit 0
fi

# Call operation_manager.py to update status
MESSAGE=$(echo "$INPUT" | python3 .claude/scripts/mutation_test/operation_manager.py \
    --port "$PORT" \
    --action update \
    --tool-name "$TOOL_NAME" \
    --mcp-response - 2>&1)

UPDATE_RESULT=$?

if [ $UPDATE_RESULT -ne 0 ]; then
    MESSAGE="Hook: ${MESSAGE}"
fi

echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
exit 0
