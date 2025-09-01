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
# Filter out excluded types, then transform each type into the tracking structure with batch numbers
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
  map({type: ., spawn_test: "untested", mutation_tests: "untested", mutation_paths: {}, batch_number: "", notes: ""})
' > test-app/examples/type_validation.json
```

This approach is fast and reliable - it creates the file immediately without any blocking issues.

### 5. Discover type capabilities and populate mutation paths using type schema
**IMPORTANT: This step enhances the initial file with spawn capability detection and detailed mutation path information.**

After creating the basic tracking file, discover type capabilities and mutation paths for all types:

1. **Read the created tracking file**:
   ```bash
   # Use Read tool to load test-app/examples/type_validation.json
   # Extract the list of type names for processing
   ```

2. **Process types in batches** (10 types at a time for efficiency):
   ```bash
   mcp__brp__brp_type_schema(
       port=22222,
       types=[batch_of_10_types]
   )
   ```

3. **For each batch result, analyze each type's capabilities**:
   - Extract `supported_operations` from the schema response
   - **Spawn capability**: If `spawn` is in `supported_operations`, keep `spawn_test: "untested"`. If not, set `spawn_test: "skipped"`
   - Extract `mutation_info` from the schema response
   - **Mutation capability**: If mutations are supported, get all available `mutation_paths` and set each to "untested". If no mutations available, set `mutation_tests: "n/a"`

4. **Update tracking file with discovered information**:
   - Use MultiEdit tool to update multiple type entries at once
   - For each type in the batch, update:
     - `spawn_test`: "untested" or "skipped" based on spawn support
     - `mutation_tests`: "untested" or "n/a" based on mutation support  
     - `mutation_paths`: populated object or empty `{}` based on available paths

5. **Repeat for all batches** until all types have their capabilities and mutation paths populated

**Expected final format after this step:**
```json
{
  "type": "bevy_transform::components::transform::Transform",
  "spawn_test": "untested",
  "mutation_tests": "untested", 
  "mutation_paths": {
    ".translation.x": "untested",
    ".translation.y": "untested", 
    ".translation.z": "untested",
    ".rotation.x": "untested",
    ".rotation.y": "untested",
    ".rotation.z": "untested",
    ".rotation.w": "untested",
    ".scale.x": "untested",
    ".scale.y": "untested", 
    ".scale.z": "untested"
  },
  "batch_number": "",
  "notes": ""
}
```

**For types without spawn support:**
```json
{
  "type": "bevy_pbr::components::VisibleMeshEntities",
  "spawn_test": "skipped",
  "mutation_tests": "n/a",
  "mutation_paths": {},
  "batch_number": "",
  "notes": ""
}
```

This approach creates a fully initialized tracking file where the test runner can immediately begin testing without needing to discover type capabilities or mutation paths.

### 6. Report results
```
✅ Initialized type validation tracking file with full type capability discovery
- Total types: [count]
- File location: test-app/examples/type_validation.json
- Spawn capabilities discovered (untested/skipped)
- Mutation paths populated (untested/n/a)
```

### 7. Cleanup
Shutdown the app after initialization:
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
    "mutation_paths": {},
    "batch_number": 1,
    "notes": ""
  },
  {
    "type": "bevy_core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpening",
    "spawn_test": "untested",
    "mutation_tests": "untested", 
    "mutation_paths": {},
    "batch_number": 1,
    "notes": ""
  },
  // ... all other types with batch_number assigned (1-based, groups of 10) ...
]
```
