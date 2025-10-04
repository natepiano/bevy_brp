# Bevy 0.17.2 Migration Plan

**Generated:** 2025-10-04
**Codebase:** /Users/natemccoy/rust/bevy_brp
**Total Applicable Guides:** 11

---

## Summary

- **REQUIRED changes:** 4 guides (177 total occurrences)
- **HIGH priority:** 3 guides (108 total occurrences)
- **MEDIUM priority:** 0 guides (0 total occurrences)
- **LOW priority:** 4 guides (28 total occurrences)

**Count Anomalies:** 0 guides with >20% variance between Pass 1 and Pass 2

**Estimated effort:**
- REQUIRED: Medium (must fix to compile) - 177 occurrences across BRP method names, render imports, observers, and sprite anchors
- HIGH: Small (should fix soon) - 108 occurrences mainly in reflection registration and cursor imports
- MEDIUM: N/A (optional improvements)
- LOW: Small (nice to have) - 28 informational occurrences

---

## ⚠️ Dependency Compatibility Review

**Status:** No bevy-related dependencies found in this project

---

## REQUIRED Changes

## Renamed BRP Methods

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/renamed_BRP_methods.md`

**Requirement Level:** REQUIRED

**Occurrences:** 80 locations across multiple files
**Pass 1 Count:** 80 | **Pass 2 Count:** 80 | **Status:** MATCH

### Migration Guide Summary

Bevy 0.17 renamed most BRP (Bevy Remote Protocol) methods to be more explicit and consistent with engine conventions. The word "destroy" was replaced with "despawn" throughout, and all method names now use a `world.` or `registry.` prefix to indicate the subsystem they operate on.

### Required Changes

**1. Update entity query operations in `mcp/src/brp_tools/tools.rs`**
```diff
-    #[brp_tool(brp_method = "bevy/query")]
+    #[brp_tool(brp_method = "world.query")]
```

**2. Update entity lifecycle operations in `mcp/src/brp_tools/tools.rs`**
```diff
-    #[brp_tool(brp_method = "bevy/spawn")]
+    #[brp_tool(brp_method = "world.spawn_entity")]

-    #[brp_tool(brp_method = "bevy/destroy")]
+    #[brp_tool(brp_method = "world.despawn_entity")]
```

**3. Update entity hierarchy operations in `mcp/src/brp_tools/tools.rs`**
```diff
-    #[brp_tool(brp_method = "bevy/reparent")]
+    #[brp_tool(brp_method = "world.reparent_entities")]
```

**4. Update component operations (entity-level) in `mcp/src/brp_tools/tools.rs`**
```diff
-    #[brp_tool(brp_method = "bevy/get")]
+    #[brp_tool(brp_method = "world.get_components")]

-    #[brp_tool(brp_method = "bevy/insert")]
+    #[brp_tool(brp_method = "world.insert_components")]

-    #[brp_tool(brp_method = "bevy/remove")]
+    #[brp_tool(brp_method = "world.remove_components")]

-    #[brp_tool(brp_method = "bevy/list")]
+    #[brp_tool(brp_method = "world.list_components")]

-    #[brp_tool(brp_method = "bevy/mutate")]
+    #[brp_tool(brp_method = "world.mutate_components")]
```

**5. Update watch operations (component monitoring) in `mcp/src/brp_tools/tools.rs`**
```diff
-    #[brp_tool(brp_method = "bevy/get+watch")]
+    #[brp_tool(brp_method = "world.get_components+watch")]

-    #[brp_tool(brp_method = "bevy/list+watch")]
+    #[brp_tool(brp_method = "world.list_components+watch")]
```

**6. Update resource operations (global state) in `mcp/src/brp_tools/tools.rs`**
```diff
-    #[brp_tool(brp_method = "bevy/get_resource")]
+    #[brp_tool(brp_method = "world.get_resources")]

-    #[brp_tool(brp_method = "bevy/insert_resource")]
+    #[brp_tool(brp_method = "world.insert_resources")]

-    #[brp_tool(brp_method = "bevy/remove_resource")]
+    #[brp_tool(brp_method = "world.remove_resources")]

-    #[brp_tool(brp_method = "bevy/list_resources")]
+    #[brp_tool(brp_method = "world.list_resources")]

-    #[brp_tool(brp_method = "bevy/mutate_resource")]
+    #[brp_tool(brp_method = "world.mutate_resources")]
```

**7. Update registry operations (type schema) in `mcp/src/brp_tools/tools.rs`**
```diff
-    #[brp_tool(brp_method = "bevy/registry/schema")]
+    #[brp_tool(brp_method = "registry.schema")]
```

**8. Update documentation references in all BRP tool files**

Example changes throughout the codebase:
```diff
-//! `bevy/query` tool - Query entities by components
+//! `world.query` tool - Query entities by components

