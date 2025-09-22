# Bug Report: bevy/reparent Message Template Variable Not Substituted

## Date Reported
2025-01-22

## Summary
The `bevy/reparent` BRP tool returns a message with an unsubstituted template variable `{{entity_count}}` instead of the actual count of reparented entities.

## Reproduction Steps
1. Launch a Bevy app with BRP enabled (e.g., `extras_plugin` example)
2. Spawn two entities with Transform components
3. Call `bevy_reparent` to parent one entity to another
4. Observe the returned message

## Expected Behavior
The message should display: "Reparented 1 entities" (or the appropriate count)

## Actual Behavior
The message displays: "Reparented {{entity_count}} entities"

## Root Cause Analysis

### Location
- **File**: `mcp/src/brp_tools/tools/bevy_reparent.rs:36`
- **Struct**: `ReparentResult`

### Issue
The `ReparentResult` struct has a message template field with the attribute:
```rust
#[to_message(message_template = "Reparented {entity_count} entities")]
pub message_template: String,
```

However, the struct lacks an `entity_count` field to provide the actual value for substitution. The template substitution system in `ResponseBuilder::substitute_dynamic_template()` looks for values in:
1. Error info
2. Metadata
3. Result
4. Parameters
5. Request parameters

Since `entity_count` is not present in any of these locations, the placeholder remains unsubstituted.

### Comparison with Working Implementation
The `bevy/query` tool correctly implements this pattern:
```rust
/// Count of entities returned
#[to_metadata(result_operation = "count")]
pub entity_count: usize,

/// Message template for formatting responses
#[to_message(message_template = "Found {entity_count} entities")]
pub message_template: String,
```

## Fix Recommendation

Add an `entity_count` field to `ReparentResult` that computes the count from the parameters:

```rust
/// Result for the `bevy/reparent` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct ReparentResult {
    /// The raw BRP response data (empty for reparent)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of entities that were reparented
    #[to_metadata(params_operation = "count_entities")]
    pub entity_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Reparented {entity_count} entities")]
    pub message_template: String,
}
```

This would require implementing a new `params_operation` in the macro that can access the `entities` field from `ReparentParams` and return its length.

## Alternative Fix

A simpler alternative would be to compute the count in metadata during tool execution by accessing the params directly, but this would require changes to how the BRP tools handle parameter metadata propagation.

## Impact
- **Severity**: Low - Cosmetic issue in response messages
- **Affected Tools**: `bevy_reparent`
- **User Impact**: Confusing message output but functionality works correctly

## Test Case
After fix implementation, the test should verify:
1. Reparenting single entity shows "Reparented 1 entities"
2. Reparenting multiple entities shows correct count
3. Reparenting zero entities (empty array) shows "Reparented 0 entities"