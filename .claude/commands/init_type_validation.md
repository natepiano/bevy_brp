# Initialize Type Validation Tracking File

## Purpose
This command initializes or reinitializes the type validation tracking file (`test-app/examples/type_validation.json`) by:
1. Launching the extras_plugin example app
2. Getting the list of all registered component types via BRP
3. Creating a fresh tracking file with all types marked as "untested"

## Usage
```
/init_type_validation
```

This will overwrite any existing `type_validation.json` file with a fresh one containing all currently registered types.

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

### 4. Create the tracking file
**IMPORTANT: Use a bash command with jq to create the file quickly and reliably.**

After getting the component list from step 3, create the tracking file using this bash command:

**Note on excluded types:**
- `ChildOf`/`Children`: Hierarchy components that cause BRP issues
- `Camera2d`: Has serialization problems with BRP  
- Prepass components: Internal rendering components not meant for direct testing
- `NotShadowCaster`/`NotShadowReceiver`/`VolumetricLight`/`OcclusionCulling`: Unit struct marker components with no spawn support and no mutation paths
- `MeshMaterial3d` types: Asset handle serialization issues - BRP cannot serialize `Arc<StrongHandle>` (missing `ReflectSerialize` registration)

```bash
# Extract the component list from result["result"] and format it as JSON array
# Filter out excluded types, then transform each type into the tracking structure
echo '[
    "component_type_1",
    "component_type_2",
    # ... all component types from result["result"] ...
]' | jq '
  map(select(
    . != "bevy_ecs::hierarchy::ChildOf" and 
    . != "bevy_ecs::hierarchy::Children" and
    . != "bevy_core_pipeline::core_2d::camera_2d::Camera2d" and
    . != "bevy_core_pipeline::prepass::DeferredPrepass" and
    . != "bevy_core_pipeline::prepass::DepthPrepass" and
    . != "bevy_core_pipeline::prepass::MotionVectorPrepass" and
    . != "bevy_core_pipeline::prepass::NormalPrepass" and
    . != "bevy_pbr::light::NotShadowCaster" and
    . != "bevy_pbr::light::NotShadowReceiver" and
    . != "bevy_pbr::volumetric_fog::VolumetricLight" and
    . != "bevy_render::experimental::occlusion_culling::OcclusionCulling" and
    . != "bevy_pbr::mesh_material::MeshMaterial3d<bevy_pbr::extended_material::ExtendedMaterial<bevy_pbr::pbr_material::StandardMaterial, bevy_pbr::decal::forward::ForwardDecalMaterialExt>>" and
    . != "bevy_pbr::mesh_material::MeshMaterial3d<bevy_pbr::pbr_material::StandardMaterial>" and
    . != "bevy_sprite::mesh2d::material::MeshMaterial2d<bevy_sprite::mesh2d::color_material::ColorMaterial>"
  )) |
  map({type: ., spawn_test: "untested", mutation_tests: "untested", notes: ""})
' > test-app/examples/type_validation.json
```

This approach is fast and reliable - it creates the file immediately without any blocking issues.

### 5. Report results
```
✅ Initialized type validation tracking file
- Total types: [count]
- File location: test-app/examples/type_validation.json
- All types marked as "untested"
```

### 6. Cleanup (optional)
If you don't need the app running after initialization:
```bash
mcp__brp__brp_shutdown(
    app_name="extras_plugin",
    port=22222
)
```

## Important Notes

- **Overwrites**: This command will overwrite any existing tracking file by using the Write tool to create a completely new file
- **Fresh start**: All types will be marked as "untested" regardless of previous test results
- **Component discovery**: Only components registered with BRP reflection will be included
- **File Creation**: ALWAYS use the Write tool to create a new file. NEVER use the Edit tool to modify an existing type_validation.json file
- **File Location**: The file is now stored in `test-app/examples/` instead of `.claude/commands/` to avoid requiring approval for edits

## Error Handling

If the app fails to launch:
- Check if port 22222 is already in use
- Ensure the extras_plugin example is built

If BRP doesn't respond:
- Verify the app includes the RemotePlugin
- Check that the app launched successfully

## Example Output

After running this command, the file will contain:
```json
[
  {
    "type": "bevy_core_pipeline::bloom::settings::Bloom",
    "spawn_test": "untested",
    "mutation_tests": "untested",
    "notes": ""
  },
  {
    "type": "bevy_core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpening",
    "spawn_test": "untested",
    "mutation_tests": "untested",
    "notes": ""
  },
  // ... all other types ...
]
```
