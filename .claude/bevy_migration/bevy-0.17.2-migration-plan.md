# Bevy 0.17.2 Migration Plan

**Generated:** 2025-10-04
**Codebase:** /Users/natemccoy/rust/bevy_brp
**Total Applicable Guides:** 13

---

## Summary

- **REQUIRED changes:** 5 guides (176 total occurrences)
- **HIGH priority:** 2 guides (108 total occurrences)
- **MEDIUM priority:** 2 guides (46 total occurrences)
- **LOW priority:** 4 guides (18 total occurrences)

**Count Anomalies:** 2 guides with >20% variance between Pass 1 and Pass 2
- observer_and_event_changes.md: Pass 1=74, Pass 2=8 (-89% - false positives in Pass 1)
- internal_entities.md: Pass 1=10, Pass 2=0 (-100% - false positives in Pass 1)

**Estimated effort:**
- REQUIRED: **Large** (must fix to compile with Bevy 0.17.2)
- HIGH: **Medium** (deprecated APIs that need migration)
- MEDIUM: **Small** (optional improvements)
- LOW: **Small** (informational only)

---

## ⚠️ Dependency Compatibility Review

**Status:** No bevy-related dependencies found in this project

---

## REQUIRED Changes

### 1. Renamed BRP Methods

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/renamed_BRP_methods.md`
**Requirement Level:** REQUIRED
**Occurrences:** 97 locations across multiple files
**Pass 1 Count:** 94 | **Pass 2 Count:** 97 | **Status:** MATCH (+3%)

#### Migration Guide Summary

Bevy 0.17.2 has renamed ALL Bevy Remote Protocol (BRP) methods to be more explicit and consistent. All component methods now use `world.*_components` pattern, all resource methods use `world.*_resources` pattern, and "destroy" has been renamed to "despawn" throughout.

#### Required Changes

**Complete Method Mapping:**

| Old Method | New Method | Occurrences |
|-----------|------------|-------------|
| `bevy/query` | `world.query` | 5 |
| `bevy/spawn` | `world.spawn_entity` | 5 |
| `bevy/destroy` | `world.despawn_entity` | 5 |
| `bevy/reparent` | `world.reparent_entities` | 4 |
| `bevy/get` | `world.get_components` | 13 |
| `bevy/insert` | `world.insert_components` | 8 |
| `bevy/remove` | `world.remove_components` | 8 |
| `bevy/list` | `world.list_components` | 14 |
| `bevy/mutate` | `world.mutate_components` | 8 |
| `bevy/get+watch` | `world.get_components+watch` | 3 |
| `bevy/list+watch` | `world.list_components+watch` | 3 |
| `bevy/get_resource` | `world.get_resources` | 4 |
| `bevy/insert_resource` | `world.insert_resources` | 4 |
| `bevy/remove_resource` | `world.remove_resources` | 4 |
| `bevy/list_resources` | `world.list_resources` | 4 |
| `bevy/mutate_resource` | `world.mutate_resources` | 4 |
| `bevy/registry/schema` | `registry.schema` | 4 |

**1. Update all BRP tool attributes in `mcp/src/tool/tool_name.rs`**
```diff
- #[brp_tool(brp_method = "bevy/spawn")]
+ #[brp_tool(brp_method = "world.spawn_entity")]

- #[brp_tool(brp_method = "bevy/destroy")]
+ #[brp_tool(brp_method = "world.despawn_entity")]

- #[brp_tool(brp_method = "bevy/get")]
+ #[brp_tool(brp_method = "world.get_components")]

(... and 12 more method renames)
```

**2. Update documentation comments in all tool implementation files:**
- `mcp/src/brp_tools/tools/bevy_query.rs`
- `mcp/src/brp_tools/tools/bevy_spawn.rs`
- `mcp/src/brp_tools/tools/bevy_destroy.rs`
- (... 12 more files)

**3. Consider renaming for consistency:**
- `BevyDestroy` enum variant → `BevyDespawn`
- `bevy_destroy.rs` file → `bevy_despawn.rs`

#### Search Pattern

```bash
rg "bevy/(query|spawn|destroy|get|insert|remove|list|mutate|reparent)" --type rust
rg "bevy/(get|insert|remove|list|mutate)_resource" --type rust
rg "bevy/registry/schema" --type rust
```

---

### 2. Anchor is Removed from Sprite

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/anchor_is_removed_from_sprite.md`
**Requirement Level:** REQUIRED
**Occurrences:** 11 locations across 1 file
**Pass 1 Count:** 11 | **Pass 2 Count:** 11 | **Status:** MATCH

