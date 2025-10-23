#!/bin/bash
# Post-Tool hook: Automatically update mutation test operation status

# Read the JSON input from stdin
INPUT=$(cat)

# STEP 1: Extract port from tool_input
PORT=$(echo "$INPUT" | jq -r '.tool_input.port // empty')

# Extract tool name for logging
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // "unknown"')

# Get timestamp for detailed log entry
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

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

OPERATION_ID=$(grep "port=${PORT} tool=announcement" /tmp/mutation_hook_debug.log 2>/dev/null | tail -1 | sed -n 's/.*op_id=\([0-9]*\).*/\1/p')
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

# Log complete operation details (matches first log entry by timestamp)
echo "[${TIMESTAMP}] port=${PORT} tool=${TOOL_NAME} op_id=${OPERATION_ID} status=${STATUS}" >> /tmp/mutation_hook_debug.log

# STEP 5: Call operation_update.py with proper escaping
if [ "$STATUS" = "FAIL" ]; then
    # Extract error message and escape it for command line
    ERROR_MSG=$(echo "$INPUT" | jq -r '.tool_response[0].text | fromjson | .metadata.original_error // .message // "Unknown error"')
    # Use Python script with stdin for error message to avoid escaping issues
    python3 .claude/scripts/mutation_test/operation_update.py --port "$PORT" --operation-id "$OPERATION_ID" --status "$STATUS" --error "$ERROR_MSG"
else
    python3 .claude/scripts/mutation_test/operation_update.py --port "$PORT" --operation-id "$OPERATION_ID" --status "$STATUS"
fi
UPDATE_RESULT=$?

if [ $UPDATE_RESULT -eq 0 ]; then
    # Update succeeded - show icon based on tool call result
    if [ "$STATUS" = "SUCCESS" ]; then
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
