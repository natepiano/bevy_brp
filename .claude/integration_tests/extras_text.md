# BRP Extras Text Input Tests

## Objective
Validate brp_extras text input method: type_text with sequential typing, special characters, and unmappable character handling.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Text Input Tests
- Clear the `TextInputContent` resource: `mcp__brp__world_insert_resources` with resource `extras_plugin::TextInputContent` and value `{"text": ""}`
- Test basic typing: `mcp__brp__brp_extras_type_text` with `{"text": "hello"}`
- Verify text appears: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello"}`
- Test sequential typing: `mcp__brp__brp_extras_type_text` with `{"text": " world"}`
- Verify concatenation: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello world"}`
- Test special characters: `mcp__brp__brp_extras_type_text` with `{"text": "!@#"}`
- Verify special chars: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello world!@#"}`
- Test unmappable characters: `mcp__brp__brp_extras_type_text` with text containing unmappable chars
- Verify skipped array is populated in response

## Expected Results
- Text typing works sequentially and accumulates correctly
- Special characters are typed properly
- Unmappable characters are reported as skipped

## Failure Criteria
STOP if: Text typing doesn't work, doesn't accumulate properly, or unmappable characters aren't reported correctly.
