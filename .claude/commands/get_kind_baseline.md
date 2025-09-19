# Get Type Kind
Analyzes type_kind values in mutation paths from the baseline file.

## Usage

### Summary Mode (no arguments)
Shows a count of how many top-level types contain at least one mutation path of each type_kind:

```bash
.claude/scripts/get_type_kind.sh
```

Example output:
```
Type kind summary (types containing at least one mutation path of each kind):

Array: 8
Enum: 45
List: 13
Map: 2
Set: 1
Struct: 67
Tuple: 5
TupleStruct: 12
Value: 98
```

### Query Mode (with type_kind argument)
Shows all top-level type names that contain at least one mutation path with the specified type_kind:

```bash
.claude/scripts/get_type_kind.sh List
```

Example output:
```
Types containing mutation paths with type_kind 'List':

bevy_ecs::hierarchy::Children
bevy_mesh::morph::MeshMorphWeights
bevy_mesh::morph::MorphWeights
bevy_mesh::skinning::SkinnedMesh
bevy_pbr::light::CascadeShadowConfig
bevy_render::view::visibility::VisibilityClass
bevy_render::view::visibility::render_layers::RenderLayers
bevy_text::pipeline::TextLayoutInfo
bevy_text::text::ComputedTextBlock
bevy_ui::ui_node::BoxShadow
bevy_ui::ui_node::Node
bevy_window::monitor::Monitor
extras_plugin::TestCollectionComponent
```

## Prerequisites

- Requires baseline file at `$TMPDIR/all_types_baseline.json`
- Python 3 must be installed
- The baseline file must have the expected structure with `type_guide` array containing types with `mutation_paths`

## Notes

- The script examines the `type_kind` field within `path_info` of each mutation path
- A type is counted/listed if it contains **at least one** mutation path with the specified type_kind
- The summary shows unique type counts (each type counted once per type_kind, regardless of how many paths match)
- Type names are sorted alphabetically in the output
