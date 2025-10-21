# Large Response Handling Tests

## Objective
Validate handling of large responses that exceed context limits, particularly registry schema operations.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing large response handling, not app management.

**IMPORTANT**: Do NOT attempt to read or access the generated files. Only validate the response metadata.

## Response Interpretation Guide

**SUCCESS for Large Request (Step 1)**: Response contains `result.saved_to_file: true` and `result.filepath` field - this means BRP server successfully wrote result to file while preserving all other response fields

**SUCCESS for Filtered Request (Step 3)**: Response contains inline `result` with schema objects - this means response was small enough to return directly

**NOT A FAILURE**: MCP tool blocking responses due to token limits - this is expected protective behavior

**ACTUAL FAILURE**: Missing `result.filepath` field when expected, or BRP server error status

## Test Steps

### 1. Large Registry Schema Request
- Execute `mcp__brp__registry_schema` without filters (intentionally large)
- Verify response indicates file output due to size
- Check that `result.filepath` is returned instead of inline schema data
- Verify response has success status, message, call_info, and `result.saved_to_file: true`

### 2. Response Metadata Validation
- Verify response includes helpful metadata about file output
- Check that `result.original_size_tokens` indicates token count that triggered file output
- Confirm `result.instructions` includes guidance for accessing the file
- Validate response status indicates success and all other fields (message, call_info) are preserved

### 3. Filtered Schema Request (Should Return Inline)
- Execute registry schema with restrictive filters
- Use `with_crates: ["bevy_transform"]` to limit size
- Verify smaller response has inline `result` field with schema data (not file reference)
- Check response format is correct with all expected fields

### 4. Response Size Management
- Compare file output vs inline response approaches
- Verify appropriate threshold handling
- Check that file output prevents context overflow while preserving response structure
- Confirm response structure and other fields (message, call_info) are preserved in both cases

## Expected Results
- ✅ Large result fields are automatically written to files
- ✅ File paths are returned in `result.filepath` field
- ✅ Response preserves all original fields (status, message, call_info)
- ✅ Result field includes `saved_to_file: true`, `original_size_tokens`, and `instructions`
- ✅ Smaller responses are returned inline appropriately
- ✅ File output prevents context limit issues while maintaining response structure
- ✅ Response handling is transparent and reliable

## Failure Criteria
STOP if: File output fails, `result.filepath` is not returned, response fields are missing, or response size management doesn't work properly.
