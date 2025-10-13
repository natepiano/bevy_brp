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
