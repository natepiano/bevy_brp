# BRP Method Argument Changes Report: Bevy 0.16.1 → 0.17.2

## Summary

Between Bevy 0.16.1 and 0.17.2, the BRP (Bevy Remote Protocol) underwent significant changes beyond just method renaming. There are **meaningful argument structure changes** that affect how methods are called.

---

## Method Files Locations

- **Bevy 0.16.1**: `/Users/natemccoy/rust/bevy-0.16.1/crates/bevy_remote/src/builtin_methods.rs`
- **Bevy 0.17.2**: `/Users/natemccoy/rust/bevy/crates/bevy_remote/src/builtin_methods.rs`

---

## Argument Structure Changes

### 1. **world.query** - Optional Parameters Added

**0.16.1 Signature:**
```rust
// Required parameters - must provide BrpQueryParams
pub fn process_remote_query_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpQueryParams { ... } = parse_some(params)?; // Requires params
```

**0.17.2 Signature:**
```rust
// Optional parameters - can call without params
pub fn process_remote_query_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpQueryParams { ... } = match params {
        Some(params) => parse_some(Some(params))?,
        None => BrpQueryParams {  // Default params if None
            data: BrpQuery {
                components: Vec::new(),
                option: ComponentSelector::default(),
                has: Vec::new(),
            },
            filter: BrpQueryFilter::default(),
            strict: false,
        },
    };
```

**Impact**: In 0.17.2, `world.query` can now be called **without parameters** to get all entities.

---

### 2. **BrpQuery.option** - Type Changed from Vec to Enum

**0.16.1 Structure:**
```rust
pub struct BrpQuery {
    #[serde(default)]
    pub components: Vec<String>,

    #[serde(default)]
    pub option: Vec<String>,  // Simple Vec<String>

    #[serde(default)]
    pub has: Vec<String>,
}
```

**0.17.2 Structure:**
```rust
pub struct BrpQuery {
    #[serde(default)]
    pub components: Vec<String>,

    #[serde(default)]
    pub option: ComponentSelector,  // NEW: Enum type instead of Vec<String>

    #[serde(default)]
    pub has: Vec<String>,
}

// NEW in 0.17.2
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentSelector {
    All,                           // NEW: Select all components
    #[serde(untagged)]
    Paths(Vec<String>),            // Original behavior
}
```

**Impact**: The `option` field now accepts either:
- `"all"` - to select all components
- An array of component paths (backward compatible with 0.16.1 format)

**Example JSON changes:**
```json
// 0.16.1 format (still works in 0.17.2)
{"data": {"option": ["bevy_transform::components::transform::Transform"]}}

// 0.17.2 new option
{"data": {"option": "all"}}
```

---

## Method Renamings (No Argument Changes)

While the following methods were renamed, their **argument structures remain identical**:

| 0.16.1 Method | 0.17.2 Method | Params Type |
|---------------|---------------|-------------|
| `bevy/get` | `world.get_components` | `BrpGetComponentsParams` |
| `bevy/get+watch` | `world.get_components+watch` | `BrpGetComponentsParams` |
| `bevy/get_resource` | `world.get_resources` | `BrpGetResourcesParams` |
| `bevy/query` | `world.query` | `BrpQueryParams` (with changes above) |
| `bevy/spawn` | `world.spawn_entity` | `BrpSpawnEntityParams` |
| `bevy/insert` | `world.insert_components` | `BrpInsertComponentsParams` |
| `bevy/insert_resource` | `world.insert_resources` | `BrpInsertResourcesParams` |
| `bevy/remove` | `world.remove_components` | `BrpRemoveComponentsParams` |
| `bevy/remove_resource` | `world.remove_resources` | `BrpRemoveResourcesParams` |
| `bevy/destroy` | `world.despawn_entity` | `BrpDespawnEntityParams` |
| `bevy/reparent` | `world.reparent_entities` | `BrpReparentEntitiesParams` |
| `bevy/list` | `world.list_components` | `BrpListComponentsParams` (optional) |
| `bevy/list+watch` | `world.list_components+watch` | `BrpListComponentsParams` |
| `bevy/mutate_component` | `world.mutate_components` | `BrpMutateComponentsParams` |
| `bevy/mutate_resource` | `world.mutate_resources` | `BrpMutateResourcesParams` |
| `bevy/list_resources` | `world.list_resources` | No params |
| `bevy/registry/schema` | `registry.schema` | `BrpJsonSchemaQueryFilter` (optional) |

---

## Internal Implementation Changes (Not User-Facing)

These changes affect internal Bevy code but not the BRP protocol itself:

1. **EventCursor → MessageCursor**: Internal type change in watch methods
   - 0.16.1: `Local<HashMap<ComponentId, EventCursor<RemovedComponentEntity>>>`
   - 0.17.2: `Local<HashMap<ComponentId, MessageCursor<RemovedComponentEntity>>>`

