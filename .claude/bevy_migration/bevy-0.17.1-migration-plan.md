# Bevy 0.17.1 Migration Plan

**Generated:** 2025-10-03
**Codebase:** /Users/natemccoy/rust/bevy_brp
**Total Applicable Guides:** 16

---

## Summary

- **REQUIRED changes:** 4 guides (79 total occurrences)
- **HIGH priority:** 4 guides (56 total occurrences)
- **MEDIUM priority:** 0 guides (0 total occurrences)
- **LOW priority:** 8 guides (11 total occurrences)

**Count Anomalies:** 6 guides with >20% variance between Pass 1 and Pass 2
- renamed_BRP_methods.md: Pass 1=101, Pass 2=73 (-28%)
- reflect_registration_changes.md: Pass 1=145, Pass 2=100 (-31%)
- event_split.md: Pass 1=51, Pass 2=51 (0%)
- bevy_render_reorganization.md: Pass 1=31, Pass 2=3 (-90%)
- observer_and_event_changes.md: Pass 1=8, Pass 2=1 (-88%)
- chromatic_aberration_option.md: Pass 1=4, Pass 2=2 (-50%)

**Estimated effort:**
- REQUIRED: Large (must fix to compile) - 79 occurrences across core BRP method mappings, sprite anchors, observers, and module reorganizations
- HIGH: Medium (should fix soon) - 56 occurrences across reflection registration, event/message split, screenshot event handling, and import updates
- MEDIUM: None
- LOW: Small (nice to have) - 11 informational occurrences with no code changes needed

---

## ⚠️ Dependency Compatibility Review

**Status:** No bevy-related dependencies found in this project

---

## REQUIRED Changes

