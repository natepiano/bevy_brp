# Bevy 0.18.0-rc.1 Migration Plan

**Generated:** 2025-12-19
**Codebase:** /home/natemccoy/rust/bevy_brp
**Total Applicable Guides:** 4

---

## EXECUTION PROTOCOL

<Instructions>
For each step in the implementation sequence:

1. **DESCRIBE**: Present the changes with:
   - Summary of what will change and why
   - Code examples showing before/after
   - List of files to be modified
   - Expected impact on the system

2. **AWAIT APPROVAL**: Stop and wait for user confirmation ("go ahead" or similar)

3. **IMPLEMENT**: Make the changes and stop

4. **BUILD & VALIDATE**: Execute the build process:
   ```bash
   cargo check -p bevy_brp_mcp
   cargo nextest run
   ```

5. **CONFIRM**: Wait for user to confirm the build succeeded

6. **MARK COMPLETE**: Update this document to mark the step as ✅ COMPLETED

7. **PROCEED**: Move to next step only after confirmation
</Instructions>

<ExecuteImplementation>
    Find the next ⏳ PENDING step in the INTERACTIVE IMPLEMENTATION SEQUENCE below.

    For the current step:
    1. Follow the <Instructions/> above for executing the step
    2. When step is complete, use Edit tool to mark it as ✅ COMPLETED
    3. Continue to next PENDING step

    If all steps are COMPLETED:
        Display: "✅ Implementation complete! All steps have been executed."
</ExecuteImplementation>

---

## INTERACTIVE IMPLEMENTATION SEQUENCE

### Step 1: Update Cargo.toml Dependencies ⏳ PENDING

**Objective:** Update Bevy version from 0.17.x to 0.18.0-rc.1 in all Cargo.toml files

**Files to modify:**
- `Cargo.toml` (workspace)
- `extras/Cargo.toml`

**Changes:**
```diff
# Cargo.toml (workspace)
[workspace.dependencies]
- bevy = { version = "0.17.3", default-features = false }
- bevy_mesh = { version = "0.17.3" }
- bevy_winit = { version = "0.17.3" }
+ bevy = { version = "0.18.0-rc.1", default-features = false }
+ bevy_mesh = { version = "0.18.0-rc.1" }
+ bevy_winit = { version = "0.18.0-rc.1" }

# extras/Cargo.toml
[dependencies]
- bevy = { version = "0.17.2", features = [
+ bevy = { version = "0.18.0-rc.1", features = [
```

**Build command:** `cargo check` (expect compilation failures until code changes are complete)

**Notes:** This step will cause build failures. Continue to next steps to fix code.

---

### Step 2: Fix AnimationTarget Split ⏳ PENDING

**Objective:** Update AnimationTarget usage to new AnimationTargetId + AnimatedBy components

**Files to modify:**
- `test-app/examples/extras_plugin.rs`

**Changes:**

**2a. Update import (line 20):**
```diff
- use bevy::animation::AnimationTarget;
+ use bevy::animation::AnimationTargetId;
+ use bevy::animation::AnimatedBy;
```

**2b. Update component usage (lines 1217-1224):**
```diff
-     // Entity with AnimationTarget for testing mutations
+     // Entity with AnimationTargetId and AnimatedBy for testing mutations
      commands.spawn((
-         AnimationTarget {
-             id:     bevy::animation::AnimationTargetId::from_name(&Name::new("test_target")),
-             player: Entity::PLACEHOLDER,
-         },
-         Name::new("AnimationTargetTestEntity"),
+         AnimationTargetId::from_name(&Name::new("test_target")),
+         AnimatedBy(Entity::PLACEHOLDER),
+         Name::new("AnimationTargetTestEntity"),
      ));
```

**Build command:** N/A (wait for all code changes)

---

### Step 3: Fix BorderRadius to Node Field ⏳ PENDING

**Objective:** Move BorderRadius from standalone component to Node.border_radius field

**Files to modify:**
- `test-app/examples/extras_plugin.rs`

**Changes (lines 690-694):**
```diff
    // Entity with BorderRadius for testing mutations
    commands.spawn((
-       BorderRadius::all(Val::Px(10.0)),
+       Node {
+           border_radius: BorderRadius::all(Val::Px(10.0)),
+           ..default()
+       },
        Name::new("BorderRadiusTestEntity"),
    ));
```

**Build command:** N/A (wait for all code changes)

---

### Step 4: Fix reinterpret_stacked_2d_as_array Result ⏳ PENDING

**Objective:** Handle new Result return type from Image::reinterpret_stacked_2d_as_array

**Files to modify:**
- `test-app/examples/extras_plugin.rs`