2. **insert_reflected_components signature**: Removed `type_registry` parameter
   - 0.16.1: `fn insert_reflected_components(type_registry: &TypeRegistry, entity_world_mut: EntityWorldMut, ...)`
   - 0.17.2: `fn insert_reflected_components(entity_world_mut: EntityWorldMut, ...)` (uses `insert_reflect` method)

3. **Component iteration method**: Changed from `components()` to `iter_components()`
   - 0.16.1: `entity.archetype().components()`
   - 0.17.2: `entity.archetype().iter_components()`

4. **Component ID lookup**: Changed from `get_id()` to `get_valid_id()`
   - 0.16.1: `world.components().get_id(type_id)`
   - 0.17.2: `world.components().get_valid_id(type_id)`

5. **registry.schema implementation**: Added `SchemaTypesMetadata` resource
   - 0.17.2 uses `world.resource::<crate::schemas::SchemaTypesMetadata>()` for additional type metadata

---

## Key Takeaways

**Two user-facing argument changes:**

1. **world.query** can now be called without parameters (defaults to querying all entities)
2. **BrpQuery.option** field changed from `Vec<String>` to `ComponentSelector` enum, allowing `"all"` to select all components

**All other methods** retained the same argument structure despite being renamed from `bevy/*` to `world.*` or `registry.*` namespaces.

---

## Implementation Plan for MCP Tool

### Current Status (Verified 2025-10-13)

- **bevy_brp_mcp** is already using Bevy 0.17.2
- **Both JSON formats already work** through the current implementation:
  - Array format: `{"option": ["path1", "path2"]}`
  - String format: `{"option": "all"}`
- Current implementation uses `pub data: Value` which passes through raw JSON to Bevy's BRP

### Recommended Changes

#### 1. Add Explicit Type Definitions in `mcp/src/brp_tools/tools/world_query.rs`

Replace the raw `Value` types with proper Rust structs that mirror Bevy's BRP types:

**Note**: `ComponentSelector` and `BrpQuery` changes are required to properly expose the new 0.17.2 functionality. `BrpQueryFilter` is an additional improvement for type safety on the `filter` field (not related to version changes).

```rust
/// Selector for optional components in a query (mirrors Bevy's ComponentSelector)
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComponentSelector {
    /// Select all components present on the entity
    All,
    /// Select specific components by their full type paths
    #[serde(untagged)]
    #[default]
    Paths(Vec<String>),
}

/// Query data specification - what component data to retrieve
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct BrpQuery {
    /// Required components - entities must have all of these
    #[serde(default)]
    pub components: Vec<String>,

    /// Optional components - retrieve if present. Can be "all" or array of paths
    #[serde(default)]
    pub option: ComponentSelector,

    /// Components to check for presence (returns boolean, not data)
    #[serde(default)]
    pub has: Vec<String>,
}

/// Query filter specification - which entities to include
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct BrpQueryFilter {
    /// Entities must have all of these components
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<String>,

    /// Entities must NOT have any of these components
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without: Vec<String>,
}
```

#### 2. Update `QueryParams` struct

Change from:
```rust
pub struct QueryParams {
    pub data: Value,
    pub filter: Option<Value>,
    // ...
}
```

To:
```rust
pub struct QueryParams {
    /// Object specifying what component data to retrieve
    pub data: BrpQuery,

    /// Object specifying which entities to query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<BrpQueryFilter>,
    // ...
}
```

### Benefits of This Approach

1. **Type Safety**: Compile-time validation of query structure
2. **Better IDE Support**: Autocomplete and type hints when using the MCP tool
3. **Clear Documentation**: The enum makes it explicit that `option` accepts either "all" or an array
4. **JSON Schema Generation**: The `JsonSchema` derive will generate proper schema showing both options
5. **Validation**: Invalid query structures will be caught during deserialization
6. **Maintainability**: Changes to Bevy's BRP types can be mirrored in our code

### Testing Strategy

After implementing the changes:

1. Test with array syntax: `{"data": {"components": ["Transform"], "option": ["Sprite"]}}`
2. Test with "all" syntax: `{"data": {"components": ["Transform"], "option": "all"}}`
3. Test with empty option (default): `{"data": {"components": ["Transform"]}}`
4. Verify JSON schema output includes proper documentation for `ComponentSelector` enum

### Version Compatibility

Since the changes are backward compatible (array syntax still works in 0.17.2), we can update the MCP tool to use explicit types without breaking existing usage. Users on Bevy 0.17.2+ will get the full benefits of both formats.

### Scope Limitations

**Other tools with `Value` fields are intentionally untyped:**

- `world.mutate_components` - `value: Value` (arbitrary component field data)
- `world.mutate_resources` - `value: Value` (arbitrary resource field data)
- `world.insert_resources` - `value: Value` (arbitrary resource data)

These remain as `Value` because they hold dynamic, type-dependent data that cannot be statically typed. The `world.query` case is unique because its structure (`data` and `filter` fields) is fixed and defined by Bevy's BRP specification, making it suitable for typed structs.