#### Migration Guide Summary

The `anchor` field has been removed from the `Sprite` struct. `Anchor` is now a required component that must be spawned alongside `Sprite`. Additionally, anchor variants have been renamed to SCREAMING_SNAKE_CASE constants.

#### Required Changes

**1. Update Sprite spawning in `test-app/examples/extras_plugin.rs:137`**
```diff
- Sprite {
-     color: Color::srgb(1.0, 0.5, 0.25),
-     custom_size: Some(Vec2::new(64.0, 64.0)),
-     anchor: bevy::sprite::Anchor::Center,
-     ..default()
- },
+ Sprite {
+     color: Color::srgb(1.0, 0.5, 0.25),
+     custom_size: Some(Vec2::new(64.0, 64.0)),
+     ..default()
+ },
+ Anchor::Center,  // Now a separate required component
```

#### Search Pattern

```bash
rg "Sprite.*anchor:" --type rust
rg "Anchor::" --type rust
```

---

### 3. Bevy Render Reorganization

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/bevy_render_reorganization.md`
**Requirement Level:** REQUIRED
**Occurrences:** 64 locations across 3 files
**Pass 1 Count:** 64 | **Pass 2 Count:** 64 | **Status:** MATCH

#### Migration Guide Summary

Major reorganization of rendering crates - `bevy_render` split into specialized crates (`bevy_camera`, `bevy_anti_alias`, `bevy_post_process`, etc.). Import paths must be updated throughout.

#### Required Changes

**1. Update anti-aliasing imports in `test-app/examples/extras_plugin.rs`**
```diff
- use bevy::core_pipeline::fxaa::Fxaa;
- use bevy::core_pipeline::smaa::Smaa;
+ use bevy::anti_alias::fxaa::Fxaa;
+ use bevy::anti_alias::smaa::Smaa;
```

**2. Update post-processing imports in `test-app/examples/extras_plugin.rs`**
```diff
- use bevy::core_pipeline::bloom::Bloom;
- use bevy::core_pipeline::dof::DepthOfField;
- use bevy::core_pipeline::post_process::ChromaticAberration;
+ use bevy::post_process::bloom::Bloom;
+ use bevy::post_process::dof::DepthOfField;
+ use bevy::post_process::ChromaticAberration;
```

**3. Update camera imports in `test-app/examples/extras_plugin.rs`**
```diff
- use bevy::render::view::visibility::NoFrustumCulling;
- use bevy::render::view::visibility::VisibilityRange;
- use bevy::render::view::visibility::RenderLayers;
+ use bevy::camera::visibility::NoFrustumCulling;
+ use bevy::camera::visibility::VisibilityRange;
+ use bevy::camera::visibility::RenderLayers;
```

**4. Update mutation knowledge in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`**
```diff
- "bevy_core_pipeline::core_3d::camera_3d::Camera3d"
+ "bevy_camera::camera_3d::Camera3d"
```

#### Search Pattern

```bash
rg "bevy::core_pipeline::(fxaa|smaa|bloom|dof)" --type rust
rg "bevy::render::view::visibility" --type rust
```

---

### 4. Text2d Moved to bevy_sprite

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/text2d_moved_to_bevy_sprite.md`
**Requirement Level:** REQUIRED
**Occurrences:** 9 locations across 1 file
**Pass 1 Count:** 8 | **Pass 2 Count:** 9 | **Status:** MATCH (+12%)

#### Migration Guide Summary

World-space text types `Text2d` and `Text2dShadow` have been moved from `bevy_text` to `bevy_sprite`.

#### Required Changes

**1. Update Text2d path in `test-app/examples/extras_plugin.rs:1121`**
```diff
- bevy::text::Text2d("Hello Text2d".to_string()),
+ bevy::sprite::Text2d("Hello Text2d".to_string()),
```

#### Search Pattern

```bash
rg "bevy::text::Text2d" --type rust
```

---

### 5. Cursor Android (Move to bevy_window)

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/cursor-android.md`
**Requirement Level:** REQUIRED
**Occurrences:** 6 locations across 1 file
**Pass 1 Count:** 5 | **Pass 2 Count:** 6 | **Status:** MATCH (+20%)