**Changes (line 556):**
```diff
    // Reinterpret as cube texture (height/width = 6)
-    image.reinterpret_stacked_2d_as_array(image.height() / image.width());
+    image.reinterpret_stacked_2d_as_array(image.height() / image.width()).expect("Failed to reinterpret image as cube texture array");
```

**Build command:**
```bash
cargo check -p bevy_brp_mcp
cargo check -p bevy_brp_extras
cargo check -p test-app
cargo nextest run
```

---

### Step 5: Update CHANGELOGs ⏳ PENDING

**Objective:** Document the migration in all 3 crate CHANGELOGs

**Files to modify:**
- `mcp/CHANGELOG.md`
- `mcp_macros/CHANGELOG.md`
- `extras/CHANGELOG.md`

**Changes:**

**mcp/CHANGELOG.md:**
```markdown
### Changed

- **Upgraded to Bevy 0.18.0**: Updated bevy dependency from 0.17.x to 0.18.0
```

**mcp_macros/CHANGELOG.md:**
```markdown
### Changed

- **Upgraded to Bevy 0.18.0**: Updated bevy dependency from 0.17.x to 0.18.0
```

**extras/CHANGELOG.md:**
```markdown
### Changed

- **Upgraded to Bevy 0.18.0**: Updated bevy dependency from 0.17.x to 0.18.0
  - `BorderRadius` now set via `Node.border_radius` field instead of standalone component
  - `AnimationTarget` split into `AnimationTargetId` + `AnimatedBy` components
  - `Image::reinterpret_stacked_2d_as_array` now returns `Result`
```

**Build command:** N/A (documentation only)

---

### Step 6: Final Validation ⏳ PENDING

**Objective:** Verify complete migration success

**Validation checklist:**
- [ ] `cargo check` passes for all crates
- [ ] `cargo nextest run` passes all tests
- [ ] No deprecation warnings related to Bevy 0.17 APIs

**Build command:**
```bash
cargo check --workspace
cargo nextest run --workspace
```

---

## Summary

- **REQUIRED changes:** 3 guides (7 total occurrences)
- **HIGH priority:** 0 guides (0 total occurrences)
- **MEDIUM priority:** 0 guides (0 total occurrences)
- **LOW priority:** 1 guide (3 total occurrences)

**Count Anomalies:** 0 guides with >20% variance between Pass 1 and Pass 2

**Estimated effort:**
- REQUIRED: Small (must fix to compile)
- HIGH: N/A
- MEDIUM: N/A
- LOW: Small (no changes needed - informational only)

---

## Dependency Compatibility Review

**Status:** No bevy-related dependencies found in this project

---

## REQUIRED Changes Detail

### BorderRadius is now a field on Node

**Guide File:** `/home/natemccoy/rust/bevy-0.18.0-rc.1/release-content/migration-guides/border_radius_is_now_a_field_on_node.md`
**Requirement Level:** REQUIRED
**Occurrences:** 3 locations across 1 file
**Pass 1 Count:** 15 | **Pass 2 Count:** 15 | **Status:** MATCH: 0%

#### Migration Guide Summary

`BorderRadius` is no longer a standalone component in Bevy 0.18. Instead, it has been moved to be a field (`border_radius: BorderRadius`) on the `Node` component. Any code that spawns `BorderRadius` as a separate component must be refactored to set it as a field within the `Node` struct.

#### Required Changes

**1. Update BorderRadius usage in `test-app/examples/extras_plugin.rs`**

The entity spawns `BorderRadius` as a separate component without an associated `Node`. This needs to be changed to spawn a `Node` with the `border_radius` field set:

```diff
    // Entity with BorderRadius for testing mutations
    commands.spawn((
-       BorderRadius::all(Val::Px(10.0)),
+       Node {
+           border_radius: BorderRadius::all(Val::Px(10.0)),
+           ..default()
+       },
        Name::new("BorderRadiusTestEntity"),
    ));
```

**Note:** The 12 occurrences of `Node` in the codebase are existing `Node` component usages that do not currently use `BorderRadius`. No changes are required for those unless you want to add border radius styling to them.

#### Search Pattern

To find all occurrences of `BorderRadius` being used as a component:
```bash
rg "BorderRadius" --type rust
```

---

### `AnimationTarget` replaced by separate components

**Guide File:** `/home/natemccoy/rust/bevy-0.18.0-rc.1/release-content/migration-guides/animation-target-refactor.md`
**Requirement Level:** REQUIRED
**Occurrences:** 5 locations across 1 file
**Pass 1 Count:** 6 | **Pass 2 Count:** 6 | **Status:** MATCH: 0%

#### Migration Guide Summary

