# ShortPath Auto-Resolution Implementation Plan

## Feasibility Assessment: **HIGHLY FEASIBLE** ✅

The system already has 90% of the infrastructure needed! There's sophisticated error handling in place with type extraction and enhanced errors. We just need to add shortPath resolution and retry logic.

## Key Findings

### Perfect Choke Point Identified
`BrpClient::execute()` in `client.rs:96-113` already:
- Detects format errors via `is_format_error()`
- Extracts type names from parameters and error messages
- Has `ENHANCED_ERRORS` flag to control behavior
- Can be extended with retry logic

### Tools That Take Type Names
- **Vec<String>**: `BevyGet`, `BevyRemove`, `BevyGetWatch`
- **String**: `BevyGetResource`, `BevyMutateComponent`, `BevyMutateResource`, `BevyInsertResource`, `BevyRemoveResource`
- **JSON Keys**: `BevySpawn`, `BevyInsert` (extracted from components object)

## Implementation Strategy

### 1. Add ShortPath Resolution Module
```rust
// mcp/src/brp_tools/brp_client/short_path_resolver.rs
struct ShortPathResolver {
    registry_cache: Option<Value>, // Cache registry for session
}

impl ShortPathResolver {
    async fn resolve_short_paths(&mut self, types: Vec<String>) -> Result<ShortPathResolution>
    fn build_ambiguity_error(&self, ambiguous: HashMap<String, Vec<String>>) -> Error
}

enum ShortPathResolution {
    AllResolved(HashMap<String, String>), // shortPath -> fullPath
    HasAmbiguities(HashMap<String, Vec<String>>), // ambiguous shortPaths
}
```

### 2. Enhance BrpClient Error Handling
Update `create_enhanced_format_error()` to:
1. Extract type names (already done)
2. **NEW**: Attempt shortPath resolution for unknown types
3. **NEW**: If successful, retry the original request with resolved types
4. **NEW**: If ambiguous, return detailed error with disambiguation options

### 3. Add Parameter Rewriting Logic
Extend `Operation::extract_type_names()` with companion `Operation::rewrite_params()` to substitute resolved fullPaths back into the original request parameters.

### 4. Enable For Specific Tools
Add `#[brp_result(enhanced_errors = true)]` to more result structs (currently only `MutateComponentResult` has this).

## Implementation Steps

### Phase 1: Core Resolution Logic
1. **ShortPathResolver**: Registry caching, resolution logic, duplicate detection
2. **Error Types**: `ShortPathResolution` enum and ambiguity error formatting
3. **Parameter Rewriting**: `Operation::rewrite_params()` method
4. **Tests**: Comprehensive test coverage for resolution edge cases

### Phase 2: Integration
1. **Enhance BrpClient**: Add retry logic to `create_enhanced_format_error()`
2. **Registry Caching**: Session-based caching to avoid repeated calls
3. **Enable Enhanced Errors**: Update more result structs to use enhanced error handling

### Phase 3: Polish & Validation
1. **No Special Cases**: All tools use uniform shortPath resolution through `BrpClient::execute()`
2. **Error Messages**: User-friendly disambiguation prompts
3. **Documentation**: Update help text to mention shortPath support
4. **Comprehensive Test Suite**: Create `.claude/commands/tests/shortpath_resolution.md`

## Benefits

- **Seamless UX**: Users can use `Transform` instead of `bevy_transform::components::transform::Transform`
- **Intelligent Errors**: When ambiguous, shows all matches with clear disambiguation
- **Zero Breaking Changes**: Existing full paths continue to work
- **Selective Rollout**: Only tools with `enhanced_errors = true` get the feature
- **Performance**: Registry caching prevents excessive calls

## Example User Experience

**Before:**
```json
{"error": "Unknown component type: `Transform`"}
```

**After (Success):**
Request automatically retried with `bevy_transform::components::transform::Transform`

**After (Ambiguous):**
```json
{
  "error": "Ambiguous type 'Transform' matches multiple types:",
  "disambiguation": {
    "Transform": [
      "bevy_transform::components::transform::Transform",
      "my_game::physics::Transform",
      "ui::layout::Transform"
    ]
  },
  "suggestion": "Use full path to specify which type you want"
}
```

This is a high-impact feature that leverages existing infrastructure beautifully!

## Comprehensive Test Requirements

### Test Coverage: `.claude/commands/tests/shortpath_resolution.md`

The shortPath resolution feature needs extensive testing across multiple scenarios and parameter types. This test should validate:

#### 1. Array Parameter Scenarios (Vec<String>)
**Tools**: `BevyGet`, `BevyRemove`, `BevyGetWatch`

- **All Valid ShortPaths**: `["Transform", "Sprite"]` → Auto-resolve to full paths
- **All Invalid Types**: `["InvalidType1", "InvalidType2"]` → Return original "Unknown component type" errors after retry
- **Mixed Valid/Invalid**: `["Transform", "InvalidType"]` → Partial resolution with remaining errors
- **Mixed Short/Full Paths**: `["Transform", "bevy_sprite::sprite::Sprite"]` → Resolve shortPaths, preserve full paths
- **Ambiguous Types**: `["Transform"]` where multiple Transform types exist → Return disambiguation error
- **Edge Case**: `["std::collections::HashMap"]` (stdlib types) → Should not resolve, return original error

#### 2. Single Parameter Scenarios (String)
**Tools**: `BevyGetResource`, `BevyMutateComponent`, etc.

- **Valid ShortPath**: `"Transform"` → Auto-resolve and retry
- **Invalid Type**: `"InvalidType"` → Return original error after retry attempt
- **Already Full Path**: `"bevy_transform::components::transform::Transform"` → Pass through unchanged
- **Ambiguous ShortPath**: `"Transform"` → Return disambiguation with all matches
- **Stdlib Types**: `"HashMap"` or `"Vec"` → Should not attempt resolution

#### 3. JSON Object Key Scenarios
**Tools**: `BevySpawn`, `BevyInsert`

- **Mixed Keys**:
  ```json
  {
    "Transform": {...},
    "bevy_sprite::sprite::Sprite": {...}
  }
  ```
  → Resolve shortPaths in keys, preserve full paths

#### 4. Error Handling Edge Cases
- **Registry Unavailable**: What happens if `bevy/registry/schema` fails?
- **Empty Registry**: App with no registered components
- **Network Timeout**: Registry call times out
- **Malformed Registry**: Registry returns invalid data

#### 5. Performance & Caching
- **Registry Caching**: Multiple calls should reuse cached registry
- **Cache Invalidation**: How to handle app restarts/changes
- **Large Registry**: Performance with 1000+ registered types

#### 6. Integration Testing
- **Cross-Tool Consistency**: Same shortPath behavior across all enhanced tools
- **Backwards Compatibility**: Full paths continue working unchanged
- **Mixed Environments**: Apps with different registered types

This comprehensive test suite ensures robust shortPath resolution that handles real-world usage patterns and edge cases gracefully.