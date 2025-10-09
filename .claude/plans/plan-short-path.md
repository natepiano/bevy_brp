# ShortPath Auto-Resolution Implementation Plan

## Feasibility Assessment: **HIGHLY FEASIBLE** ✅

The system already has 90% of the infrastructure needed! There's sophisticated error handling in place with type extraction and enhanced errors. We just need to add shortPath resolution and retry logic.

## Key Findings

### Perfect Choke Point Identified
`BrpClient::execute()` in `client.rs:96-113` already:
- Detects format errors via `has_format_error_code()`
- Extracts type names from parameters and error messages
- Has `ADD_TYPE_GUIDE_TO_ERROR` flag to control behavior
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
    registry: HashMap<String, Value>, // NO CACHING - fetched fresh per error
}

impl ShortPathResolver {
    /// Create fresh instance per error - reuse TypeGuideEngine's registry fetching
    pub async fn new(port: Port) -> Result<Self> {
        // Reuse existing registry fetching mechanism from TypeGuideEngine
        // Make TypeGuideEngine::fetch_registry() public or extract to common helper
        let registry = TypeGuideEngine::fetch_registry(port).await?;
        Ok(Self { registry })
    }

    pub fn resolve_short_paths(&self, types: Vec<String>) -> ShortPathResolution
    pub fn build_ambiguity_error(&self, ambiguous: HashMap<String, Vec<String>>) -> Error
}

enum ShortPathResolution {
    AllResolved(HashMap<String, String>), // shortPath -> fullPath
    HasAmbiguities(HashMap<String, Vec<String>>), // ambiguous shortPaths
}
```

### 2. Refactor BrpClient Error Handler Pipeline
Update `BrpClient::execute()` with efficient sequential error handling:

**Extract type names ONCE** at beginning of error pipeline using `method.extract_type_names(params)`

1. **FIRST**: ShortPath resolution handler (for "Unknown component type" errors only)
   - Use pre-extracted type names
   - Attempt shortPath resolution using registry
   - If successful, retry with resolved parameters
   - If ambiguous, return detailed disambiguation error
   - **If resolution fails, return original error immediately (do NOT pass through)**

2. **SECOND**: Enhanced format error handler (for KNOWN types with format errors)
   - Only when `ADD_TYPE_GUIDE_TO_ERROR = true`
   - Pass pre-extracted type names to `add_type_guide_to_error()`
   - Refactor `add_type_guide_to_error()` to accept type names parameter
   - Remove all type extraction logic from this function (types already known)
   - Always call `add_type_guide_to_error()` since types are guaranteed valid

3. **THIRD**: Regular error handling (fallback)

**Note on ADD_TYPE_GUIDE_TO_ERROR flag**: The current boolean flag appropriately represents a binary choice (enhanced vs basic error handling). ShortPath resolution operates independently as a separate error handling step, maintaining clear separation of concerns where enhanced errors focus on format correction for known types, while ShortPath resolution handles unknown type name resolution.

### 3. Add Method-Specific Type Extraction Logic
Add type extraction as a method on `BrpMethod` enum to follow proper object-oriented design principles:

```rust
use crate::json_object::{JsonObjectAccess, IntoStrings};
use crate::tool::{BrpMethod, ParameterName};

impl BrpMethod {
    /// Extract type names from parameters for this method
    /// Uses type-safe field access via ParameterName enum
    pub fn extract_type_names(&self, params: &Value) -> Vec<String> {
        match self {
            // Vec<String> in Components field
            Self::BevyGet | Self::BevyRemove => {
                params.get_field(ParameterName::Components)
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str())
                        .into_strings())
                    .unwrap_or_default()
            },
            // Vec<String> in Types field
            Self::BevyGetWatch => {
                params.get_field(ParameterName::Types)
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str())
                        .into_strings())
                    .unwrap_or_default()
            },
            // String in Component field
            Self::BevyMutateComponent => {
                params.get_field_str(ParameterName::Component)
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default()
            },
            // String in Resource field
            Self::BevyGetResource | Self::BevyInsertResource |
            Self::BevyMutateResource | Self::BevyRemoveResource => {
                params.get_field_str(ParameterName::Resource)
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default()
            },
            // Object keys in Components field
            Self::BevySpawn | Self::BevyInsert => {
                params.get_field(ParameterName::Components)
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.keys().into_strings())
                    .unwrap_or_default()
            },
            _ => Vec::new(),
        }
    }
}