The `AnimationTarget` component has been split into two separate components for greater flexibility. The `AnimationTarget::id` field is now an `AnimationTargetId` component, and `AnimationTarget::player` is now an `AnimatedBy` component. This allows calculating the animation target ID first while deferring the player choice until later.

#### Required Changes

**1. Update import in `test-app/examples/extras_plugin.rs` (line 20)**
```diff
- use bevy::animation::AnimationTarget;
+ use bevy::animation::AnimationTargetId;
+ use bevy::animation::AnimatedBy;
```

**2. Update AnimationTarget usage in `test-app/examples/extras_plugin.rs` (lines 1217-1224)**

The comment and component usage need to be updated:
```diff
-     // Entity with AnimationTarget for testing mutations
+     // Entity with AnimationTargetId and AnimatedBy for testing mutations
      commands.spawn((
-         AnimationTarget {
-             id:     bevy::animation::AnimationTargetId::from_name(&Name::new("test_target")),
-             player: Entity::PLACEHOLDER,
-         },
-         Name::new("AnimationTargetTestEntity"),
+         AnimationTargetId::from_name(&Name::new("test_target")),
+         AnimatedBy(Entity::PLACEHOLDER),
+         Name::new("AnimationTargetTestEntity"),
      ));
```

#### Search Pattern

To find all occurrences:
```bash
rg "AnimationTarget" --type rust
```

---

### Image::reinterpret_size and Image::reinterpret_stacked_2d_as_array now return a Result

**Guide File:** `/home/natemccoy/rust/bevy-0.18.0-rc.1/release-content/migration-guides/image_reinterpret_returns_result.md`
**Requirement Level:** REQUIRED
**Occurrences:** 1 location across 1 file
**Pass 1 Count:** 1 | **Pass 2 Count:** 1 | **Status:** MATCH: 0%

#### Migration Guide Summary

`Image::reinterpret_size` and `Image::reinterpret_stacked_2d_as_array` now return a `Result` instead of panicking. Previously, calling these methods on image assets that did not conform to certain constraints could lead to runtime panics. The new return type makes the API safer and more explicit about the constraints, requiring callers to handle the `Result`.

#### Required Changes

**1. Update `reinterpret_stacked_2d_as_array` call in `test-app/examples/extras_plugin.rs`**

The call at line 556 ignores the return value. Since the method now returns a `Result`, this needs to handle the result appropriately (unwrap, expect, or proper error handling).

```diff
    // Reinterpret as cube texture (height/width = 6)
-    image.reinterpret_stacked_2d_as_array(image.height() / image.width());
+    image.reinterpret_stacked_2d_as_array(image.height() / image.width()).expect("Failed to reinterpret image as cube texture array");
```

#### Search Pattern

To find all occurrences:
```bash
rg "reinterpret_stacked_2d_as_array|reinterpret_size" --type rust
```

---

## LOW Priority Changes

### `AmbientLight` split into a component and a resource

**Guide File:** `/home/natemccoy/rust/bevy-0.18.0-rc.1/release-content/migration-guides/ambient_light_split.md`
**Requirement Level:** LOW
**Occurrences:** 3 locations across 1 file
**Pass 1 Count:** 3 | **Pass 2 Count:** 3 | **Status:** MATCH: 0%

#### Migration Guide Summary

In Bevy 0.18, `AmbientLight` has been split into two separate structs: `AmbientLight` (a component that can be added to a `Camera` to override the default) and `GlobalAmbientLight` (a resource for the entire world, automatically added by `LightPlugin`). When using `AmbientLight` as a resource via `insert_resource`, it should be renamed to `GlobalAmbientLight`.

#### Required Changes

**1. This codebase uses `AmbientLight` correctly as a component on a Camera entity - NO CHANGE REQUIRED**

The occurrences in `test-app/examples/extras_plugin.rs` use `AmbientLight` as a component attached to a Camera entity, which is the correct usage in Bevy 0.18. The migration guide indicates that `AmbientLight` should only be renamed to `GlobalAmbientLight` when used as a resource via `insert_resource()`. Since this codebase uses it as a component (spawned with a Camera3d), no changes are needed.

The current usage:
```rust
commands.spawn((
    Camera3d::default(),
    Camera {
        is_active: false, // Disable this test camera to avoid rendering
        ..default()
    },
    AmbientLight::default(),  // Correct: component on a Camera entity
    Msaa::default(),
    Transform::from_xyz(100.0, 100.0, 100.0),
    Name::new("AmbientLightTestEntity"),
));
```

This is the intended usage of `AmbientLight` as a camera-specific override component in Bevy 0.18.

#### Search Pattern

To find all occurrences:
```bash
rg "AmbientLight" --type rust
```

---

## Guides Not Applicable to This Codebase

The following 58 guides from Bevy 0.18.0-rc.1 do not apply to this codebase:

