# Initialize Type Validation Tracking File

## Purpose
This command initializes or reinitializes the type validation tracking file (`test-app/examples/type_validation.json`) by:
1. Launching the extras_plugin example app
2. Getting the list of all registered component types via BRP
3. Creating a fresh tracking file with all types marked as "untested"
4. Discovering spawn support and mutation paths for ALL types systematically

## Usage
```
/init_type_validation
```

This will overwrite any existing `type_validation.json` file with a fresh one containing all currently registered types ready for testing.

## Execution Steps

### 1. Launch the extras_plugin app
```bash
mcp__brp__brp_launch_bevy_example(
    example_name="extras_plugin",
    port=22222
)
```

### 2. Verify BRP connectivity
```bash
mcp__brp__brp_status(
    app_name="extras_plugin",
    port=22222
)
```

Wait for confirmation that BRP is responding before proceeding.

### 3. Get list of all component types
```bash
result = mcp__brp__bevy_list(port=22222)
```

This returns an array of all registered component type names.

### 4. Call brp_type_schema with filtered types
**IMPORTANT: DO NOT create any intermediate files, Python scripts, or helper files. Call brp_type_schema directly with the filtered list.**

Call `mcp__brp__brp_type_schema` with ALL types EXCEPT these problematic ones:
- `bevy_ecs::hierarchy::ChildOf`
- `bevy_ecs::hierarchy::Children`
- `bevy_core_pipeline::core_2d::camera_2d::Camera2d`
- `bevy_core_pipeline::prepass::DeferredPrepass`
- `bevy_core_pipeline::prepass::DepthPrepass`
- `bevy_core_pipeline::prepass::MotionVectorPrepass`
- `bevy_core_pipeline::prepass::NormalPrepass`
- `bevy_pbr::light::NotShadowCaster`
- `bevy_pbr::light::NotShadowReceiver`
- `bevy_pbr::volumetric_fog::VolumetricLight`
- `bevy_render::experimental::occlusion_culling::OcclusionCulling`
- `bevy_render::camera::manual_texture_view::ManualTextureViewHandle`
- `bevy_render::camera::projection::Projection`
- `bevy_pbr::mesh_material::MeshMaterial3d<bevy_pbr::extended_material::ExtendedMaterial<bevy_pbr::pbr_material::StandardMaterial, bevy_pbr::decal::forward::ForwardDecalMaterialExt>>`
- `bevy_pbr::mesh_material::MeshMaterial3d<bevy_pbr::pbr_material::StandardMaterial>`
- `bevy_sprite::mesh2d::material::MeshMaterial2d<bevy_sprite::mesh2d::color_material::ColorMaterial>`
- `bevy_pbr::components::CascadesVisibleEntities`
- `bevy_pbr::components::CubemapVisibleEntities`
- `bevy_pbr::components::VisibleMeshEntities`
- `bevy_pbr::light_probe::LightProbe`
- `bevy_render::primitives::CascadesFrusta`
- `bevy_render::primitives::CubemapFrusta`
- `bevy_render::primitives::Frustum`
- `bevy_render::sync_world::SyncToRenderWorld`
- `bevy_render::view::visibility::NoFrustumCulling`
- `bevy_render::view::visibility::VisibleEntities`
- `bevy_ui::measurement::ContentSize`
- `bevy_ui::widget::button::Button`
- `bevy_ui::widget::label::Label`
- `bevy_window::window::PrimaryWindow`
- `bevy_render::camera::camera::CameraMainTextureUsages`
- `bevy_render::camera::camera::CameraRenderGraph`
- `bevy_render::camera::camera::Exposure`

Build the types array directly in the tool call by filtering the result from step 3.
The tool will automatically save its result to a file and return the filepath (e.g., `/var/folders/.../mcp_response_brp_type_schema_12345.json`).

### 5. Transform the result with jq
**CRITICAL: This is a REAL bash command to execute, NOT pseudocode. Use the actual filepath returned from step 4.**

Execute this exact jq command using the Bash tool, replacing `FILEPATH` with the actual path from step 4:

```bash
jq '
.type_info | to_entries | [.[] | . as $item | .key as $idx | {
  type: .value.type_name,
  spawn_support: (if (.value.supported_operations // []) | contains(["spawn", "insert"]) then "supported" else "not_supported" end),
  mutation_paths: ((.value.mutation_paths // {}) | keys),
  test_status: "untested",
  batch_number: null,
  fail_reason: ""
}]' FILEPATH > test-app/tests/type_validation.json
```

**Note:** All types are initialized with `batch_number: null` - batch assignment will be done separately.

This command directly creates `test-app/tests/type_validation.json` from the BRP response.

### 6. Verify final file structure
The completed file should have:
- All types with spawn support properly identified (`"supported"` or `"not_supported"`)
- All types with mutation paths listed as arrays
- All types starting with `test_status: "untested"`
- All types starting with `batch_number: null` (batch assignment done separately)

Types that support spawn typically have:
- `has_deserialize: true` and `has_serialize: true` in the BRP response
- A `spawn_format` field in the BRP response
- `["query", "get", "mutate", "spawn", "insert"]` in supported_operations

### 7. Report results
```bash
# Generate summary statistics
echo "✅ Initialized type validation tracking file with complete capability discovery"
jq -r '
  "- Total types: " + (. | length | tostring) + "\n" +
  "- Spawn-supported types: " + ([.[] | select(.spawn_support == "supported")] | length | tostring) + "\n" +
  "- Types with mutations: " + ([.[] | select(.mutation_paths | length > 0)] | length | tostring) + "\n" +
  "- Types with no capabilities: " + ([.[] | select(.spawn_support == "not_supported" and (.mutation_paths | length == 0))] | length | tostring)
' test-app/tests/type_validation.json
```

### 9. Cleanup
Shutdown the app:
```bash
mcp__brp__brp_shutdown(
    app_name="extras_plugin", 
    port=22222
)
```

## Critical Success Factors

1. **NO intermediate files** - Do NOT create Python scripts, temp files, or any other files
2. **Direct tool usage only** - Use only MCP tools and the single jq command shown
3. **Single output file** - Only create/modify `test-app/tests/type_validation.json`
4. **Use actual BRP responses** - Base spawn support and mutation paths on actual BRP discovery
5. **Execute jq command exactly** - The jq command in step 5 is a REAL command to execute with Bash, not pseudocode

## Expected Results

- Spawn-supported types: Types with Serialize/Deserialize traits (Name, Transform, Node, Window, BackgroundColor, test components, etc.)
- Non-spawn types: Most rendering/internal components (Sprite, Camera components, visibility components, etc.)  
- All types should have their actual mutation paths populated as arrays
- All types start with `test_status: "untested"` and empty `fail_reason`