// Update Operation to use the new method
impl Operation {
    /// Extract type names using the unified BrpMethod extraction
    pub fn extract_type_names(self, method: &BrpMethod, params: &Value) -> Vec<String> {
        // Operation enum only cares about the subset it handles for format correction
        // Delegate to the unified extraction method
        method.extract_type_names(params)
    }
}
```

**Important**: Also update the existing extraction helper functions to use type-safe field access:

```rust
/// Extract type names from components object keys in spawn/insert operations
fn extract_from_components_object(params: &Value) -> Vec<String> {
    params
        .get_field(ParameterName::Components)
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Extract type name from resource field in resource operations
fn extract_from_resource_field(params: &Value) -> Vec<String> {
    params
        .get_field(ParameterName::Resource)
        .and_then(|v| v.as_str())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}

/// Extract single component type from component field in mutation operations
fn extract_single_component_type(params: &Value) -> Vec<String> {
    params
        .get_field(ParameterName::Component)
        .and_then(|v| v.as_str())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}

/// Extract single resource type from resource field in mutation operations
fn extract_single_resource_type(params: &Value) -> Vec<String> {
    params
        .get_field(ParameterName::Resource)
        .and_then(|v| v.as_str())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}
```

**Key improvements**:
- Single source of truth for type extraction in `BrpMethod::extract_type_names()`
- Operation enum delegates to the unified method for its subset of operations
- Uses `ParameterName` enum instead of string literals
- Uses `JsonObjectAccess::get_field()` for type-safe field access
- Uses `IntoStrings` trait for cleaner string collection
- Operation enum remains focused on format error correction (malformed values)

### 4. Refactor Enhanced Error System
Simplify the enhanced error system now that unknown types are handled separately:
- Update `add_type_guide_to_error()` signature to accept pre-extracted type names
- Remove `extract_types_from_error_message()` and Operation enum extraction logic from `try_add_type_guide_to_error()`
- Remove `create_minimal_type_error()` conditional - always use `add_type_guide_to_error()`
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
- **Issue**: Plan adds shortPath resolution to add_type_guide_to_error but this is only called when tools have ADD_TYPE_GUIDE_TO_ERROR=true, which most Vec<String> tools don't have
- **Reasoning**: With the corrected error handling architecture, this concern is obsolete. ShortPath resolution now happens in its own dedicated error handler before enhanced errors, making it independent of `ADD_TYPE_GUIDE_TO_ERROR = true` configuration. Enhanced error system remains focused on format errors for spawn/insert/mutate operations. Vec<String> tools don't need enhanced errors enabled for shortPath functionality.
- **Existing Implementation**: ShortPath resolution operates independently in the error pipeline and doesn't depend on enhanced error configurations
- **Critical Note**: The original finding was based on the flawed Operation enum approach that has been corrected

## Design Review Skip Notes

### TYPE-SYSTEM-1: ShortPathResolution enum uses raw string types - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Add ShortPath Resolution Module
- **Issue**: ShortPathResolution enum uses HashMap<String, String> and HashMap<String, Vec<String>> representing finite state transitions but lacks type safety for the resolution status
- **Reasoning**: This finding misapplies type safety principles. The strings represent dynamic component type paths from Bevy's runtime registry (like 'bevy_transform::components::transform::Transform'), not finite predefined values. These are arbitrary text paths that change based on what components are registered in the Bevy app. Adding newtype wrappers like ShortPath(String) would add boilerplate without providing meaningful compile-time safety, since the actual validation happens at runtime through registry lookups. This is appropriate string usage for arbitrary text processing, not a case where enums would be beneficial.
- **Decision**: User elected to skip this recommendation

### IMPLEMENTATION-2: Missing comprehensive error handling strategy for registry failures - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Comprehensive Test Requirements
- **Issue**: Plan mentions registry failure scenarios in test section but doesn't define the error handling strategy in the implementation sections. What happens when registry calls fail, timeout, or return malformed data?
- **Reasoning**: This finding is based on incorrect analysis. The comprehensive error handling already exists throughout the stack. ShortPath resolution will reuse TypeGuideEngine's registry fetching mechanism, which already has proper error handling. Registry failures should surface as errors and stop execution - this is the correct behavior. The existing error handling infrastructure is sufficient and well-designed.
- **Decision**: User elected to skip this recommendation

### DESIGN-4: Plan violates atomic migration by creating parallel systems instead of replacing Operation enum - **Verdict**: REJECTED
- **Status**: SKIPPED
- **Location**: Section: Refactor Enhanced Error System
- **Issue**: Plan proposes keeping Operation enum unchanged while creating new extract_type_names_from_params function. This violates atomic migration principle by creating hybrid approach instead of complete replacement
- **Reasoning**: This finding is based on outdated or incorrect information. The actual implementation does NOT create a parallel system. Instead, the Operation enum was properly enhanced with an `extract_type_names` method (lines 201-229) that handles type extraction based on operation state. There is no separate `extract_type_names_from_params` function as claimed. The design is clean and unified - the Operation enum owns its behavior through proper methods. The implementation correctly supports only the 4 BRP methods that actually need type extraction (spawn, insert, mutate operations), while appropriately excluding read-only operations that don't send type data. The current code follows good object-oriented design principles with proper encapsulation.
- **Decision**: User elected to skip this recommendation

## Implementation Steps

### Phase 1: Core Resolution Logic
1. **ShortPathResolver**: Registry caching, resolution logic, duplicate detection
2. **Error Types**: `ShortPathResolution` enum and ambiguity error formatting
3. **Parameter Rewriting**: `rewrite_params_with_resolved_types()` function (separate from Operation enum)
4. **Tests**: Comprehensive test coverage for resolution edge cases

### Phase 2: Integration
1. **Enhance BrpClient**: Add shortPath resolution as FIRST step in error pipeline (before enhanced errors)
2. **No Caching**: Fetch registry fresh per error attempt - no session state
3. **Keep Enhanced Errors unchanged**: Enhanced errors remain separate for format correction

### Phase 3: Polish & Validation
1. **Universal Coverage**: All tools that handle type names get shortPath resolution through `BrpClient::execute()`, independent of enhanced_errors setting
2. **Error Messages**: User-friendly disambiguation prompts
3. **Documentation**: Update help text to mention shortPath support
4. **Comprehensive Test Suite**: Create `.claude/commands/tests/shortpath_resolution.md`

## Benefits

- **Seamless UX**: Users can use `Transform` instead of `bevy_transform::components::transform::Transform`
- **Intelligent Errors**: When ambiguous, shows all matches with clear disambiguation
- **Zero Breaking Changes**: Existing full paths continue to work
- **Universal Application**: ShortPath resolution applies to ALL BRP calls that contain type names, regardless of enhanced_errors setting
- **Simplicity**: No caching complexity - fresh registry fetch per error keeps design simple

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
- **Registry Unavailable**: What happens if `registry.schema` fails?
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