-//! `bevy/spawn` tool - Spawn entities with components
+//! `world.spawn_entity` tool - Spawn entities with components

-//! `bevy/destroy` tool - Destroy entities permanently
+//! `world.despawn_entity` tool - Despawn entities permanently

-/// Parameters for the `bevy/insert` tool
+/// Parameters for the `world.insert_components` tool
```

### Search Pattern

To find all occurrences:
```bash
rg "bevy/(query|spawn|destroy|get|insert|remove|list|mutate|reparent|get_resource|insert_resource|remove_resource|list_resources|mutate_resource)" --type rust
rg "bevy/registry/schema" --type rust
```

---

## `bevy_render` reorganization

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/bevy_render_reorganization.md`

**Requirement Level:** REQUIRED

**Occurrences:** 41 locations across 3 files
**Pass 1 Count:** 41 | **Pass 2 Count:** 41 | **Status:** MATCH

### Migration Guide Summary

This migration involves a major reorganization of Bevy's rendering architecture where types have been moved from `bevy_render` and `bevy_core_pipeline` to new specialized crates: `bevy_camera`, `bevy_shader`, `bevy_light`, `bevy_mesh`, `bevy_image`, `bevy_ui_render`, `bevy_sprite_render`, `bevy_anti_alias`, and `bevy_post_process`.

### Required Changes

**1. Update camera and post-processing imports in `test-app/examples/extras_plugin.rs`**
```diff
-use bevy::core_pipeline::Skybox;
-use bevy::core_pipeline::bloom::Bloom;
-use bevy::core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
-use bevy::core_pipeline::dof::DepthOfField;
-use bevy::core_pipeline::fxaa::Fxaa;
-use bevy::core_pipeline::post_process::ChromaticAberration;
-use bevy::core_pipeline::prepass::MotionVectorPrepass;
+use bevy::camera::{Camera, Camera2d, Camera3d};
+use bevy::camera::visibility::Visibility;
+use bevy::core_pipeline::Skybox;
+use bevy::post_process::{Bloom, ChromaticAberration};
+use bevy::post_process::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
+use bevy::post_process::dof::DepthOfField;
+use bevy::anti_alias::fxaa::Fxaa;
+use bevy::core_pipeline::prepass::MotionVectorPrepass;
```

**2. Update SMAA component usage in `test-app/examples/extras_plugin.rs`**
```diff
-bevy::core_pipeline::smaa::Smaa::default(),
+bevy::anti_alias::smaa::Smaa::default(),
```

**3. Update documentation example in `mcp/src/brp_tools/tools/bevy_registry_schema.rs`**
```diff
-/// Exclude types from these crates (e.g., [`bevy_render`, `bevy_pbr`])
+/// Exclude types from these crates (e.g., [`bevy_camera`, `bevy_pbr`])
```

**4. Update Camera3d type path in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`**
```diff
-"bevy_core_pipeline::core_3d::camera_3d::Camera3d",
+"bevy_camera::core_3d::camera_3d::Camera3d",
```

### Search Pattern

To find all occurrences:
```bash
rg "bevy::core_pipeline::(bloom|fxaa|smaa|post_process)" --type rust
rg "bevy_core_pipeline" --type rust
rg "ChromaticAberration|Camera3d" --type rust
```

---

## Anchor is now a required component on Sprite

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/anchor_is_removed_from_sprite.md`

**Requirement Level:** REQUIRED

**Occurrences:** 16 locations across 3 files
**Pass 1 Count:** 16 | **Pass 2 Count:** 16 | **Status:** MATCH

### Migration Guide Summary

The `anchor` field has been removed from the `Sprite` struct. The `Anchor` component is now a required component when spawning sprites. Additionally, anchor variants have been converted from enum variants to associated constants.

### Required Changes

**1. Remove `anchor` field from `Sprite` struct initialization in `test-app/examples/extras_plugin.rs`**
```diff
 commands.spawn((
     Sprite {
         color: Color::srgb(1.0, 0.5, 0.25),
         custom_size: Some(Vec2::new(64.0, 64.0)),
         flip_x: false,
         flip_y: false,
-        anchor: bevy::sprite::Anchor::Center,
         ..default()
     },
+    bevy::sprite::Anchor::Center,
     Transform::from_xyz(100.0, 100.0, 0.0),
     Name::new("TestSprite"),
 ));
```

**2. Update Anchor variant references to associated constants**

