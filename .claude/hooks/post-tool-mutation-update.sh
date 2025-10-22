#!/bin/bash
# Post-Tool hook: Automatically update mutation test operation status

# Read the JSON input from stdin
INPUT=$(cat)

# STEP 1: Extract port from tool_input
PORT=$(echo "$INPUT" | jq -r '.tool_input.port // empty')

# STEP 2: Check if port is in mutation test range (30001-30010)
if [ -z "$PORT" ] || [ "$PORT" -lt 30001 ] || [ "$PORT" -gt 30010 ]; then
    MESSAGE="Hook: Port ${PORT} not in test range, skipping"
    echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
    exit 0
fi

# STEP 3: Read operation_id from temp file
TEMP_FILE="/tmp/mutation_test_op_${PORT}.txt"
if [ ! -f "$TEMP_FILE" ]; then
    MESSAGE="Hook: No operation announcement found at ${TEMP_FILE}"
    echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
    exit 0
fi

OPERATION_ID=$(cat "$TEMP_FILE" 2>/dev/null)
if [ -z "$OPERATION_ID" ]; then
    MESSAGE="Hook: Empty operation_id in ${TEMP_FILE}"
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

# STEP 5: Call operation_update.py with proper escaping
if [ "$STATUS" = "FAIL" ]; then
    # Extract error message and escape it for command line
    ERROR_MSG=$(echo "$INPUT" | jq -r '.tool_response[0].text | fromjson | .metadata.original_error // .message // "Unknown error"')
    # Use Python script with stdin for error message to avoid escaping issues
    python3 .claude/scripts/mutation_test/operation_update.py --port "$PORT" --operation-id "$OPERATION_ID" --status "$STATUS" --error "$ERROR_MSG" > /tmp/mutation_hook_update_${PORT}_${OPERATION_ID}.log 2>&1
else
    python3 .claude/scripts/mutation_test/operation_update.py --port "$PORT" --operation-id "$OPERATION_ID" --status "$STATUS" > /tmp/mutation_hook_update_${PORT}_${OPERATION_ID}.log 2>&1
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
    MESSAGE="⚠️ Failed to update op ${OPERATION_ID} (see /tmp/mutation_hook_update_${PORT}_${OPERATION_ID}.log)"
fi

# Output message visible to both user and agent
echo "{\"continue\": true, \"systemMessage\": \"${MESSAGE}\", \"hookSpecificOutput\": {\"hookEventName\": \"PostToolUse\", \"additionalContext\": \"${MESSAGE}\"}}"
exit 0
