# Large Response Handling Tests

## Objective
Validate handling of large responses that exceed context limits, particularly registry schema operations.

## Test Steps

### 1. Large Registry Schema Request
- Execute `mcp__brp__bevy_registry_schema` without filters (intentionally large)
- Verify response indicates file output due to size
- Check that file path is returned instead of inline data
- Confirm file is created at specified path

### 2. File Content Validation
- Read the generated schema file
- Verify file contains valid JSON schema data
- Check for presence of expected type information
- Confirm file structure includes shortPath, typePath, properties

### 3. Filtered Schema Request (Should Return Inline)
- Execute registry schema with restrictive filters
- Use `with_crates: ["bevy_transform"]` to limit size
- Verify smaller response is returned inline (not as file)
- Check response format is correct

### 4. File Cleanup Validation
- Verify file paths are accessible for reading
- Check file permissions allow access
- Confirm files can be opened by external tools
- Test file cleanup if needed

### 5. Response Size Management
- Compare file output vs inline response approaches
- Verify appropriate threshold handling
- Check that file output prevents context overflow
- Confirm response metadata is helpful

## Expected Results
- ✅ Large responses are automatically written to files
- ✅ File paths are returned in responses
- ✅ Generated files contain valid schema data
- ✅ Smaller responses are returned inline appropriately
- ✅ File output prevents context limit issues
- ✅ Response handling is transparent and reliable

## Failure Criteria
STOP if: File output fails, generated files are inaccessible, or response size management doesn't work properly.