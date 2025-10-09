# BRP Method Rename Plan - Bevy 0.17 Migration
---

## Entity/Component Methods

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
