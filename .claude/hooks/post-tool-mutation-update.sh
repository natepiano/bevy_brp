#!/bin/bash
# Post-Tool hook: Automatically update mutation test operation status

# Read the JSON input from stdin
INPUT=$(cat)

# STEP 0: Check if we're in the bevy_brp project root
# Look for the presence of the mutation test script directory
if [ ! -d ".claude/scripts/mutation_test" ]; then
    # Not in project root, silently exit
    echo '{"continue": true}'
    exit 0
fi

# STEP 1: Extract port from tool_input
PORT=$(echo "$INPUT" | jq -r '.tool_input.port // empty')

# Extract tool name for logging
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // "unknown"')

# STEP 2: Check if port is in mutation test range (30001-30010)
if [ -z "$PORT" ] || [ "$PORT" -lt 30001 ] || [ "$PORT" -gt 30010 ]; then
    MESSAGE="Hook: Port ${PORT} not in test range, skipping"
    echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
    exit 0
fi

# STEP 3: Read operation_id from debug log (most recent announcement for this port)
if [ ! -f /tmp/mutation_hook_debug.log ]; then
    MESSAGE="Hook: No debug log found at /tmp/mutation_hook_debug.log"
    echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
    exit 0
fi

OPERATION_ID=$(grep "port=${PORT} op_id=" /tmp/mutation_hook_debug.log 2>/dev/null | grep "is next" | tail -1 | sed -n 's/.*op_id=\([0-9]*\).*/\1/p')
if [ -z "$OPERATION_ID" ]; then
    MESSAGE="Hook: No announcement found for port ${PORT} in debug log"
    echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
    exit 0
fi

# STEP 4: Extract status from tool_response (it's in tool_response[0].text as JSON string)
STATUS_FIELD=$(echo "$INPUT" | jq -r '.tool_response[0].text | fromjson | .status // "unknown"')

# Determine SUCCESS or FAIL
if [ "$STATUS_FIELD" = "success" ]; then
    STATUS="SUCCESS"
else
    STATUS="FAIL"
fi

# Note: operation_update.py handles all logging, this hook just coordinates

# STEP 5: Call operation_update.py - pass full MCP response, get back final status
FINAL_STATUS=$(echo "$INPUT" | python3 .claude/scripts/mutation_test/operation_update.py \
    --port "$PORT" \
    --operation-id "$OPERATION_ID" \
    --tool-name "$TOOL_NAME" \
    --mcp-response -)

UPDATE_RESULT=$?

if [ $UPDATE_RESULT -eq 0 ]; then
    # Use final status from operation_update.py (after validation)
    if [ "$FINAL_STATUS" = "SUCCESS" ]; then
        MESSAGE="✅ Op ${OPERATION_ID}: SUCCESS"
    else
        MESSAGE="💥 Op ${OPERATION_ID}: FAIL"
    fi
else
    MESSAGE="⚠️ Failed to update op ${OPERATION_ID}"
fi

# Output message visible to both user and agent
echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
exit 0