- release-content/migration-guides/BorderRects_fields_are_now_vec2s.md
- release-content/migration-guides/animation-event-trigger-rename.md
- release-content/migration-guides/archetype_query_data.md
- release-content/migration-guides/asset_plugin_processed_override.md
- release-content/migration-guides/asset_watcher_async_sender.md
- release-content/migration-guides/bevy_input_features.md
- release-content/migration-guides/bevy_manifest_scope_api.md
- release-content/migration-guides/bind-group-layout-descriptors.md
- release-content/migration-guides/bindgroup-labels-mandatory.md
- release-content/migration-guides/bundle_component_ids.md
- release-content/migration-guides/cargo_feature_collections.md
- release-content/migration-guides/change_detection_refactors.md
- release-content/migration-guides/changed_asset_server_init.md
- release-content/migration-guides/combinator_system.md
- release-content/migration-guides/custom_asset_source_infallible.md
- release-content/migration-guides/derive_compile_error_for_non_static_resource.md
- release-content/migration-guides/dragenter-now-fires-on-drag-starts.md
- release-content/migration-guides/draw_functions.md
- release-content/migration-guides/dynamic_relationships_api.md
- release-content/migration-guides/enable_prepass.md
- release-content/migration-guides/entities_apis.md
- release-content/migration-guides/extracted_uinodes_z_order.md
- release-content/migration-guides/feature-cleanup.md
- release-content/migration-guides/function_system_generics.md
- release-content/migration-guides/generalized_atmosphere.md
- release-content/migration-guides/get_components.md
- release-content/migration-guides/get_many_renamed_to_get_disjoint.md
- release-content/migration-guides/gizmos-cuboid.md
- release-content/migration-guides/gizmos-render.md
- release-content/migration-guides/gltf-coordinate-conversion.md
- release-content/migration-guides/image_loader_array_layout.md
- release-content/migration-guides/image_render_target_scale_factor_is_now_f32.md
- release-content/migration-guides/immutable-entity-events.md
- release-content/migration-guides/internal_disabling_component_removed.md
- release-content/migration-guides/lineheight_is_now_a_separate_component.md
- release-content/migration-guides/load_context_asset_path.md
- release-content/migration-guides/process_trait_changes.md
- release-content/migration-guides/reader_required_features.md
- release-content/migration-guides/readers_impl_async_seek.md
- release-content/migration-guides/reflect_parentheses.md
- release-content/migration-guides/remove-dummy-white-gpu-image.md
- release-content/migration-guides/remove_dangling_with_align.md
- release-content/migration-guides/remove_ron_reexport.md
- release-content/migration-guides/removed-font-atlas-sets.md
- release-content/migration-guides/removed_simple_executor.md
- release-content/migration-guides/rename-clear_children.md
- release-content/migration-guides/rename-reflect-documentation-feature.md
- release-content/migration-guides/rename_thin_column.md
- release-content/migration-guides/render_target_component.md
- release-content/migration-guides/same_state_transitions.md
- release-content/migration-guides/schedule_cleanup.md
- release-content/migration-guides/set_index_buffer.md
- release-content/migration-guides/text_layout_info_section_rects_is_replaced_by_run_geometry.md
- release-content/migration-guides/the_non-text_areas_of_text_nodes_are_no_longer_pickable.md
- release-content/migration-guides/thin_slice_ptr_get_unchecked.md
- release-content/migration-guides/tilemap_chunk_layout_change.md
- release-content/migration-guides/type_path_for_asset_traits.md
- release-content/migration-guides/winit_user_events_removed.md

---

## Update CHANGELOGs

After completing the migration, update the CHANGELOG.md for each crate:

### mcp/CHANGELOG.md

```markdown
### Changed

- **Upgraded to Bevy 0.18.0**: Updated bevy dependency from 0.17.x to 0.18.0
```

### mcp_macros/CHANGELOG.md

```markdown
### Changed

- **Upgraded to Bevy 0.18.0**: Updated bevy dependency from 0.17.x to 0.18.0
```

### extras/CHANGELOG.md

```markdown
### Changed

- **Upgraded to Bevy 0.18.0**: Updated bevy dependency from 0.17.x to 0.18.0
  - `BorderRadius` now set via `Node.border_radius` field instead of standalone component
  - `AnimationTarget` split into `AnimationTargetId` + `AnimatedBy` components
  - `Image::reinterpret_stacked_2d_as_array` now returns `Result`
```

---

## Reference

- **Migration guides directory:** /home/natemccoy/rust/bevy-0.18.0-rc.1/release-content/migration-guides
- **Bevy 0.18.0-rc.1 release notes:** https://github.com/bevyengine/bevy/releases/tag/v0.18.0-rc.1