## Renamed BRP Methods

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/renamed_BRP_methods.md`
**Requirement Level:** REQUIRED
**Occurrences:** 73 locations across 20 files
**Pass 1 Count:** 101 | **Pass 2 Count:** 73 | **Status:** ANOMALY: -28%

### Migration Guide Summary

Most Bevy Remote Protocol methods have been renamed to be more explicit and organized under a `world.*` namespace prefix. The word `destroy` has been replaced with `despawn` to match Bevy engine terminology. Methods like `bevy/query` become `world.query`, `bevy/spawn` becomes `world.spawn_entity`, and resource operations are pluralized (e.g., `bevy/get_resource` becomes `world.get_resources`).

### Required Changes

**1. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/list**
```diff
-    #[brp_tool(brp_method = "bevy/list", params = "ListParams", result = "ListResult")]
+    #[brp_tool(brp_method = "world.list_components", params = "ListParams", result = "ListResult")]
```

**2. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/get**
```diff
-    #[brp_tool(brp_method = "bevy/get", params = "GetParams", result = "GetResult")]
+    #[brp_tool(brp_method = "world.get_components", params = "GetParams", result = "GetResult")]
```

**3. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/destroy**
```diff
-        brp_method = "bevy/destroy",
+        brp_method = "world.despawn_entity",
```

**4. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/insert**
```diff
-        brp_method = "bevy/insert",
+        brp_method = "world.insert_components",
```

**5. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/remove**
```diff
-        brp_method = "bevy/remove",
+        brp_method = "world.remove_components",
```

**6. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/list_resources**
```diff
-        brp_method = "bevy/list_resources",
+        brp_method = "world.list_resources",
```

**7. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/get_resource**
```diff
-        brp_method = "bevy/get_resource",
+        brp_method = "world.get_resources",
```

**8. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/insert_resource**
```diff
-        brp_method = "bevy/insert_resource",
+        brp_method = "world.insert_resources",
```

**9. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/remove_resource**
```diff
-        brp_method = "bevy/remove_resource",
+        brp_method = "world.remove_resources",
```

**10. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/mutate_resource**
```diff
-        brp_method = "bevy/mutate_resource",
+        brp_method = "world.mutate_resources",
```

**11. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/mutate_component**
```diff
-        brp_method = "bevy/mutate_component",
+        brp_method = "world.mutate_components",
```

**12. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/query**
```diff
-        brp_method = "bevy/query",
+        brp_method = "world.query",
```

**13. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/spawn**
```diff
-        brp_method = "bevy/spawn",
+        brp_method = "world.spawn_entity",
```

**14. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/registry/schema**
```diff
-        brp_method = "bevy/registry/schema",
+        brp_method = "registry.schema",
```

**15. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/reparent**
```diff
-        brp_method = "bevy/reparent",
+        brp_method = "world.reparent_entities",
```

**16. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/get+watch**
```diff
-    #[brp_tool(brp_method = "bevy/get+watch")]
+    #[brp_tool(brp_method = "world.get_components+watch")]
```

**17. Update brp_method in `mcp/src/tool/tool_name.rs` - bevy/list+watch**
```diff
-    #[brp_tool(brp_method = "bevy/list+watch")]
+    #[brp_tool(brp_method = "world.list_components+watch")]
```

**18. Update comment in `mcp/src/tool/tool_name.rs`**
```diff
-        /// The BRP method name (e.g., "bevy/spawn")
+        /// The BRP method name (e.g., "world.spawn_entity")
```

**19. Update comment in `mcp/src/brp_tools/brp_type_guide/all_types_tool.rs`**
```diff
-//! their type schema information in a single call. It combines `bevy/list` and
+//! their type schema information in a single call. It combines `world.list_components` and
```

**20. Update comment in `mcp/src/brp_tools/brp_type_guide/all_types_tool.rs`**
```diff
-    // First, get all registered types using bevy/list without entity parameter
+    // First, get all registered types using world.list_components without entity parameter
```

**21. Update error message in `mcp/src/brp_tools/brp_type_guide/all_types_tool.rs`**
```diff
-                    "bevy/list did not return an array of types".to_string(),
+                    "world.list_components did not return an array of types".to_string(),
```

**22. Update error message in `mcp/src/brp_tools/brp_type_guide/all_types_tool.rs`**
```diff
-            return Err(Error::BrpCommunication("bevy/list returned no data".to_string()).into());
+            return Err(Error::BrpCommunication("world.list_components returned no data".to_string()).into());
```

**23. Update error message in `mcp/src/brp_tools/brp_type_guide/all_types_tool.rs`**
```diff
-                "bevy/list failed: {}",
+                "world.list_components failed: {}",
```

**24. Update comment in `mcp_macros/src/result_struct.rs`**
```diff
-                // For bevy/get - handles both strict=true (flat) and strict=false (nested) formats
+                // For world.get_components - handles both strict=true (flat) and strict=false (nested) formats
```

**25. Update comment in `mcp_macros/src/result_struct.rs`**
```diff
-                // For bevy/get result structure
+                // For world.get_components result structure
```

**26. Update example in `mcp_macros/src/lib.rs`**
```diff
-///     #[brp_tool(brp_method = "bevy/destroy", params = "DestroyParams")]
+///     #[brp_tool(brp_method = "world.despawn_entity", params = "DestroyParams")]
```

**27. Update example in `mcp_macros/src/lib.rs`**
```diff
-///     #[brp_tool(brp_method = "bevy/get+watch")]
+///     #[brp_tool(brp_method = "world.get_components+watch")]
```

**28. Update module doc comment in `mcp/src/brp_tools/tools/bevy_remove_resource.rs`**
```diff
-//! `bevy/remove_resource` tool - Remove resources
+//! `world.remove_resources` tool - Remove resources
```

**29. Update doc comment in `mcp/src/brp_tools/tools/bevy_remove_resource.rs`**
```diff
-/// Parameters for the `bevy/remove_resource` tool
+/// Parameters for the `world.remove_resources` tool
```

**30. Update doc comment in `mcp/src/brp_tools/tools/bevy_remove_resource.rs`**
```diff
-/// Result for the `bevy/remove_resource` tool
+/// Result for the `world.remove_resources` tool
```

**31. Update module doc comment in `mcp/src/brp_tools/tools/bevy_get_resource.rs`**
```diff
-//! `bevy/get_resource` tool - Get resource data
+//! `world.get_resources` tool - Get resource data
```

**32. Update doc comment in `mcp/src/brp_tools/tools/bevy_get_resource.rs`**
```diff
-/// Parameters for the `bevy/get_resource` tool
+/// Parameters for the `world.get_resources` tool
```

**33. Update doc comment in `mcp/src/brp_tools/tools/bevy_get_resource.rs`**
```diff
-/// Result for the `bevy/get_resource` tool
+/// Result for the `world.get_resources` tool
```

**34. Update module doc comment in `mcp/src/brp_tools/tools/bevy_remove.rs`**
```diff
-//! `bevy/remove` tool - Remove components from entities
+//! `world.remove_components` tool - Remove components from entities
```

**35. Update doc comment in `mcp/src/brp_tools/tools/bevy_remove.rs`**
```diff
-/// Parameters for the `bevy/remove` tool
+/// Parameters for the `world.remove_components` tool
```

**36. Update doc comment in `mcp/src/brp_tools/tools/bevy_remove.rs`**
```diff
-/// Result for the `bevy/remove` tool
+/// Result for the `world.remove_components` tool
```

**37. Update module doc comment in `mcp/src/brp_tools/tools/bevy_mutate_component.rs`**
```diff
-//! `bevy/mutate_component` tool - Mutate component fields
+//! `world.mutate_components` tool - Mutate component fields
```

**38. Update doc comment in `mcp/src/brp_tools/tools/bevy_mutate_component.rs`**
```diff
-/// Parameters for the `bevy/mutate_component` tool
+/// Parameters for the `world.mutate_components` tool
```

**39. Update doc comment in `mcp/src/brp_tools/tools/bevy_mutate_component.rs`**
```diff
-/// Result for the `bevy/mutate_component` tool
+/// Result for the `world.mutate_components` tool
```

**40. Update module doc comment in `mcp/src/brp_tools/tools/bevy_insert_resource.rs`**
```diff
-//! `bevy/insert_resource` tool - Insert or update resources
+//! `world.insert_resources` tool - Insert or update resources
```

**41. Update doc comment in `mcp/src/brp_tools/tools/bevy_insert_resource.rs`**
```diff
-/// Parameters for the `bevy/insert_resource` tool
+/// Parameters for the `world.insert_resources` tool
```

**42. Update doc comment in `mcp/src/brp_tools/tools/bevy_insert_resource.rs`**
```diff
-/// Result for the `bevy/insert_resource` tool
+/// Result for the `world.insert_resources` tool
```

**43. Update module doc comment in `mcp/src/brp_tools/tools/bevy_list.rs`**
```diff
-//! `bevy/list` tool - List components on an entity or all component types
+//! `world.list_components` tool - List components on an entity or all component types
```

**44. Update doc comment in `mcp/src/brp_tools/tools/bevy_list.rs`**
```diff
-/// Parameters for the `bevy/list` tool
+/// Parameters for the `world.list_components` tool
```

**45. Update doc comment in `mcp/src/brp_tools/tools/bevy_list.rs`**
```diff
-/// Result for the `bevy/list` tool
+/// Result for the `world.list_components` tool
```

**46. Update module doc comment in `mcp/src/brp_tools/tools/bevy_reparent.rs`**
```diff
-//! `bevy/reparent` tool - Change entity parents
+//! `world.reparent_entities` tool - Change entity parents
```

**47. Update doc comment in `mcp/src/brp_tools/tools/bevy_reparent.rs`**
```diff
-/// Parameters for the `bevy/reparent` tool
+/// Parameters for the `world.reparent_entities` tool
```

**48. Update doc comment in `mcp/src/brp_tools/tools/bevy_reparent.rs`**
```diff
-/// Result for the `bevy/reparent` tool
+/// Result for the `world.reparent_entities` tool
```

**49. Update module doc comment in `mcp/src/brp_tools/tools/bevy_get.rs`**
```diff
-//! `bevy/get` tool - Get component data from entities
+//! `world.get_components` tool - Get component data from entities
```

**50. Update doc comment in `mcp/src/brp_tools/tools/bevy_get.rs`**
```diff
-/// Parameters for the `bevy/get` tool
+/// Parameters for the `world.get_components` tool
```

**51. Update doc comment in `mcp/src/brp_tools/tools/bevy_get.rs`**
```diff
-/// Result for the `bevy/get` tool
+/// Result for the `world.get_components` tool
```

**52. Update doc comment in `mcp/src/brp_tools/tools/brp_execute.rs`**
```diff
-    /// The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')
+    /// The BRP method to execute (e.g., 'rpc.discover', 'world.get_components', 'world.query')
```

**53. Update module doc comment in `mcp/src/brp_tools/tools/bevy_query.rs`**
```diff
-//! `bevy/query` tool - Query entities by components
+//! `world.query` tool - Query entities by components
```

**54. Update doc comment in `mcp/src/brp_tools/tools/bevy_query.rs`**
```diff
-/// Parameters for the `bevy/query` tool
+/// Parameters for the `world.query` tool
```

**55. Update doc comment in `mcp/src/brp_tools/tools/bevy_query.rs`**
```diff
-/// Result for the `bevy/query` tool
+/// Result for the `world.query` tool
```

**56. Update module doc comment in `mcp/src/brp_tools/tools/bevy_registry_schema.rs`**
```diff
-//! `bevy/registry/schema` tool - Get type schemas
+//! `registry.schema` tool - Get type schemas
```

**57. Update doc comment in `mcp/src/brp_tools/tools/bevy_registry_schema.rs`**
```diff
-/// Parameters for the `bevy/registry/schema` tool
+/// Parameters for the `registry.schema` tool
```

**58. Update doc comment in `mcp/src/brp_tools/tools/bevy_registry_schema.rs`**
```diff
-/// Result for the `bevy/registry/schema` tool
+/// Result for the `registry.schema` tool
```

**59. Update module doc comment in `mcp/src/brp_tools/tools/bevy_destroy.rs`**
```diff
-//! `bevy/destroy` tool - Destroy entities permanently
+//! `world.despawn_entity` tool - Despawn entities permanently
```

**60. Update doc comment in `mcp/src/brp_tools/tools/bevy_destroy.rs`**
```diff
-/// Parameters for the `bevy/destroy` tool
+/// Parameters for the `world.despawn_entity` tool
```

**61. Update doc comment in `mcp/src/brp_tools/tools/bevy_destroy.rs`**
```diff
-/// Result for the `bevy/destroy` tool
+/// Result for the `world.despawn_entity` tool
```

**62. Update module doc comment in `mcp/src/brp_tools/tools/bevy_spawn.rs`**
```diff
-//! `bevy/spawn` tool - Spawn entities with components
+//! `world.spawn_entity` tool - Spawn entities with components
```

**63. Update doc comment in `mcp/src/brp_tools/tools/bevy_spawn.rs`**
```diff
-/// Parameters for the `bevy/spawn` tool
+/// Parameters for the `world.spawn_entity` tool
```

**64. Update doc comment in `mcp/src/brp_tools/tools/bevy_spawn.rs`**
```diff
-/// Result for the `bevy/spawn` tool
+/// Result for the `world.spawn_entity` tool
```

**65. Update module doc comment in `mcp/src/brp_tools/tools/bevy_list_resources.rs`**
```diff
-//! `bevy/list_resources` tool - List all registered resources
+//! `world.list_resources` tool - List all registered resources
```

**66. Update doc comment in `mcp/src/brp_tools/tools/bevy_list_resources.rs`**
```diff
-/// Parameters for the `bevy/list_resources` tool
+/// Parameters for the `world.list_resources` tool
```

**67. Update doc comment in `mcp/src/brp_tools/tools/bevy_list_resources.rs`**
```diff
-/// Result for the `bevy/list_resources` tool
+/// Result for the `world.list_resources` tool
```

**68. Update module doc comment in `mcp/src/brp_tools/tools/bevy_insert.rs`**
```diff
-//! `bevy/insert` tool - Insert or replace components on entities
+//! `world.insert_components` tool - Insert or replace components on entities
```

**69. Update doc comment in `mcp/src/brp_tools/tools/bevy_insert.rs`**
```diff
-/// Parameters for the `bevy/insert` tool
+/// Parameters for the `world.insert_components` tool
```

**70. Update doc comment in `mcp/src/brp_tools/tools/bevy_insert.rs`**
```diff
-/// Result for the `bevy/insert` tool
+/// Result for the `world.insert_components` tool
```

**71. Update module doc comment in `mcp/src/brp_tools/tools/bevy_mutate_resource.rs`**
```diff
-//! `bevy/mutate_resource` tool - Mutate resource fields
+//! `world.mutate_resources` tool - Mutate resource fields
```

**72. Update doc comment in `mcp/src/brp_tools/tools/bevy_mutate_resource.rs`**
```diff
-/// Parameters for the `bevy/mutate_resource` tool
+/// Parameters for the `world.mutate_resources` tool
```

**73. Update doc comment in `mcp/src/brp_tools/tools/bevy_mutate_resource.rs`**
```diff
-/// Result for the `bevy/mutate_resource` tool
+/// Result for the `world.mutate_resources` tool
```

### Search Pattern

To find all occurrences:
```bash
rg "bevy/query|bevy/spawn|bevy/destroy|bevy/get|bevy/insert|bevy/remove|bevy/list|bevy/reparent|bevy/mutate|bevy/get_resource|bevy/insert_resource|bevy/list_resources|bevy/remove_resource|bevy/mutate_resource|bevy/mutate_component|bevy/registry/schema|bevy/get\+watch|bevy/list\+watch" --type rust
```

---

## Anchor is removed from Sprite

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/anchor_is_removed_from_sprite.md`
**Requirement Level:** REQUIRED
**Occurrences:** 2 locations across 1 file
**Pass 1 Count:** 15 | **Pass 2 Count:** 13 | **Status:** MATCH (±13%)

