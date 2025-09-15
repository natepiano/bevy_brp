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

### 2. Refactor BrpClient Error Handler Pipeline
Update `BrpClient::execute()` with efficient sequential error handling:

**Extract type names ONCE** at beginning of error pipeline using `extract_type_names_by_method()`

1. **FIRST**: ShortPath resolution handler (for "Unknown component type" errors only)
   - Use pre-extracted type names
   - Attempt shortPath resolution using registry
   - If successful, retry with resolved parameters
   - If ambiguous, return detailed disambiguation error
   - **If resolution fails, return original error immediately (do NOT pass through)**

2. **SECOND**: Enhanced format error handler (for KNOWN types with format errors)
   - Only when `enhanced_errors = true`
   - Pass pre-extracted type names to `create_enhanced_format_error()`
   - Refactor `create_enhanced_format_error()` to accept type names parameter
   - Remove all type extraction logic from this function (types already known)
   - Always call `create_full_type_error()` since types are guaranteed valid

3. **THIRD**: Regular error handling (fallback)

### 3. Add Method-Specific Type Extraction Logic
Create dedicated type extraction that doesn't rely on Operation enum:
- `extract_type_names_by_method()` - handles different parameter patterns per BrpMethod
- `rewrite_params_with_resolved_types()` - substitutes resolved fullPaths back into parameters
- Keep Operation enum focused on format error correction (malformed values)

### 4. Refactor Enhanced Error System
Simplify the enhanced error system now that unknown types are handled separately:
- Update `create_enhanced_format_error()` signature to accept pre-extracted type names
- Remove `extract_types_from_error_message()` and Operation enum extraction logic
- Remove `create_minimal_type_error()` conditional - always use `create_full_type_error()`
- Keep enhanced error configurations unchanged (no need to add to Vec<String> tools)

## Design Review Skip Notes

### DESIGN-1: Potential duplication with existing TypeGuideEngine - **Verdict**: REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Add ShortPath Resolution Module
- **Issue**: Proposed ShortPathResolver duplicates functionality already in TypeGuideEngine which has registry caching, type lookup, and error handling capabilities
- **Reasoning**: This isn't meaningful duplication - both operations follow appropriate fetch-use-discard pattern for registry data. TypeGuideEngine creates registry once per type guide request, uses it, throws it away. ShortPath resolution will do the same for error handling. Creating shared infrastructure would add complexity without meaningful benefit for these ephemeral, request-scoped operations.
- **Existing Implementation**: TypeGuideEngine design is appropriate for its use case and shortPath resolution should follow the same simple pattern
- **Critical Note**: Future reviewers should distinguish between harmful code duplication and appropriate independent operations that happen to use similar infrastructure

### IMPLEMENTATION-1: No strategy for extending type extraction beyond Operation enum - **Verdict**: REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Add Parameter Rewriting Logic
- **Issue**: Plan proposes Operation::rewrite_params() but doesn't address how to handle BevyGet, BevyRemove, BevyGetWatch which need different parameter extraction patterns for Vec<String> fields
- **Reasoning**: With the updated error handling architecture, this finding is no longer relevant. The plan now uses `extract_type_names_by_method()` at the beginning of the error pipeline and doesn't require Operation enum extension. Operation enum remains focused on format error correction only. The suggested trait-based system would be over-engineering for the simpler method-specific extraction approach adopted.
- **Existing Implementation**: Plan already addresses this through `extract_type_names_by_method()` and keeps Operation enum unchanged
- **Critical Note**: The corrected architecture eliminated the need for Operation enum extension or complex trait-based systems

### DESIGN-2: Plan doesn't leverage existing enhanced error infrastructure consistently - **Verdict**: REDUNDANT
- **Status**: REDUNDANT - Already addressed in plan
- **Location**: Section: Enhance BrpClient Error Handling
- **Issue**: Plan adds shortPath resolution to create_enhanced_format_error but this is only called when tools have enhanced_errors=true, which most Vec<String> tools don't have
- **Reasoning**: With the corrected error handling architecture, this concern is obsolete. ShortPath resolution now happens in its own dedicated error handler before enhanced errors, making it independent of `enhanced_errors = true` configuration. Enhanced error system remains focused on format errors for spawn/insert/mutate operations. Vec<String> tools don't need enhanced errors enabled for shortPath functionality.
- **Existing Implementation**: ShortPath resolution operates independently in the error pipeline and doesn't depend on enhanced error configurations
- **Critical Note**: The original finding was based on the flawed Operation enum approach that has been corrected

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