#### Migration Guide Summary

Cursor-related types moved from `bevy_winit` to `bevy_window` to reduce dependencies.

#### Required Changes

**1. Update import in `test-app/examples/extras_plugin.rs:53`**
```diff
- use bevy_winit::cursor::CursorIcon;
+ use bevy::window::CursorIcon;
```

#### Search Pattern

```bash
rg "bevy_winit::cursor" --type rust
```

---

## HIGH Priority Changes

### 6. Reflect Registration Changes

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/reflect_registration_changes.md`
**Requirement Level:** HIGH
**Occurrences:** 44 locations across 1 file
**Pass 1 Count:** 44 | **Pass 2 Count:** 44 | **Status:** MATCH

#### Migration Guide Summary

Bevy 0.17 introduces automatic type registration for reflected types. Most manual `.register_type()` calls can be removed, except for generic types.

#### Required Changes

**1. Remove non-generic type registrations in `test-app/examples/extras_plugin.rs` (lines 505-552)**

Can safely remove ~35 of the 44 `register_type` calls for simple custom types.

**Keep registrations for:**
- Generic types containing `HashMap`, `HashSet`, `Vec`, `Option`, `Arc`
- Bevy built-in types (until verified)

```diff
  // Keep these (generic types):
  .register_type::<SimpleSetComponent>()  // Has HashSet
  .register_type::<TestMapComponent>()    // Has HashMap

  // Can remove these (simple types):
- .register_type::<SimpleTestComponent>()
- .register_type::<AnotherSimpleComponent>()
```

#### Search Pattern

```bash
rg "register_type::<" --type rust
```

---

### 7. Send Event Rename

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/send_event_rename.md`
**Requirement Level:** HIGH
**Occurrences:** 3 locations across 1 file
**Pass 1 Count:** 4 | **Pass 2 Count:** 3 | **Status:** MATCH (-25%)

#### Migration Guide Summary

Buffered events renamed to "messages" - `World::send_event` becomes `World::write_message`.

#### Required Changes

**1. Update deprecated method in `extras/src/keyboard.rs:494`**
```diff
- world.send_event(event);
+ world.write_message(event);
```

#### Search Pattern

```bash
rg "send_event" --type rust
```

---

## MEDIUM Priority Changes

### 8. Event Split / Rename

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/event_split.md`
**Requirement Level:** MEDIUM
**Occurrences:** 5 locations across 4 files
**Pass 1 Count:** 5 | **Pass 2 Count:** 5 | **Status:** MATCH

#### Migration Guide Summary

Major conceptual rename: buffered events now use `Message` trait with `MessageWriter`/`MessageReader` instead of `Event`/`EventWriter`/`EventReader`.

#### Required Changes

**1. Update EventWriter in `extras/src/keyboard.rs:536`**
```diff
- mut keyboard_events: EventWriter<bevy::input::keyboard::KeyboardInput>,
+ mut keyboard_events: MessageWriter<bevy::input::keyboard::KeyboardInput>,
```

**2. Update EventWriter in `extras/src/shutdown.rs:38`**
```diff
- mut exit: EventWriter<bevy::app::AppExit>,
+ mut exit: MessageWriter<bevy::app::AppExit>,
```

**3. Update EventReader in test examples** (3 files)
```diff
- mut events: EventReader<KeyboardInput>,
+ mut events: MessageReader<KeyboardInput>,
```

#### Search Pattern

```bash
rg "EventWriter|EventReader" --type rust
```

---

### 9. Chromatic Aberration Option

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/chromatic_aberration_option.md`
**Requirement Level:** MEDIUM
**Occurrences:** 2 locations across 1 file
**Pass 1 Count:** 2 | **Pass 2 Count:** 2 | **Status:** MATCH

