# BRP Method Rename Plan - Bevy 0.17 Migration

This plan covers renaming all 17 BRP methods from Bevy 0.16 to 0.17 naming conventions.

## Overview of Changes

| Category | Old Namespace | New Namespace | Key Changes |
|----------|---------------|---------------|-------------|
| Entity/Component | `bevy/*` | `world.*` | Pluralization, `destroy`→`despawn` |
| Resource | `bevy/*_resource` | `world.*_resources` | Pluralization |
| Registry | `registry/schema` | `registry.schema` | Separator change `/`→`.` |

---

## Entity/Component Methods

### 3. `bevy/destroy` → `world.despawn_entity`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyDestroy` → `WorldDespawnEntity`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.despawn_entity")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_destroy.rs` → `world_despawn_entity.rs`
- [ ] Update file header comment and struct docs in `world_despawn_entity.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_destroy.txt` → `world_despawn_entity.txt`
- [ ] Update description text in `world_despawn_entity.txt`

### 4. `bevy/reparent` → `world.reparent_entities`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyReparent` → `WorldReparentEntities`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.reparent_entities")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_reparent.rs` → `world_reparent_entities.rs`
- [ ] Update file header comment and struct docs in `world_reparent_entities.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_reparent.txt` → `world_reparent_entities.txt`
- [ ] Update description text in `world_reparent_entities.txt`

### 5. `bevy/get` → `world.get_components`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyGet` → `WorldGetComponents`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.get_components")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_get.rs` → `world_get_components.rs`
- [ ] Update file header comment and struct docs in `world_get_components.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_get.txt` → `world_get_components.txt`
- [ ] Update description text in `world_get_components.txt`
- [ ] `mcp/help_text/brp_execute.txt` - Update example references

### 6. `bevy/insert` → `world.insert_components`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyInsert` → `WorldInsertComponents`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.insert_components")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_insert.rs` → `world_insert_components.rs`
- [ ] Update file header comment and struct docs in `world_insert_components.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_insert.txt` → `world_insert_components.txt`
- [ ] Update description text in `world_insert_components.txt`
- [ ] `mcp/help_text/brp_type_guide.txt` - Update cross-references
- [ ] `mcp/help_text/brp_all_type_guides.txt` - Update cross-references

### 7. `bevy/remove` → `world.remove_components`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyRemove` → `WorldRemoveComponents`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.remove_components")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_remove.rs` → `world_remove_components.rs`
- [ ] Update file header comment and struct docs in `world_remove_components.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_remove.txt` → `world_remove_components.txt`
- [ ] Update description text in `world_remove_components.txt`

### 9. `bevy/mutate` → `world.mutate_components`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyMutateComponent` → `WorldMutateComponents`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.mutate_components")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_mutate_component.rs` → `world_mutate_components.rs`
- [ ] Update file header comment and struct docs in `world_mutate_components.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_mutate_component.txt` → `world_mutate_components.txt`
- [ ] Update description text in `world_mutate_components.txt`

---

## Watch Methods

### 10. `bevy/get+watch` → `world.get_components+watch`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyGetWatch` → `WorldGetComponentsWatch`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.get_components+watch")]`
- [ ] Rename `mcp/src/brp_tools/watch_tools/bevy_get_watch.rs` → `world_get_components_watch.rs`
- [ ] Update file header comment and struct docs in `world_get_components_watch.rs`
- [ ] Update `mcp/src/brp_tools/watch_tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_get_watch.txt` → `world_get_components_watch.txt`
- [ ] Update description text in `world_get_components_watch.txt`

---

## Resource Methods

### 13. `bevy/insert_resource` → `world.insert_resources`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyInsertResource` → `WorldInsertResources`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.insert_resources")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_insert_resource.rs` → `world_insert_resources.rs`
- [ ] Update file header comment and struct docs in `world_insert_resources.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_insert_resource.txt` → `world_insert_resources.txt`
- [ ] Update description text in `world_insert_resources.txt`
- [ ] `mcp/help_text/brp_type_guide.txt` - Update cross-references
- [ ] `mcp/help_text/brp_all_type_guides.txt` - Update cross-references

### 14. `bevy/remove_resource` → `world.remove_resources`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyRemoveResource` → `WorldRemoveResources`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.remove_resources")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_remove_resource.rs` → `world_remove_resources.rs`
- [ ] Update file header comment and struct docs in `world_remove_resources.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_remove_resource.txt` → `world_remove_resources.txt`
- [ ] Update description text in `world_remove_resources.txt`



### 16. `bevy/mutate_resource` → `world.mutate_resources`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyMutateResource` → `WorldMutateResources`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "world.mutate_resources")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_mutate_resource.rs` → `world_mutate_resources.rs`
- [ ] Update file header comment and struct docs in `world_mutate_resources.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_mutate_resource.txt` → `world_mutate_resources.txt`
- [ ] Update description text in `world_mutate_resources.txt`

---

## Registry Methods

### 17. `registry/schema` → `registry.schema`

- [ ] `mcp/src/tool/tool_name.rs` - Rename enum variant `BevyRegistrySchema` → `RegistrySchema`
- [ ] `mcp/src/tool/tool_name.rs` - Update `#[brp_tool(brp_method = "registry.schema")]`
- [ ] Rename `mcp/src/brp_tools/tools/bevy_registry_schema.rs` → `registry_schema.rs`
- [ ] Update file header comment and struct docs in `registry_schema.rs`
- [ ] Update `mcp/src/brp_tools/tools/mod.rs` imports
- [ ] Rename `mcp/help_text/bevy_registry_schema.txt` → `registry_schema.txt`
- [ ] Update description text in `registry_schema.txt`

---

## Naming Decisions

### ✅ File Naming - DECIDED
**Decision:** Rename all `.rs` and `.txt` files to match new method names


### ✅ Enum Variant Naming - DECIDED
**Decision:** Rename all enum variants to match new method names using `World*`/`Registry*` prefixes
- `BevySpawn` → `WorldSpawnEntity`
- `BevyDestroy` → `WorldDespawnEntity`
- `BevyGetResource` → `WorldGetResources`
- `BevyRegistrySchema` → `RegistrySchema`
- etc.

This matches Bevy's new naming convention and keeps the codebase consistent with the protocol changes.

---

## Testing Plan

After all renames are complete:

1. [ ] Run `cargo build` to verify compilation
2. [ ] Run `cargo +nightly fmt` to format code
3. [ ] Run `cargo nextest run` to verify all tests pass
4. [ ] Test each renamed method manually with `brp_launch_bevy_example extras_plugin`
5. [ ] Verify MCP tool names are correct (should be snake_case of enum variants)
6. [ ] Update integration tests in `.claude/commands/tests/` if needed
7. [ ] Update migration guide notes for this codebase

---

## Rollout Strategy

**Recommended approach:** Update in phases to minimize errors

### Phase 1: Core Entity/Component Methods (1-9)
Complete all entity/component method renames first since they're most commonly used.

### Phase 2: Watch Methods (10-11)
Update watch methods after core methods are working.

### Phase 3: Resource Methods (12-16)
Update resource methods.

### Phase 4: Registry Methods (17)
Update registry method last.

### Phase 5: Naming Consistency
Make final decisions on enum variants, file names, and help text file names if needed.
