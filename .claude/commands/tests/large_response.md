# Large Response Handling Tests

## Objective
Validate handling of large responses that exceed context limits, particularly registry schema operations.

## Test Steps

### 1. Large Registry Schema Request
- Execute `mcp__brp__bevy_registry_schema` without filters (intentionally large)
- Verify response indicates file output due to size
- Check that file path is returned instead of inline data
- Confirm file is created at specified path

### 2. Response Metadata Validation
- Verify response includes helpful metadata about file output
- Check that response indicates token count that triggered file output
- Confirm response includes instructions for accessing the file
- Validate response status indicates success

### 3. Filtered Schema Request (Should Return Inline)
- Execute registry schema with restrictive filters
- Use `with_crates: ["bevy_transform"]` to limit size
- Verify smaller response is returned inline (not as file)
- Check response format is correct

### 4. Response Size Management
- Compare file output vs inline response approaches
- Verify appropriate threshold handling
- Check that file output prevents context overflow
- Confirm response metadata is helpful

## Expected Results
- ✅ Large responses are automatically written to files
- ✅ File paths are returned in responses
- ✅ Response metadata includes token count and helpful instructions
- ✅ Smaller responses are returned inline appropriately
- ✅ File output prevents context limit issues
- ✅ Response handling is transparent and reliable

## Failure Criteria
STOP if: File output fails, file paths are not returned, or response size management doesn't work properly.