#### Migration Guide Summary

`ChromaticAberration::color_lut` changed from `Handle<Image>` to `Option<Handle<Image>>`. Using `default()` constructor continues to work without changes.

#### Required Changes

**No immediate changes needed** - current usage of `ChromaticAberration::default()` is compatible.

If custom LUTs are added in the future:
```rust
ChromaticAberration {
    color_lut: Some(custom_lut_handle),
    ..default()
}
```

#### Search Pattern

```bash
rg "ChromaticAberration" --type rust
```

---

## LOW Priority Changes

### 10. Observer and Event Changes

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/observer_and_event_changes.md`
**Requirement Level:** LOW
**Occurrences:** 8 locations across 2 files
**Pass 1 Count:** 74 | **Pass 2 Count:** 8 | **Status:** ANOMALY: -89%

#### Migration Guide Summary

Observer API changes: `Trigger<E>` renamed to `On<E>`. Pass 1 false positives from common method names (Add, Insert, Remove).

#### Required Changes

**1. Update observer in `extras/src/screenshot.rs:59`**
```diff
- .observe(move |trigger: Trigger<ScreenshotCaptured>| {
-     let img = trigger.event().0.clone();
+ .observe(move |screenshot: On<ScreenshotCaptured>| {
+     let img = screenshot.event().0.clone();
```

#### Search Pattern

```bash
rg "Trigger<" --type rust
```

---

### 11. Internal Entities

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/internal_entities.md`
**Requirement Level:** LOW (informational only)
**Occurrences:** 0 locations
**Pass 1 Count:** 10 | **Pass 2 Count:** 0 | **Status:** ANOMALY: -100%

#### Migration Guide Summary

Observers and one-shot systems now marked as `Internal` and hidden from default queries. Pass 1 false positives from unrelated "Internal" string matches.

#### Required Changes

**No action required** - bevy_brp doesn't query for Observer entities or use `World::register_system`.

---

### 12. TextShadow Moved to Widget Text Module

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/textshadow_is_moved_to_widget_text_module.md`
**Requirement Level:** LOW
**Occurrences:** 3 locations across 1 file
**Pass 1 Count:** 3 | **Pass 2 Count:** 3 | **Status:** MATCH

#### Migration Guide Summary

`TextShadow` moved from `bevy::ui` to `bevy::ui::widget::text`. Code using `bevy::prelude::TextShadow` unaffected.

#### Required Changes

**No action required** - current code uses `bevy::prelude::TextShadow` which continues to work.

---

### 13. Window Resolution Constructors

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/window_resolution_constructors.md`
**Requirement Level:** LOW
**Occurrences:** 4 locations across 1 file
**Pass 1 Count:** 4 | **Pass 2 Count:** 4 | **Status:** MATCH

#### Migration Guide Summary

`WindowResolution::new()` now takes `u32` instead of `f32`. Code only references type name, doesn't construct instances.

#### Required Changes

**No action required** - no constructor calls found in codebase.

---

## Guides Not Applicable to This Codebase

The following 102 guides from Bevy 0.17.2 do not apply to this codebase:

- LightVisibilityClass_rename.md
- Newtype_ScrollPosition.md
- RenderTargetInfo_default.md
- UI_scroll_position_is_now_logical.md
- animation_graph_no_more_asset_ids.md
- assets-insert-result.md
- check_change_ticks.md
- clone_behavior_no_longer_eq.md
- combine_soundness_fix.md
- component-lifecycle-module.md
- component_entry.md
- components-registrator-derefmut.md
- composable_specialization.md
- compressed-image-saver.md
- deprecate_iter_entities.md
- dragenter_includes_dragged_entity.md
- dynamic-bundle-movingptr.md
- entities_apis.md
- entity_cloner_builder_split.md
- entity_representation.md
- extract-picking-plugin-members.md
- extract-pointer-input-plugin-members.md
- extract_fn_is_mut.md
- extract_ui_text_colors_per_glyph.md
- extracted_uinodes_z_order.md
- fullscreen_shader_resource.md
- gated_reader.md
- generic-option-parameter.md
- glam-rand-upgrades.md
- gles_optional.md
- gltf-animation-load-optional.md
- handle_weak_replaced_with_handle_uuid.md
- hdr_component.md
- incorrect-type-error-on-run-system-command.md
- interned-labels-cleanup.md
- labeled_asset_scope_errors.md
- log-diagnostics-hash-set.md
- map_set_apply.md
- merge_observerState_observer_single_component.md
- mesh_compute_smooth_normals.md
- non-generic-access.md
- observers_may_not_be_exclusive.md
- overflowclipbox_default_is_now_paddingbox.md
- parallelism_strategy_changes.md
- per-world-error-handler.md
- picking_location_not_component.md
- pointer_target.md
- primitives_non_const_generic_meshable.md
- query_items_borrow_from_query_state.md
- rangefinder.md
- reflect_asset_asset_ids.md
- relationship_set_risky.md
- relative_cursor_position_is_object_centered.md
- remove_archetype_component_id.md
- remove_bundle_register_required_components.md
- remove_cosmic_text_reexports.md
- remove_default_extend_from_iter.md
- remove_deprecated_batch_spawning.md
- remove_scale_value.md
- remove_text_font_from_constructor_methods.md
- remove_the_add_sub_impls_on_volume.md
- removed_components_stores_messages.md
- rename-justifytext.md
- rename_condition.md
- rename_pointer_events.md
- rename_spawn_gltf_material_name.md
- rename_state_scoped.md
- rename_timer_paused_and_finished.md
- rename_transform_compute_matrix.md
- renamed_computednodetarget.md
- render_graph_app_to_ext.md
- render_startup.md
- render_target_info_error.md
- replace_non_send_resources.md
- required_components_rework.md
- rework_merge_mesh_error.md
- rot2_matrix_construction.md
- scalar-field-on-vector-space.md
- scene_spawner_api.md
- schedule_cleanup.md
- separate-border-colors.md
- simple_executor_going_away.md
- spawnable-list-movingptr.md
- specialized_ui_transform.md
- split-window.md
- split_up_computeduitargetcamera.md
- stack_z_offsets_changes.md
- state_scoped_entities_by_default.md
- stop-exposing-minimp3.md
- stop_storing_system_access.md
- sync_cell_utils.md
- system_run_returns_result.md
- system_set_naming_convention.md
- taa_non_experimental.md
- texture_format_pixel_size_returns_result.md
- ui-debug-overlay.md
- unified_system_state_flag.md
- view-transformations.md
- wayland.md
- wgpu_25.md
- zstd.md

---

## Next Steps

1. **Start with REQUIRED changes** (must fix to compile with Bevy 0.17.2)
   - Update all BRP method names (97 occurrences) - **Most critical**
   - Fix render reorganization imports (64 occurrences)
   - Remove `anchor` field from Sprite (1 occurrence)
   - Update Text2d import path (1 occurrence)
   - Update CursorIcon import (1 occurrence)

2. **Address HIGH priority changes** (deprecated features)
   - Optionally clean up reflect registrations (44 occurrences)
   - Update `send_event` to `write_message` (1 occurrence)

3. **Consider MEDIUM priority improvements**
   - EventWriter → MessageWriter migration (5 occurrences)
   - ChromaticAberration field type (informational only)

4. **Review LOW priority informational items**
   - Observer API update (1 occurrence)
   - TextShadow, WindowResolution (no action needed)

5. **Test thoroughly after each category of changes**
   - Run `cargo build && cargo +nightly fmt` after each change
   - Run `cargo nextest run` for test validation
   - Test BRP communication with Bevy 0.17.2 apps

6. **Special considerations**
   - Remember to reload MCP server after code changes (per CLAUDE.md)
   - Update mutation knowledge type paths to match new crate structure
   - Consider renaming `BevyDestroy` → `BevyDespawn` for consistency

---

## Reference

- **Migration guides directory:** /Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides
- **Bevy 0.17.2 release notes:** https://github.com/bevyengine/bevy/releases/tag/v0.17.2
- **Total guides analyzed:** 115
- **Applicable guides:** 13
- **Not applicable guides:** 102