### Migration Guide Summary

The `anchor` field has been removed from `Sprite` component. `Anchor` is now a required component on `Sprite` instead of a field. Additionally, anchor variant naming has changed from PascalCase (e.g., `BottomLeft`) to SCREAMING_SNAKE_CASE constants (e.g., `BOTTOM_LEFT`), with `Anchor::Custom(value)` replaced by `Anchor(value)`.

### Required Changes

**1. Remove `anchor` field from `Sprite` initialization and add `Anchor` as separate component in `test-app/examples/extras_plugin.rs`**
```diff
  commands.spawn((
      Sprite {
          color: Color::srgb(1.0, 0.5, 0.25),
          custom_size: Some(Vec2::new(64.0, 64.0)),
          flip_x: false,
          flip_y: false,
-         anchor: bevy::sprite::Anchor::Center,
          ..default()
      },
+     bevy::sprite::Anchor::Center,
      Transform::from_xyz(100.0, 100.0, 0.0),
      Name::new("TestSprite"),
      bevy::render::view::visibility::RenderLayers::layer(1),
  ));
```

**2. No changes needed for standalone `Anchor` component in `test-app/examples/extras_plugin.rs`**

The existing code at line 689:
```rust
commands.spawn((bevy::sprite::Anchor::Center, Name::new("AnchorTestEntity")));
```

This is already correct for Bevy 0.17 - `Anchor` is spawned as a component, not as a field of `Sprite`.

### Search Pattern

To find all occurrences:
```bash
rg "anchor:" --type rust
rg "bevy::sprite::Anchor" --type rust
```

---

