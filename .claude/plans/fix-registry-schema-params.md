# Plan: Fix `registry.schema` Parameter Structure Mismatch

## Problem

The MCP `registry_schema` tool uses a flat parameter structure, but BRP expects a nested structure for type filtering.

**MCP sends (incorrect):**
```json
{
  "with_types": ["Component", "Resource"],
  "without_types": ["RenderResource"]
}
```

**BRP expects (correct):**
```json
{
  "type_limit": {
    "with": ["Component", "Resource"],
    "without": ["RenderResource"]
  }
}
```

## Impact

Type filtering via `with_types`/`without_types` parameters silently fails - BRP ignores unrecognized fields.

## Solution

Change `RegistrySchemaParams` to match BRP's nested structure. This is a breaking API change - document in CHANGELOG.md.

## Implementation

### Step 1: Update RegistrySchemaParams

**File:** `mcp/src/brp_tools/tools/registry_schema.rs`

Replace the current struct with:

```rust
//! `registry.schema` tool - Get type schemas

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Type filtering constraints for registry.schema
#[derive(Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct TypeLimit {
    /// Include only types with these reflect traits (e.g., ["Component", "Resource"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<String>,

    /// Exclude types with these reflect traits (e.g., ["RenderResource"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without: Vec<String>,
}

/// Parameters for the `registry.schema` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct RegistrySchemaParams {
    /// Include only types from these crates (e.g., ["bevy_transform", "my_game"])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_crates: Option<Vec<String>>,

    /// Exclude types from these crates (e.g., ["bevy_render", "bevy_pbr"])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub without_crates: Option<Vec<String>>,

    /// Type filtering by reflect traits
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_limit: Option<TypeLimit>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `registry.schema` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct RegistrySchemaResult {
    /// The raw BRP response - map of type schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of types returned
    #[to_metadata(result_operation = "count")]
    pub type_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Retrieved {type_count} schemas")]
    pub message_template: String,
}
```

---

### Step 2: Update Help Text

**File:** `mcp/help_text/registry_schema.md`

Update parameter documentation to reflect new structure:

```markdown
## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `with_crates` | string[] | No | Include only types from these crates |
| `without_crates` | string[] | No | Exclude types from these crates |
| `type_limit` | object | No | Filter by reflect traits |
| `type_limit.with` | string[] | No | Include only types with these traits |
| `type_limit.without` | string[] | No | Exclude types with these traits |
| `port` | number | No | BRP port (default: 15702) |

## Examples

### Filter by crate
```json
{
  "with_crates": ["bevy_transform", "my_game"]
}
```

### Filter by type (Components only)
```json
{
  "type_limit": {
    "with": ["Component"]
  }
}
```

### Combined filtering
```json
{
  "with_crates": ["bevy_ecs"],
  "type_limit": {
    "with": ["Resource"],
    "without": ["Default"]
  }
}
```
```

---

### Step 3: Document Breaking Change

**File:** `CHANGELOG.md`

Add entry:

```markdown
### Breaking Changes

- **`registry_schema` parameter structure changed**: The `with_types` and `without_types` parameters have been replaced with a nested `type_limit` object to match BRP's expected format.

  Before:
  ```json
  { "with_types": ["Component"], "without_types": ["Resource"] }
  ```

  After:
  ```json
  { "type_limit": { "with": ["Component"], "without": ["Resource"] } }
  ```
```

---

## File Summary

| File | Change |
|------|--------|
| `mcp/src/brp_tools/tools/registry_schema.rs` | Replace flat params with nested `TypeLimit` struct |
| `mcp/help_text/registry_schema.md` | Update parameter documentation |
| `CHANGELOG.md` | Document breaking change |

---

## Testing

1. Build: `cargo build -p bevy_brp_mcp`
2. Launch a Bevy app with BRP enabled
3. Test type filtering:
   ```json
   {
     "type_limit": {
       "with": ["Component"]
     },
     "with_crates": ["bevy_transform"]
   }
   ```
4. Verify response only contains Component types from bevy_transform