For all anchor usages:
- `Anchor::BottomLeft` → `Anchor::BOTTOM_LEFT`
- `Anchor::BottomCenter` → `Anchor::BOTTOM_CENTER`
- `Anchor::BottomRight` → `Anchor::BOTTOM_RIGHT`
- `Anchor::CenterLeft` → `Anchor::CENTER_LEFT`
- `Anchor::Center` → `Anchor::Center` (unchanged)
- `Anchor::CenterRight` → `Anchor::CENTER_RIGHT`
- `Anchor::TopLeft` → `Anchor::TOP_LEFT`
- `Anchor::TopCenter` → `Anchor::TOP_CENTER`
- `Anchor::TopRight` → `Anchor::TOP_RIGHT`
- `Anchor::Custom(value)` → `Anchor(value)`

### Search Pattern

To find all occurrences:
```bash
rg "Sprite\s*\{" --type rust -A 10
rg "anchor:" --type rust
rg "Anchor::" --type rust
```

---

## Observer / Event API Changes

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/observer_and_event_changes.md`

**Requirement Level:** REQUIRED

**Occurrences:** 1 location
**Pass 1 Count:** 1 | **Pass 2 Count:** 1 | **Status:** MATCH

### Migration Guide Summary

The observer "trigger" API has been significantly refactored in Bevy 0.17.2 to improve clarity and type-safety. The primary change is that `Trigger<T>` has been renamed to `On<T>`, and the event access patterns have changed.

### Required Changes

**1. Update observer callback in `extras/src/screenshot.rs`**
```diff
-.observe(move |trigger: Trigger<ScreenshotCaptured>| {
+.observe(move |screenshot: On<ScreenshotCaptured>| {
     info!("Screenshot captured! Starting async save to: {path_for_observer}");
-    let img = trigger.event().0.clone();
+    let img = screenshot.event().0.clone();
     let path_clone = path_for_observer.clone();
```

### Search Pattern

To find all occurrences:
```bash
rg "Trigger<" --type rust
rg "\.observe\(" --type rust -A 3
```

---

## HIGH Priority Changes

## Bevy 0.17: Automatic Type Registration for Reflection

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/reflect_registration_changes.md`

**Requirement Level:** HIGH

**Occurrences:** 100 locations across 1 file
**Pass 1 Count:** 100 | **Pass 2 Count:** 100 | **Status:** MATCH

### Migration Guide Summary

Bevy 0.17 introduces automatic registration for types implementing `Reflect`, eliminating the need for manual `.register_type()` calls. This is gated by the `reflect_auto_register` feature (enabled by default) or the fallback `reflect_auto_register_static` feature. Generic types still require manual registration.

### Required Changes

**1. Remove all 44 manual `.register_type()` calls in `test-app/examples/extras_plugin.rs`**
```diff
 fn main() {
     App::new()
         .add_plugins(DefaultPlugins)
         .add_plugins(brp_plugin)
         .init_resource::<KeyboardInputHistory>()
         .insert_resource(CurrentPort(port))
-        // Register test resources
-        .register_type::<TestConfigResource>()
-        .register_type::<RuntimeStatsResource>()
-        // Register test components
-        .register_type::<TestStructWithSerDe>()
-        .register_type::<TestStructNoSerDe>()
-        .register_type::<SimpleSetComponent>()
-        .register_type::<TestMapComponent>()
-        .register_type::<TestEnumKeyedMap>()
-        .register_type::<SimpleTestEnum>()
-        .register_type::<TestEnumWithSerDe>()
-        .register_type::<NestedConfigEnum>()
-        .register_type::<SimpleNestedEnum>()
-        .register_type::<OptionTestEnum>()
-        .register_type::<WrapperEnum>()
-        .register_type::<TestVariantChainEnum>()
-        .register_type::<MiddleStruct>()
-        .register_type::<BottomEnum>()
-        .register_type::<TestEnumNoSerDe>()
-        .register_type::<TestArrayField>()
-        .register_type::<TestArrayTransforms>()
-        .register_type::<TestTupleField>()
-        .register_type::<TestTupleStruct>()
-        .register_type::<TestComplexTuple>()
-        .register_type::<TestComplexComponent>()
-        .register_type::<TestCollectionComponent>()
-        .register_type::<TestMixedMutabilityCore>()
-        .register_type::<TestMixedMutabilityVec>()
-        .register_type::<TestMixedMutabilityArray>()
-        .register_type::<TestMixedMutabilityTuple>()
-        .register_type::<TestMixedMutabilityEnum>()
-        .register_type::<TestPartiallyMutableNested>()
-        .register_type::<TestDeeplyNested>()
-        // Register gamepad types for BRP access
-        .register_type::<Gamepad>()
-        .register_type::<GamepadSettings>()
-        // Register Screenshot type for BRP access
-        .register_type::<Screenshot>()
-        // Register missing components for BRP access
-        .register_type::<MotionVectorPrepass>()
-        .register_type::<NotShadowCaster>()
-        .register_type::<NotShadowReceiver>()
-        .register_type::<VolumetricLight>()
-        .register_type::<OcclusionCulling>()
-        .register_type::<NoFrustumCulling>()
-        .register_type::<CalculatedClip>()
-        .register_type::<Button>()
-        .register_type::<Label>()
-        .register_type::<BorderRadius>()
+        // Types with #[derive(Reflect)] are now automatically registered
+        // via the reflect_auto_register feature (enabled by default)
         .add_systems(Startup, (setup_test_entities, setup_ui))
         .run();
 }
```

**2. Keep all `#[derive(Reflect)]` and `#[reflect(...)]` attributes - no changes needed**

Type definitions remain unchanged - automatic registration works with existing `#[derive(Reflect)]` attributes.

### Search Pattern

To find all occurrences:
```bash
rg "\.register_type::<" --type rust
rg "#\[derive.*Reflect.*\]" --type rust
```

---

## Move cursor-related types from `bevy_winit` to `bevy_window`

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/cursor-android.md`

**Requirement Level:** HIGH

**Occurrences:** 5 locations across 1 file
**Pass 1 Count:** 5 | **Pass 2 Count:** 5 | **Status:** MATCH

### Migration Guide Summary

Cursor-related types have been moved from `bevy_winit` to `bevy_window` to reduce dependencies. The types `CursorIcon`, `CustomCursor`, `CustomCursorImage`, and `CustomCursorUrl` now reside in `bevy::window` instead of `bevy_winit`.

### Required Changes

**1. Update CursorIcon import in `test-app/examples/extras_plugin.rs`**
```diff
- use bevy_winit::cursor::CursorIcon;
+ use bevy::window::CursorIcon;
```

All existing usage of `CursorIcon` remains unchanged - only the import path needs updating.

### Search Pattern

To find all occurrences:
```bash
rg "bevy_winit.*cursor" --type rust
rg "CursorIcon" --type rust
```

---

## Rename `send_event` and similar methods to `write_message`

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/send_event_rename.md`

**Requirement Level:** HIGH

**Occurrences:** 3 locations across 1 file
**Pass 1 Count:** 3 | **Pass 2 Count:** 3 | **Status:** MATCH

### Migration Guide Summary

The guide renames event-related methods to message-related methods, following the 0.16 change where `EventWriter::send` became `EventWriter::write`. The term "buffered events" is now called "Messages". This affects methods on `World`, `DeferredWorld`, `Commands`, `Events` (now `Messages`), and related types.

### Required Changes

**1. Replace `World::send_event` with `World::write_message` in `extras/src/keyboard.rs`**
```diff
-    for event in press_events {
-        world.send_event(event);
-    }
+    for event in press_events {
+        world.write_message(event);
+    }
```

The two `EventWriter` occurrences are already using the post-0.16 API (where `send` was renamed to `write`), so they don't require changes.

### Search Pattern

To find all occurrences:
```bash
rg "send_event" --type rust
rg "EventWriter" --type rust
```

---

## MEDIUM Priority Changes

No MEDIUM priority changes identified.

---

## LOW Priority Changes

## Updated `glam`, `rand` and `getrandom` versions with new failures when building for web

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/glam-rand-upgrades.md`

**Requirement Level:** LOW

**Occurrences:** 17 locations (all string constants)
**Pass 1 Count:** 17 | **Pass 2 Count:** 17 | **Status:** MATCH

### Migration Guide Summary

This migration guide covers dependency version upgrades for glam, rand, and getrandom. The codebase uses glam v0.29.3 (already compatible) and has no direct usage of rand APIs. All 16 glam occurrences are string constants defining type names for BRP type system introspection, not runtime API usage.

### Required Changes

No code changes required. The codebase:
- Already uses compatible glam version (v0.29.3)
- Has no direct rand API usage (dependency is transitive through Bevy)
- Has no WASM build targets requiring getrandom configuration
- All glam references are type name constants used for reflection

### Search Pattern

To verify:
```bash
rg "glam::" --type rust
rg "thread_rng|from_entropy" --type rust
```

---

## Text2d moved to bevy_sprite

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/text2d_moved_to_bevy_sprite.md`

**Requirement Level:** LOW

**Occurrences:** 7 locations across 1 file
**Pass 1 Count:** 7 | **Pass 2 Count:** 7 | **Status:** MATCH

### Migration Guide Summary

The world-space text types `Text2d` and `Text2dShadow` have been moved to the `bevy_sprite` crate. This codebase uses `Text2d` in test code.

### Required Changes

**1. Update Text2d import in `test-app/examples/extras_plugin.rs`**
```diff
-    use bevy::text::Text2d;
+    use bevy::sprite::Text2d;
```

Usage remains unchanged - only the import path needs updating.

### Search Pattern

To find all occurrences:
```bash
rg "bevy::text::Text2d" --type rust
rg "Text2d" --type rust
```

---

## Event trait split / Rename

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/event_split.md`

**Requirement Level:** LOW

**Occurrences:** 11 locations (informational)
**Pass 1 Count:** 11 | **Pass 2 Count:** 11 | **Status:** MATCH

### Migration Guide Summary

This migration guide describes a terminology change where "buffered events" (sent/read via `EventWriter`/`EventReader`) are now called "messages" using the `Message` trait. However, the actual type names `EventWriter` and `EventReader` remain unchanged in 0.17.2 - they are deprecated but still functional.

### Required Changes

No immediate changes required. The guide describes a conceptual split between:
- **EventWriter/EventReader** (still valid, for buffered message passing)
- **Message trait** (new, for types used with writers/readers)
- **Event trait** (now exclusively for observer patterns)

The codebase currently uses `EventWriter` (2 occurrences) and `EventReader` (3 occurrences) which continue to work. Migration to `MessageWriter`/`MessageReader` can be deferred until these types are fully removed.

### Search Pattern

To find all occurrences:
```bash
rg "EventWriter|EventReader" --type rust
rg "MessageWriter|MessageReader" --type rust
```

---

## ChromaticAberration LUT is now Option

**Guide File:** `/Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides/chromatic_aberration_option.md`

**Requirement Level:** LOW

**Occurrences:** 2 locations (no changes needed)
**Pass 1 Count:** 2 | **Pass 2 Count:** 2 | **Status:** MATCH

### Migration Guide Summary

The `ChromaticAberration` component's `color_lut` field changed from `Handle<Image>` to `Option<Handle<Image>>`. The field now accepts `None` to use the default LUT, while custom LUTs should be wrapped in `Some()`.

### Required Changes

No migration needed. The codebase only uses `ChromaticAberration::default()` for testing mutations in the test-app example. There are no custom `color_lut` assignments that would require wrapping in `Some()`.

### Search Pattern

To find all occurrences:
```bash
rg "ChromaticAberration" --type rust
rg "color_lut" --type rust
```

---

## Guides Not Applicable to This Codebase

The following 104 guides from Bevy 0.17.2 do not apply to this codebase:

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
- release-content/migration-guides/observers_may_not_be_exclusive.md
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
- release-content/migration-guides/remove_text_font_from_constructor_methods.md
- release-content/migration-guides/remove_the_add_sub_impls_on_volume.md
- release-content/migration-guides/removed_components_stores_messages.md
- release-content/migration-guides/rename-justifytext.md
- release-content/migration-guides/rename_condition.md
- release-content/migration-guides/rename_pointer_events.md
- release-content/migration-guides/rename_spawn_gltf_material_name.md
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
- release-content/migration-guides/rot2_matrix_construction.md
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
- release-content/migration-guides/taa_non_experimental.md
- release-content/migration-guides/textshadow_is_moved_to_widget_text_module.md
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

1. Start with REQUIRED changes (must fix to compile with Bevy 0.17.2)
   - Update all BRP method names (80 occurrences)
   - Fix bevy_render reorganization imports (41 occurrences)
   - Remove anchor field from Sprite and add as component (16 occurrences)
   - Update observer Trigger to On (1 occurrence)

2. Address HIGH priority changes (deprecated features)
   - Remove .register_type() calls (44 occurrences) - auto-registration handles this
   - Update CursorIcon import path (5 occurrences)
   - Rename send_event to write_message (3 occurrences)

3. Consider LOW priority improvements
   - Update Text2d import (informational)
   - Review EventWriter/EventReader for future MessageWriter/MessageReader migration

4. Test thoroughly after each category of changes

5. Run `cargo check` and `cargo nextest run` frequently

---

## Reference

- **Migration guides directory:** /Users/natemccoy/rust/bevy-0.17.2/release-content/migration-guides
- **Bevy 0.17.2 release notes:** https://github.com/bevyengine/bevy/releases/tag/v0.17.2