## Observer / Event API Changes

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/observer_and_event_changes.md`
**Requirement Level:** REQUIRED
**Occurrences:** 1 location across 1 file
**Pass 1 Count:** 8 | **Pass 2 Count:** 1 | **Status:** ANOMALY: -88% (Note: Pass 1 likely counted token-level matches across all patterns; Pass 2 found actual code requiring migration)

### Migration Guide Summary

The observer trigger API has been redesigned for better clarity and type-safety. The `Trigger` type is renamed to `On`, lifecycle events drop the "On" prefix (`OnAdd` → `Add`), and entity targeting has changed from `trigger.target()` to direct field access on events. The `.observe()` method is not affected by these changes.

### Required Changes

**1. Update observer parameter type from `Trigger<ScreenshotCaptured>` to `On<ScreenshotCaptured>` in `extras/src/screenshot.rs`**
```diff
-        .observe(move |trigger: Trigger<ScreenshotCaptured>| {
+        .observe(move |screenshot_captured: On<ScreenshotCaptured>| {
             info!("Screenshot captured! Starting async save to: {path_for_observer}");
-            let img = trigger.event().0.clone();
+            let img = screenshot_captured.event().0.clone();
             let path_clone = path_for_observer.clone();
```

### Search Patterns

To find all occurrences of `Trigger` in observers:
```bash
rg "Trigger<" --type rust
```

To find all `.observe()` calls that may need updating:
```bash
rg "\.observe\(" --type rust -A 3
```

---

## Text2d moved to bevy_sprite

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/text2d_moved_to_bevy_sprite.md`
**Requirement Level:** REQUIRED
**Occurrences:** 3 locations across 1 file
**Pass 1 Count:** 3 | **Pass 2 Count:** 3 | **Status:** MATCH

### Migration Guide Summary

The world-space text types `Text2d` and `Text2dShadow` have been moved from `bevy_text` to the `bevy_sprite` crate. These types should now be imported from `bevy::sprite` instead of `bevy::text`.

### Required Changes

**1. Update Text2d import in `test-app/examples/extras_plugin.rs`**
```diff
- bevy::text::Text2d("Hello Text2d".to_string()),
+ bevy::sprite::Text2d("Hello Text2d".to_string()),
```

**2. Update Text2d comment reference in `test-app/examples/extras_plugin.rs`**
```diff
- // Entity with Text2d for testing mutations
+ // Entity with Text2d for testing mutations
```
(Comment remains valid as it's just a description)

**3. Update Text2d name reference in `test-app/examples/extras_plugin.rs`**
```diff
- Name::new("Text2dTestEntity"),
+ Name::new("Text2dTestEntity"),
```
(Name remains valid as it's just identifying the entity)

### Search Pattern

To find all occurrences:
```bash
rg "bevy::text::Text2d" --type rust
```

---

## HIGH Priority Changes

## Reflect Registration Changes

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/reflect_registration_changes.md`
**Requirement Level:** HIGH
**Occurrences:** 44 locations across 1 file
**Pass 1 Count:** 145 | **Pass 2 Count:** 100 | **Status:** ANOMALY: -31%

### Migration Guide Summary

Bevy 0.17 now automatically registers types implementing `Reflect` using compiler magic, eliminating the need for most manual `.register_type` calls. This requires enabling the `reflect_auto_register` feature (included in default features) or the fallback `reflect_auto_register_static` feature. Generic types still require manual registration.

### Required Changes

**1. Remove manual registration for `TestConfigResource` in `test-app/examples/extras_plugin.rs`**
```diff
        .insert_resource(CurrentPort(port))
-       // Register test resources
-       .register_type::<TestConfigResource>()
-       .register_type::<RuntimeStatsResource>()
        // Register test components
```

**2. Remove manual registration for `TestStructWithSerDe` in `test-app/examples/extras_plugin.rs`**
```diff
-       // Register test components
-       .register_type::<TestStructWithSerDe>()
-       .register_type::<TestStructNoSerDe>()
```

**3. Remove manual registration for `SimpleSetComponent` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<SimpleSetComponent>()
-       .register_type::<TestMapComponent>()
```

**4. Remove manual registration for `TestEnumKeyedMap` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestEnumKeyedMap>()
-       .register_type::<SimpleTestEnum>()
```

**5. Remove manual registration for `TestEnumWithSerDe` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestEnumWithSerDe>()
-       .register_type::<NestedConfigEnum>()
```

**6. Remove manual registration for `SimpleNestedEnum` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<SimpleNestedEnum>()
-       .register_type::<OptionTestEnum>()
```

**7. Remove manual registration for `WrapperEnum` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<WrapperEnum>()
-       .register_type::<TestVariantChainEnum>()
```

**8. Remove manual registration for `MiddleStruct` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<MiddleStruct>()
-       .register_type::<BottomEnum>()
```

**9. Remove manual registration for `TestEnumNoSerDe` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestEnumNoSerDe>()
-       .register_type::<TestArrayField>()
```

**10. Remove manual registration for `TestArrayTransforms` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestArrayTransforms>()
-       .register_type::<TestTupleField>()
```

**11. Remove manual registration for `TestTupleStruct` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestTupleStruct>()
-       .register_type::<TestComplexTuple>()
```

**12. Remove manual registration for `TestComplexComponent` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestComplexComponent>()
-       .register_type::<TestCollectionComponent>()
```

**13. Remove manual registration for `TestMixedMutabilityCore` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestMixedMutabilityCore>()
-       .register_type::<TestMixedMutabilityVec>()
```

**14. Remove manual registration for `TestMixedMutabilityArray` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestMixedMutabilityArray>()
-       .register_type::<TestMixedMutabilityTuple>()
```

**15. Remove manual registration for `TestMixedMutabilityEnum` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestMixedMutabilityEnum>()
-       .register_type::<TestPartiallyMutableNested>()
```

**16. Remove manual registration for `TestDeeplyNested` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<TestDeeplyNested>()
        // Register gamepad types for BRP access
```

**17. Remove manual registration for `Gamepad` in `test-app/examples/extras_plugin.rs`**
```diff
-       // Register gamepad types for BRP access
-       .register_type::<Gamepad>()
-       .register_type::<GamepadSettings>()
```

**18. Remove manual registration for `Screenshot` in `test-app/examples/extras_plugin.rs`**
```diff
-       // Register Screenshot type for BRP access
-       .register_type::<Screenshot>()
```

**19. Remove manual registration for `MotionVectorPrepass` in `test-app/examples/extras_plugin.rs`**
```diff
-       // Register missing components for BRP access
-       .register_type::<MotionVectorPrepass>()
-       .register_type::<NotShadowCaster>()
```

**20. Remove manual registration for `NotShadowReceiver` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<NotShadowReceiver>()
-       .register_type::<VolumetricLight>()
```

**21. Remove manual registration for `OcclusionCulling` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<OcclusionCulling>()
-       .register_type::<NoFrustumCulling>()
```

**22. Remove manual registration for `CalculatedClip` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<CalculatedClip>()
-       .register_type::<Button>()
```

**23. Remove manual registration for `Label` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<Label>()
-       .register_type::<BorderRadius>()
```

**24. Remove final manual registration for `BorderRadius` in `test-app/examples/extras_plugin.rs`**
```diff
-       .register_type::<BorderRadius>()
        .add_systems(
```

### Search Pattern

To find all occurrences:
```bash
rg "\.register_type" --type rust
```

**Note:** This is a test application, so these changes are HIGH priority rather than REQUIRED. The `reflect_auto_register` feature is enabled by default in Bevy 0.17, so all these manual registrations can be safely removed for non-generic types. The count variance is explained by Pass 1 counting multiple pattern matches (register_type, .register_type, Reflect) across the same lines, while Pass 2 focuses on the actual actionable occurrences.

---

## Event trait split / Rename

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/event_split.md`
**Requirement Level:** HIGH
**Occurrences:** 5 locations across 3 files
**Pass 1 Count:** 51 | **Pass 2 Count:** 51 | **Status:** ANOMALY: +0%

### Migration Guide Summary

Bevy 0.17 splits the `Event` concept for clarity: "buffered events" (sent/read via `EventWriter`/`EventReader`) are now called "messages" using the `Message` trait and `MessageWriter`/`MessageReader`/`Messages<M>`. The `Event` trait is now reserved for "observable events" only. Most types should use either `Message` or `Event`, not both.

### Required Changes

**Note:** The 46 "Message" occurrences in Pass 1 are from documentation comments, error variant names, and field names in the MCP codebase (e.g., `message_template`, `MissingMessageTemplate`). These are NOT Bevy event-related and should NOT be changed. The actual migration applies only to the 5 Bevy event system usages below.

**1. Update EventWriter in `extras/src/keyboard.rs:536`**
```diff
  pub fn process_timed_key_releases(
      mut commands: Commands,
      time: Res<Time>,
      mut query: Query<(Entity, &mut TimedKeyRelease)>,
-     mut keyboard_events: EventWriter<bevy::input::keyboard::KeyboardInput>,
+     mut keyboard_events: MessageWriter<bevy::input::keyboard::KeyboardInput>,
  ) {
```

**2. Update EventWriter in `extras/src/shutdown.rs:38`**
```diff
  pub fn deferred_shutdown_system(
      pending: Option<ResMut<PendingShutdown>>,
-     mut exit: EventWriter<bevy::app::AppExit>,
+     mut exit: MessageWriter<bevy::app::AppExit>,
  ) {
```

**3. Update EventReader in `test-app/examples/extras_plugin.rs:1343`**
```diff
  #[allow(clippy::assigning_clones)] // clone_from doesn't work due to borrow checker
  fn track_keyboard_input(
-     mut events: EventReader<KeyboardInput>,
+     mut events: MessageReader<KeyboardInput>,
      mut history: ResMut<KeyboardInputHistory>,
  ) {
```

**4. Update EventReader in `test-duplicate-a/examples/extras_plugin_duplicate.rs`**
```diff
  #[allow(clippy::assigning_clones)] // clone_from doesn't work due to borrow checker
  fn track_keyboard_input(
-     mut events: EventReader<KeyboardInput>,
+     mut events: MessageReader<KeyboardInput>,
      mut history: ResMut<KeyboardInputHistory>,
  ) {
```

**5. Update EventReader in `test-duplicate-b/examples/extras_plugin_duplicate.rs`**
```diff
  #[allow(clippy::assigning_clones)] // clone_from doesn't work due to borrow checker
  fn track_keyboard_input(
-     mut events: EventReader<KeyboardInput>,
+     mut events: MessageReader<KeyboardInput>,
      mut history: ResMut<KeyboardInputHistory>,
  ) {
```

### Search Pattern

To find all occurrences:
```bash
rg "EventWriter|EventReader" --type rust
```

---

## Rename `send_event` and similar methods to `write_message`

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/send_event_rename.md`
**Requirement Level:** HIGH
**Occurrences:** 3 locations across 2 files
**Pass 1 Count:** 3 | **Pass 2 Count:** 3 | **Status:** MATCH

### Migration Guide Summary

Following up on the EventWriter::send being renamed to EventWriter::write in 0.16, many similar methods have been renamed in Bevy 0.17.1. "Buffered events" are now known as Messages, and the naming reflects this change. This includes both World and Commands message methods, with the old methods being deprecated.

### Required Changes

**1. Update World::send_event to World::write_message in `extras/src/keyboard.rs`**
```diff
     // Always send press events first
     let press_events = create_keyboard_events(&key_codes, true);
     for event in press_events {
-        world.send_event(event);
+        world.write_message(event);
     }
```

**2. Update EventWriter declaration in `extras/src/keyboard.rs`**
```diff
 pub fn process_timed_key_releases(
     mut commands: Commands,
     time: Res<Time>,
     mut query: Query<(Entity, &mut TimedKeyRelease)>,
-    mut keyboard_events: EventWriter<bevy::input::keyboard::KeyboardInput>,
+    mut keyboard_events: EventWriter<bevy::input::keyboard::KeyboardInput>,
 ) {
```
Note: EventWriter itself is not renamed - this is just a parameter declaration. The method `.write()` used on line 554 is already correct and was updated in 0.16.

**3. Update EventWriter declaration in `extras/src/shutdown.rs`**
```diff
 /// System to handle deferred shutdown
 pub fn deferred_shutdown_system(
     pending: Option<ResMut<PendingShutdown>>,
-    mut exit: EventWriter<bevy::app::AppExit>,
+    mut exit: EventWriter<bevy::app::AppExit>,
 ) {
```
Note: EventWriter itself is not renamed - this is just a parameter declaration. The method `.write()` used on line 45 is already correct and was updated in 0.16.

### Search Pattern

To find all occurrences:
```bash
rg "\.send_event\(" --type rust
```

---

## TextShadow is moved to widget::text module

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/textshadow_is_moved_to_widget_text_module.md`
**Requirement Level:** HIGH
**Occurrences:** 1 locations across 1 files
**Pass 1 Count:** 1 | **Pass 2 Count:** 1 | **Status:** MATCH

### Migration Guide Summary

The `TextShadow` component has been moved from `bevy::prelude` to `bevy::ui::widget::text`. Code using `TextShadow` from the prelude needs to update imports to the new module location.

### Required Changes

**1. Update TextShadow import in `test-app/examples/extras_plugin.rs`**
```diff
- bevy::prelude::TextShadow {
+ bevy::ui::widget::text::TextShadow {
      offset: Vec2::new(2.0, 2.0),
      color: Color::srgba(0.0, 0.0, 0.0, 0.5),
  },
```

### Search Pattern

To find all occurrences:
```bash
rg "TextShadow" --type rust
```

---

## LOW Priority Changes

## bevy_render reorganization

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/bevy_render_reorganization.md`
**Requirement Level:** LOW
**Occurrences:** 3 locations across 3 files
**Pass 1 Count:** 31 | **Pass 2 Count:** 3 | **Status:** ANOMALY: -90%

### Migration Guide Summary

Bevy 0.17 reorganized `bevy_render` by moving types into specialized crates: `bevy_camera` for camera types, `bevy_shader` for shader types, `bevy_light` for lighting, `bevy_mesh` for meshes, `bevy_image` for images, `bevy_ui_render` for UI rendering, `bevy_sprite_render` for sprite rendering, and `bevy_anti_alias`/`bevy_post_process` for post-processing effects. Most references in this codebase appear in documentation strings and comments rather than actual type usage.

### Required Changes

**1. Update documentation example in `mcp/src/brp_tools/tools/bevy_registry_schema.rs`**
```diff
- /// Exclude types from these crates (e.g., [`bevy_render`, `bevy_pbr`])
+ /// Exclude types from these crates (e.g., [`bevy_camera`, `bevy_pbr`])
```

**2. Update type constant in `mcp/src/brp_tools/brp_type_guide/constants.rs`**
```diff
- pub const TYPE_BEVY_IMAGE_HANDLE: &str = "bevy_asset::handle::Handle<bevy_image::image::Image>";
+ // No change needed - bevy_image is the correct new crate name after reorganization
```

**3. Update documentation comment in `mcp/src/brp_tools/brp_client/constants.rs`**
```diff
- /// `ClearColor`   "Error accessing element with .red access(offset 3): Expected variant field
+ // No change needed - this is documentation of an example error message, not actual code
```

### Search Pattern

To find all occurrences:
```bash
rg "bevy_render|bevy_image|bevy_sprite_render|ClearColor|ViewVisibility|Mesh2d" --type rust
```

---

## Move cursor-related types from bevy_winit to bevy_window

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/cursor-android.md`
**Requirement Level:** LOW
**Occurrences:** 4 locations across 1 file
**Pass 1 Count:** 5 | **Pass 2 Count:** 4 | **Status:** MATCH (-20% - likely due to comment not being counted)

### Migration Guide Summary

Cursor-related types have been moved from `bevy_winit` to `bevy_window` to reduce dependencies. This includes `CursorIcon`, `CustomCursor`, `CustomCursorImage`, and `CustomCursorUrl`. The import path must be updated from `bevy_winit::cursor` to `bevy::window` or `bevy_window`.

### Required Changes

**1. Update import statement in `test-app/examples/extras_plugin.rs`**
```diff
- use bevy_winit::cursor::CursorIcon;
+ use bevy::window::CursorIcon;
```

**2. Update comment reference in `test-app/examples/extras_plugin.rs` (line 697)**

No code change needed - this is a comment that mentions `CursorIcon` but doesn't require migration.

**3. Update CursorIcon usage in `test-app/examples/extras_plugin.rs` (line 699)**

The actual usage on line 699 already uses the correct `bevy::window::SystemCursorIcon::Default` enum variant, so once the import is updated, this will work correctly without further changes:
```rust
CursorIcon::System(bevy::window::SystemCursorIcon::Default),
```

### Search Pattern

To find all occurrences:
```bash
rg "bevy_winit::cursor" --type rust
```

---

## Observers May Not Be Exclusive

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/observers_may_not_be_exclusive.md`
**Requirement Level:** LOW
**Occurrences:** 0 locations across 0 files
**Pass 1 Count:** 5 | **Pass 2 Count:** 0 | **Status:** ANOMALY: Pass 1 found `&mut World` in systems/handlers, not observers

### Migration Guide Summary

Bevy 0.17.1 no longer allows exclusive systems (`&mut World` parameters) to be used as observers. This was never sound as the engine keeps references alive during observer invocation. Instead, observers should use `DeferredWorld` for non-structural changes or `Commands` for structural changes.

### Required Changes

No changes required. The codebase has 5 instances of `&mut World` usage, but none are in observer contexts:

1. **BRP method handlers** (4 instances) - These are regular systems registered via `RemotePlugin.with_method()`, not observers. They are converted to systems using `IntoSystem::into_system()`, which is valid.
   - `extras/src/window_title.rs:10` - `handler(In(params): In<Option<Value>>, world: &mut World)`
   - `extras/src/screenshot.rs:16` - `handler(In(params): In<Option<Value>>, world: &mut World)`
   - `extras/src/keyboard.rs:458` - `send_keys_handler(In(params): In<Option<Value>>, world: &mut World)`
   - `extras/src/shutdown.rs:17` - `handler(In(_): In<Option<Value>>, world: &mut World)`

2. **Startup system** (1 instance) - This is a regular system added via `add_systems(Startup, ...)`, not an observer:
   - `extras/src/plugin.rs:114` - `app.add_systems(Startup, move |_world: &mut World| { ... })`

3. **Observers** (1 instance) - The one observer in the codebase correctly uses `Trigger<T>` instead of `&mut World`:
   - `extras/src/screenshot.rs:76` - `.observe(move |trigger: Trigger<ScreenshotCaptured>| { ... })` ✅

### Search Pattern

To find observers with `&mut World` parameters (none found):
```bash
rg "\.observe.*&mut World" --type rust
```

---

## ChromaticAberration LUT is now Option

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/chromatic_aberration_option.md`
**Requirement Level:** LOW
**Occurrences:** 2 locations across 1 files
**Pass 1 Count:** 4 | **Pass 2 Count:** 2 | **Status:** ANOMALY: -50%

### Migration Guide Summary

The `ChromaticAberration` component's `color_lut` field changed from `Handle<Image>` to `Option<Handle<Image>>`, allowing it to fall back to a default image when `None`. Users assigning custom LUTs must wrap values in `Some`.

### Required Changes

The bevy_brp codebase only uses `ChromaticAberration::default()` and does not directly access or set the `color_lut` field. No changes are required as the default constructor handles the migration automatically. The component is used in the test app for mutation testing purposes.

**Note on occurrence count variance:** Pass 1 found 4 occurrences (likely counting "ChromaticAberration" and "color_lut" separately, 2+2=4), while Pass 2 found 2 actual uses of the `ChromaticAberration` type (import and default instantiation). There are 0 occurrences of `color_lut` field access in the codebase. This is informational only as no code changes are needed.

### Search Pattern

To find all occurrences:
```bash
rg "ChromaticAberration" --type rust
rg "color_lut" --type rust
```

---

## Replaced TextFont constructor methods with From impls

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/remove_text_font_from_constructor_methods.md`
**Requirement Level:** LOW
**Occurrences:** 0 locations requiring migration (6 TextFont usages already compliant)
**Pass 1 Count:** 6 | **Pass 2 Count:** 6 | **Status:** MATCH

### Migration Guide Summary

The `TextFont::from_font` and `TextFont::from_line_height` constructor methods have been removed in favor of `From` trait implementations. Code should now use `TextFont::from(font_handle)` instead of `TextFont::from_font(font_handle)` and `TextFont::from(line_height)` instead of `TextFont::from_line_height(line_height)`.

### Required Changes

No changes required. All 6 occurrences of `TextFont` in the codebase use struct initialization syntax (`TextFont { font_size: ..., ..default() }`), not the removed constructor methods.

**Verified locations (already compliant):**
- test-duplicate-b/examples/extras_plugin_duplicate.rs (1 occurrence)
- test-app/src/bin/test_app.rs (1 occurrence)
- test-duplicate-a/examples/extras_plugin_duplicate.rs (1 occurrence)
- test-app/examples/extras_plugin.rs (2 occurrences)
- test-app/examples/no_extras_plugin.rs (1 occurrence)

### Search Pattern

To find all occurrences:
```bash
rg "TextFont::(from_font|from_line_height)" --type rust
```

---

## Use glTF material names for spawned primitive entities

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/rename_spawn_gltf_material_name.md`
**Requirement Level:** LOW
**Occurrences:** 3 locations across 1 files
**Pass 1 Count:** 3 | **Pass 2 Count:** 3 | **Status:** MATCH

### Migration Guide Summary

Bevy's glTF loader now uses material names when naming primitive entities instead of primitive indices. The `Name` component on mesh primitives changed from `MeshName.0` format to `MeshName.MaterialName` format. If code relied on the previous `Name` value, it should use the new `GltfMeshName` component instead.

### Required Changes

No changes required. The codebase uses `GltfMaterialName` component in test code for spawning test entities with glTF material names. This usage is compatible with Bevy 0.17.1 - the component itself was not renamed or deprecated, only the behavior of how primitive entity `Name` components are populated changed.

### Search Pattern

To find all occurrences:
```bash
rg "GltfMaterialName" --type rust
```

---

## Fix `From<Rot2>` implementation for `Mat2`

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/rot2_matrix_construction.md`
**Requirement Level:** LOW
**Occurrences:** 2 locations across 1 file
**Pass 1 Count:** 2 | **Pass 2 Count:** 2 | **Status:** MATCH

### Migration Guide Summary

The `From<Rot2>` implementation for `Mat2` was corrected from producing clockwise rotation (the inverse) to producing counterclockwise rotation (the correct form). This affects rotation matrices created using this conversion, which now rotate counterclockwise as intended rather than clockwise. The codebase only references `Mat2` as type name constants for documentation purposes.

### Required Changes

No changes required. The codebase only contains string constants referencing the `Mat2` type name for type guide documentation purposes. These are used to identify and document the `Mat2` type but do not construct rotation matrices from `Rot2` or use the `From<Rot2>` implementation. The occurrences are:

**1. Type constant for `bevy_math::mat2::Mat2` in `mcp/src/brp_tools/brp_type_guide/constants.rs`**
```rust
// Line 70 - No change needed, this is just a type name constant
pub const TYPE_BEVY_MAT2: &str = "bevy_math::mat2::Mat2";
```

**2. Type constant for `glam::Mat2` in `mcp/src/brp_tools/brp_type_guide/constants.rs`**
```rust
// Line 87 - No change needed, this is just a type name constant
pub const TYPE_GLAM_MAT2: &str = "glam::Mat2";
```

### Search Pattern

To find all occurrences:
```bash
rg "Mat2" --type rust
```

---

## TAA is no longer experimental

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides/taa_non_experimental.md`
**Requirement Level:** LOW
**Occurrences:** 2 locations across 1 file
**Pass 1 Count:** 2 | **Pass 2 Count:** 2 | **Status:** MATCH

### Migration Guide Summary

TAA (Temporal Anti-Aliasing) is no longer experimental in Bevy 0.17. The `TemporalAntiAliasPlugin` is now part of `DefaultPlugins` via `AntiAliasPlugin`, and import paths have changed from `bevy::anti_alias::experimental::taa` to `bevy::anti_alias::taa`. Additionally, `TemporalAntiAliasing` now uses `MipBias` as a required component in the main world instead of manually overriding it in the render world.

### Required Changes

**Note:** The bevy_brp codebase uses `MipBias` correctly in its test application. No migration changes are required because:
1. `MipBias` is already imported from the correct path (`bevy::render::camera::MipBias`)
2. It's already used as a component in the main world (not render world)
3. The codebase doesn't use `TemporalAntiAliasing` or `TemporalAntiAliasPlugin` directly

The two occurrences of `MipBias` found are:
- Import statement at line 40
- Component instantiation at line 182

Both are already following the new Bevy 0.17 pattern and require no changes.

### Search Pattern

To find all occurrences:
```bash
rg "MipBias|TemporalAntiAlias" --type rust
```

---

## Guides Not Applicable to This Codebase

The following 99 guides from Bevy 0.17.1 do not apply to this codebase:

- release-content/migration-guides/LightVisibilityClass_rename.md
- release-content/migration-guides/Newtype_ScrollPosition.md
- release-content/migration-guides/RenderTargetInfo_default.md
- release-content/migration-guides/UI_scroll_position_is_now_logical.md
- release-content/migration-guides/animation_graph_no_more_asset_ids.md
- release-content/migration-guides/assets-insert-result.md
- release-content/migration-guides/check_change_ticks.md
- release-content/migration-guides/clone_behavior_no_longer_eq.md
- release-content/migration-guides/combine_soundness_fix.md
- release-content/migration-guides/component-lifecycle-module.md
- release-content/migration-guides/component_entry.md
- release-content/migration-guides/components-registrator-derefmut.md
- release-content/migration-guides/composable_specialization.md
- release-content/migration-guides/compressed-image-saver.md
- release-content/migration-guides/deprecate_iter_entities.md
- release-content/migration-guides/dragenter_includes_dragged_entity.md
- release-content/migration-guides/dynamic-bundle-movingptr.md
- release-content/migration-guides/entities_apis.md
- release-content/migration-guides/entity_cloner_builder_split.md
- release-content/migration-guides/entity_representation.md
- release-content/migration-guides/extract-picking-plugin-members.md
- release-content/migration-guides/extract-pointer-input-plugin-members.md
- release-content/migration-guides/extract_fn_is_mut.md
- release-content/migration-guides/extract_ui_text_colors_per_glyph.md
- release-content/migration-guides/extracted_uinodes_z_order.md
- release-content/migration-guides/fullscreen_shader_resource.md
- release-content/migration-guides/gated_reader.md
- release-content/migration-guides/generic-option-parameter.md
- release-content/migration-guides/glam-rand-upgrades.md
- release-content/migration-guides/gles_optional.md
- release-content/migration-guides/gltf-animation-load-optional.md
- release-content/migration-guides/handle_weak_replaced_with_handle_uuid.md
- release-content/migration-guides/hdr_component.md
- release-content/migration-guides/incorrect-type-error-on-run-system-command.md
- release-content/migration-guides/internal_entities.md
- release-content/migration-guides/interned-labels-cleanup.md
- release-content/migration-guides/labeled_asset_scope_errors.md
- release-content/migration-guides/log-diagnostics-hash-set.md
- release-content/migration-guides/map_set_apply.md
- release-content/migration-guides/merge_observerState_observer_single_component.md
- release-content/migration-guides/mesh_compute_smooth_normals.md
- release-content/migration-guides/non-generic-access.md
- release-content/migration-guides/overflowclipbox_default_is_now_paddingbox.md
- release-content/migration-guides/parallelism_strategy_changes.md
- release-content/migration-guides/per-world-error-handler.md
- release-content/migration-guides/picking_location_not_component.md
- release-content/migration-guides/pointer_target.md
- release-content/migration-guides/primitives_non_const_generic_meshable.md
- release-content/migration-guides/query_items_borrow_from_query_state.md
- release-content/migration-guides/rangefinder.md
- release-content/migration-guides/reflect_asset_asset_ids.md
- release-content/migration-guides/relationship_set_risky.md
- release-content/migration-guides/relative_cursor_position_is_object_centered.md
- release-content/migration-guides/remove_archetype_component_id.md
- release-content/migration-guides/remove_bundle_register_required_components.md
- release-content/migration-guides/remove_cosmic_text_reexports.md
- release-content/migration-guides/remove_default_extend_from_iter.md
- release-content/migration-guides/remove_deprecated_batch_spawning.md
- release-content/migration-guides/remove_scale_value.md
- release-content/migration-guides/remove_the_add_sub_impls_on_volume.md
- release-content/migration-guides/removed_components_stores_messages.md
- release-content/migration-guides/rename-justifytext.md
- release-content/migration-guides/rename_condition.md
- release-content/migration-guides/rename_pointer_events.md
- release-content/migration-guides/rename_state_scoped.md
- release-content/migration-guides/rename_timer_paused_and_finished.md
- release-content/migration-guides/rename_transform_compute_matrix.md
- release-content/migration-guides/renamed_computednodetarget.md
- release-content/migration-guides/render_graph_app_to_ext.md
- release-content/migration-guides/render_startup.md
- release-content/migration-guides/render_target_info_error.md
- release-content/migration-guides/replace_non_send_resources.md
- release-content/migration-guides/required_components_rework.md
- release-content/migration-guides/rework_merge_mesh_error.md
- release-content/migration-guides/scalar-field-on-vector-space.md
- release-content/migration-guides/scene_spawner_api.md
- release-content/migration-guides/schedule_cleanup.md
- release-content/migration-guides/separate-border-colors.md
- release-content/migration-guides/simple_executor_going_away.md
- release-content/migration-guides/spawnable-list-movingptr.md
- release-content/migration-guides/specialized_ui_transform.md
- release-content/migration-guides/split-window.md
- release-content/migration-guides/split_up_computeduitargetcamera.md
- release-content/migration-guides/stack_z_offsets_changes.md
- release-content/migration-guides/state_scoped_entities_by_default.md
- release-content/migration-guides/stop-exposing-minimp3.md
- release-content/migration-guides/stop_storing_system_access.md
- release-content/migration-guides/sync_cell_utils.md
- release-content/migration-guides/system_run_returns_result.md
- release-content/migration-guides/system_set_naming_convention.md
- release-content/migration-guides/texture_format_pixel_size_returns_result.md
- release-content/migration-guides/ui-debug-overlay.md
- release-content/migration-guides/unified_system_state_flag.md
- release-content/migration-guides/view-transformations.md
- release-content/migration-guides/wayland.md
- release-content/migration-guides/wgpu_25.md
- release-content/migration-guides/window_resolution_constructors.md
- release-content/migration-guides/zstd.md

---

## Next Steps

1. Start with REQUIRED changes (must fix to compile with Bevy 0.17.1)
   - Update all BRP method names (73 occurrences)
   - Fix Sprite anchor field migration (2 occurrences)
   - Update Trigger to On in observers (1 occurrence)
   - Move Text2d imports from bevy_text to bevy_sprite (3 occurrences)

2. Address HIGH priority changes (deprecated features)
   - Remove manual .register_type calls (44 occurrences)
   - Update EventWriter/EventReader to MessageWriter/MessageReader (5 occurrences)
   - Rename World::send_event to World::write_message (3 occurrences)
   - Update TextShadow import path (1 occurrence)

3. Consider MEDIUM and LOW priority improvements
   - LOW: Documentation updates for bevy_render reorganization
   - LOW: CursorIcon import path updates
   - LOW: Informational changes (no code changes needed)

4. Test thoroughly after each category of changes
5. Run `cargo check` and `cargo test` frequently

---

## Reference

- **Migration guides directory:** /Users/natemccoy/rust/bevy-0.17.1/release-content/migration-guides
- **Bevy 0.17.1 release notes:** https://github.com/bevyengine/bevy/releases/tag/v0.17.1
