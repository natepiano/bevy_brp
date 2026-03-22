# BRP Extras Text Input Tests

## Objective
Validate brp_extras text input method: type_text with sequential typing, special characters, and unmappable character handling.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Clear Text Resource
- Execute `mcp__brp__world_insert_resources` with resource `extras_plugin::TextInputContent` and value `{"text": ""}`
- Verify the resource is cleared

### 3. Basic Typing
- Execute `mcp__brp__brp_extras_type_text` with `{"text": "hello"}`
- Execute `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`
- Verify the resource returns `{"text": "hello"}`

### 4. Sequential Typing
- Execute `mcp__brp__brp_extras_type_text` with `{"text": " world"}`
- Execute `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`
- Verify the resource returns `{"text": "hello world"}`

### 5. Special Characters
- Execute `mcp__brp__brp_extras_type_text` with `{"text": "!@#"}`
- Execute `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`
- Verify the resource returns `{"text": "hello world!@#"}`

### 6. Unmappable Characters
- Execute `mcp__brp__brp_extras_type_text` with text containing unmappable chars (e.g. `"café"`)
- Verify the skipped array is populated in the response

## Expected Results
- Text typing works sequentially and accumulates correctly
- Special characters are typed properly
- Unmappable characters are reported as skipped

## Failure Criteria
STOP if: Text typing doesn't work, doesn't accumulate properly, or unmappable characters aren't reported correctly.
