# Plan: Fix Enum Examples Structure

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
   cargo build && cargo +nightly fmt
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

## INTERACTIVE IMPLEMENTATION SEQUENCE

### Step 0: Disconnect Old Enum Builder ✅ COMPLETED
→ **See detailed section 0 below**

### Step 1: Foundation Types Setup ✅ COMPLETED
→ **See detailed section 1 below**
→ **TEMPORARY FIXES ADDED**: Modified conversion code in types.rs `from_mutation_path_internal()` and `convert_signature_groups_array()` to compile with new ExampleGroup structure - must be properly fixed in Step 6

### Step 2: Internal Enum Builder Structure ✅ COMPLETED
→ **See detailed section 2 below**

### Step 3: Data Structure Extensions ✅ COMPLETED
→ **See detailed section 3 below**

### Step 4: Enum Builder Core Implementation ✅ COMPLETED
→ **See detailed section 4 below**

### Step 5: Protocol Enforcer Updates ✅ COMPLETED
→ **See detailed section 5 below**

### Step 6: Conversion Logic Simplification ⏳ PENDING
→ **See detailed section 6 below**

### Step 7: Complete Validation ⏳ PENDING
→ **See detailed section 8 below**

## Problem Statement

The current system has multiple issues:
1. **Inconsistent field naming**: `"variants"` internally vs `"applicable_variants"` in the API
2. **JSON encoding hacks**: Using `__variant_context` and magic field names in JSON
3. **Tight coupling**: Enum builder output is tightly coupled to conversion logic through JSON structure
4. **Bug in old implementation**: When an enum is a field in a struct, the root example incorrectly shows an array of examples instead of a concrete value
5. **Bug in new implementation**: Not returning the proper examples structure for enum root paths

## Current Flow

1. **Old enum_builder.rs**:
   - Builds JSON with `"variants"` field
   - Uses `__variant_context` wrapper for child paths
   - Returns array of signature groups for root path

2. **Conversion in types.rs**:
   - Parses JSON looking for `"variants"` field
   - Extracts `__variant_context` for applicable variants
   - Creates `ExampleGroup` with `applicable_variants`

3. **Output Structure**:
   - Root enum path: `examples` array with `applicable_variants`, `signature`, `example`
   - Child paths (.0, .1, etc.): Single `example` with `variants` field listing applicable variants
   - Embedded in struct: Should be single concrete value, but currently broken (shows array)

## Proposed Solution

### 0. Disconnect Old Enum Builder ✅ COMPLETED

**Objective:** Fully migrate to new enum builder and disconnect old implementation

**Key Changes:**
1. **Type visibility swap**: Made `EnumVariantInfo` and `EnumFieldInfo` public in `new_enum_builder.rs` with proper serde derives
2. **Old builder isolation**: Converted old builder types to private `EnumVariantInfoOld`/`EnumFieldInfoOld`
3. **Module exports**: Commented out old enum_builder in mod.rs, now only exports from new_enum_builder
4. **Type system**: Removed `USE_NEW_ENUM_BUILDER` flag, now always uses `NewEnumMutationBuilder` with ProtocolEnforcer wrapper
5. **Full migration**: All builders now use ProtocolEnforcer wrapper (line 86 in type_kind.rs)

**Result:** Old enum_builder.rs is completely disconnected but kept for reference. System fully uses new implementation.

### 1. Foundation Types Setup ✅ COMPLETED

**Objective:** Add shared types and EnumContext to core modules for enum handling foundation

**Files to modify:**
- `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
- `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

**Build:** `cargo build && cargo +nightly fmt`

**ACTUAL IMPLEMENTATION NOTES:**
- **Moved** `VariantSignature` enum from private in new_enum_builder.rs to public in types.rs with Display trait
- **Modified** existing `ExampleGroup` struct with new required fields (changed from Optional fields)
- Added `serialize_signature` helper function for ExampleGroup serialization
- **Moved** `shorten_type_name` helper function from enum_builder.rs to types.rs (also removed duplicate from new_enum_builder.rs)
- Updated new_enum_builder.rs to use public VariantSignature from types.rs
- Removed `format_signature()` from new_enum_builder.rs (replaced by Display trait)
- Added `EnumContext` enum to recursion_context.rs
- Added `enum_context: Option<EnumContext>` field to RecursionContext
- Updated RecursionContext::new() to initialize enum_context as None (changed from `const fn` to `fn`)
- Updated create_recursion_context() to propagate parent's enum_context
- **TEMPORARY**: Modified `MutationPath::from_mutation_path_internal()` - skips creating ExampleGroup for non-array cases
- **TEMPORARY**: Modified `MutationPath::from_mutation_path_internal()` - uses `applicable_variants.is_empty()` check
- **TEMPORARY**: Modified `MutationPath::convert_signature_groups_array()` - creates dummy VariantSignature values

#### Update types.rs with shared types

```rust
// In types.rs - move VariantSignature here for public API use
/// Variant signature types for enum variants - used for grouping similar structures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariantSignature {
    /// Unit variant (no data)
    Unit,
    /// Tuple variant with ordered types
    Tuple(Vec<BrpTypeName>),
    /// Struct variant with named fields and types
    Struct(Vec<(String, BrpTypeName)>),
}

impl std::fmt::Display for VariantSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Tuple(types) => {
                let type_names: Vec<String> = types.iter().map(|t| shorten_type_name(t.as_str())).collect();
                write!(f, "tuple({})", type_names.join(", "))
            }
            Self::Struct(fields) => {
                let field_strs: Vec<String> = fields.iter()
                    .map(|(name, type_name)| format!("{}: {}", name, shorten_type_name(type_name.as_str())))
                    .collect();
                write!(f, "struct{{{}}}", field_strs.join(", "))
            }
        }
    }
}

/// Example group for enum variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleGroup {
    /// List of variants that share this signature
    pub applicable_variants: Vec<String>,

    /// The variant signature type (serialized as string using Display)
    #[serde(serialize_with = "serialize_signature")]
    pub signature: VariantSignature,

    /// Example value for this group
    pub example: Value,
}

fn serialize_signature<S>(sig: &VariantSignature, s: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    s.serialize_str(&sig.to_string())
}
```

#### EnumContext Addition

Add to `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`:

```rust
/// Tracks enum-specific context during recursion
#[derive(Debug, Clone)]
pub enum EnumContext {
    /// This enum is establishing the root context
    Root,

    /// Building under enum variant(s)
    Child {
        /// Chain of variant constraints from parent to child
        /// e.g., [(TestEnumWithSerDe, ["Nested"]), (NestedConfigEnum, ["Conditional"])]
        variant_chain: Vec<(BrpTypeName, Vec<String>)>,
    },
}

pub struct RecursionContext {
    // ... existing fields ...
    /// Track enum context - None for non-enum types
    pub enum_context: Option<EnumContext>,
}
```

#### RecursionContext Creation Changes

Need to update these methods to handle `enum_context`:
- `RecursionContext::root()` - starts with `enum_context: None`
- `RecursionContext::create_child_context()` - propagates parent's `enum_context` by default
- ProtocolEnforcer - sets `Some(EnumContext::Root)` when processing an enum type
- Enum builder's child creation - sets `Some(EnumContext::Child { ... })` for its children

### 2. Internal Enum Builder Structure ⏳ PENDING

**Objective:** Add internal MutationExample enum and helper methods to enum builder

**Files to modify:**
- `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/new_enum_builder.rs`

**Build:** `cargo build && cargo +nightly fmt`
**Dependencies:** Requires Step 1

#### Internal MutationExample Enum for Enum Builder Only

Keep the `example: Value` field in `MutationPathInternal` unchanged. Instead, use `MutationExample` as an internal implementation detail within the enum builder only:

```rust
// In new_enum_builder.rs ONLY - not exported to other modules
use std::fmt;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;

/// Internal example data for enum mutation paths - used only within enum builder
/// Other builders continue using Value directly
#[derive(Debug, Clone)]
enum MutationExample {
    /// Simple example value (for non-enum types and embedded enum values)
    Simple(Value),

    /// Multiple examples with signatures (for enum root paths)
    /// Each group has applicable_variants, signature, and example
    EnumRoot(Vec<ExampleGroup>),

    /// Example with variant context (for enum child paths like .0, .1, .enabled)
    EnumChild {
        example: Value,
        applicable_variants: Vec<String>,
    },
}
```

The `MutationExample` enum belongs in `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/new_enum_builder.rs` as an internal implementation detail.

The enum builder determines what to return based on context:
- **Return `Simple`**: When `ctx.enum_context` is `None` (non-enum parent needs concrete value)
- **Return `EnumRoot`**: When `ctx.enum_context` is `Some(EnumContext::Root)` (building the enum's root path)
- **Return `EnumChild`**: When `ctx.enum_context` is `Some(EnumContext::Child { ... })` (building under enum variant)

### 3. Data Structure Extensions ⏳ PENDING

**Objective:** Add enum_root_examples field to MutationPathInternal for direct data transfer

**Files to modify:**
- `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Build:** `cargo build && cargo +nightly fmt`
**Dependencies:** Requires Step 1

#### Update MutationPathInternal

```rust
pub struct MutationPathInternal {
    /// Example data for this path - for enum children includes applicable_variants
    pub example: Value,

    /// For enum roots only: the examples array with all variant groups
    /// None for all other paths (including enum children and regular types)
    pub enum_root_examples: Option<Vec<ExampleGroup>>,

    // ... other fields remain the same
}
```

This explicit field eliminates JSON parsing in conversion - data transfers directly to `MutationPath`.

### 4. Enum Builder Core Implementation ⏳ PENDING

**Objective:** Implement new assemble_from_children logic with EnumContext handling

**Files to modify:**
- `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/new_enum_builder.rs`

**Build:** `cargo build && cargo +nightly fmt`
**Dependencies:** Requires Steps 1, 2, 3

#### new_enum_builder.rs Implementation

**Note:** This implementation extracts helper functions `build_variant_example()`, `concrete_example()`, and `flatten_variant_chain()` to keep the code clean and maintainable. The `format_signature()` function is replaced by the `Display` trait implementation on `VariantSignature`.

```rust
impl NewEnumMutationBuilder {
    /// Build a complete example for a variant with all its fields
    fn build_variant_example(
        &self,
        signature: &VariantSignature,
        variant_name: &str,
        children: &HashMap<MutationPathDescriptor, Value>,  // Takes Value, not MutationExample
    ) -> Value {
        match signature {
            VariantSignature::Unit => {
                json!(variant_name)
            }
            VariantSignature::Tuple(types) => {
                let mut tuple_values = Vec::new();
                for index in 0..types.len() {
                    let descriptor = MutationPathDescriptor::from(index.to_string());
                    let value = children.get(&descriptor)
                        .cloned()  // Just clone the Value directly
                        .unwrap_or(json!(null));
                    tuple_values.push(value);
                }
                json!({ variant_name: tuple_values })
            }
            VariantSignature::Struct(field_types) => {
                let mut field_values = serde_json::Map::new();
                for (field_name, _) in field_types {
                    let descriptor = MutationPathDescriptor::from(field_name.clone());
                    let value = children.get(&descriptor)
                        .cloned()  // Just clone the Value directly
                        .unwrap_or(json!(null));
                    field_values.insert(field_name.clone(), value);
                }
                json!({ variant_name: field_values })
            }
        }
    }

    /// Create a concrete example value for embedding in a parent structure
    fn concrete_example(
        &self,
        variant_groups: &[(VariantSignature, Vec<EnumVariant>)],
        children: &HashMap<MutationPathDescriptor, Value>,  // Takes Value, not MutationExample
    ) -> Value {
        // Pick first unit variant if available, otherwise first example
        let unit_variant = variant_groups.iter()
            .find(|(sig, _)| matches!(sig, VariantSignature::Unit))
            .and_then(|(_, variants)| variants.first());

        if let Some(variant) = unit_variant {
            return json!(variant.name());
        }

        // Fall back to first available example with full structure
        variant_groups.iter()
            .next()
            .map(|(sig, variants)| {
                let representative = variants.first().unwrap();
                self.build_variant_example(sig, representative.name(), children)
            })
            .unwrap_or(json!(null))
    }

    /// Separator used for flattening nested enum variant chains into dot notation
    /// e.g., "Nested.Conditional" for nested enum variants
    const VARIANT_PATH_SEPARATOR: &str = ".";

    /// Flatten variant chain into dot notation for nested enums
    fn flatten_variant_chain(variant_chain: &[(BrpTypeName, Vec<String>)]) -> Vec<String> {
        // e.g., [(TestEnum, ["Nested"]), (NestedEnum, ["Conditional"])] → ["Nested.Conditional"]
        if variant_chain.is_empty() {
            return vec![];
        }

        // Only return the variants from the last level in the chain
        if let Some((_, last_variants)) = variant_chain.last() {
            let prefix_parts: Vec<String> = variant_chain.iter()
                .take(variant_chain.len() - 1)
                .filter_map(|(_, v)| v.first().cloned())
                .collect();

            if prefix_parts.is_empty() {
                last_variants.clone()
            } else {
                last_variants.iter()
                    .map(|v| {
                        let mut full_path = prefix_parts.clone();
                        full_path.push(v.clone());
                        full_path.join(VARIANT_PATH_SEPARATOR)
                    })
                    .collect()
            }
        } else {
            vec![]
        }
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,  // Takes Value like all other builders
    ) -> Result<Value> {  // Returns Value like all other builders
        let schema = ctx.require_registry_schema()?;
        let all_variants = extract_enum_variants(schema, &ctx.registry);
        let variant_groups = group_variants_by_signature(all_variants);

        // Build internal MutationExample to organize the enum logic
        let mutation_example = match ctx.enum_context {
            Some(EnumContext::Root) => {
                // Build examples array for enum root path
                let mut examples = Vec::new();

                for (signature, variants_in_group) in &variant_groups {
                    let representative = variants_in_group.first().ok_or_else(|| {
                        Report::new(Error::InvalidState("Empty variant group".to_string()))
                    })?;

                    let example = self.build_variant_example(
                        signature,
                        representative.name(),
                        &children
                    );

                    let applicable_variants: Vec<String> = variants_in_group
                        .iter()
                        .map(|v| v.name().to_string())
                        .collect();

                    examples.push(ExampleGroup {
                        applicable_variants,
                        signature: signature.clone(),
                        example,
                    });
                }

                MutationExample::EnumRoot(examples)
            }

            Some(EnumContext::Child { ref variant_chain }) => {
                // Building under another enum - return EnumChild
                let example = self.concrete_example(&variant_groups, &children);
                let applicable_variants = Self::flatten_variant_chain(variant_chain);

                MutationExample::EnumChild {
                    example,
                    applicable_variants,
                }
            }

            None => {
                // Parent is not an enum - return a concrete example
                let example = self.concrete_example(&variant_groups, &children);
                MutationExample::Simple(example)
            }
        };

        // Convert MutationExample to Value for ProtocolEnforcer to process
        match mutation_example {
            MutationExample::Simple(val) => Ok(val),
            MutationExample::EnumRoot(examples) => {
                // For enum roots, return both examples array and a default concrete value
                // ProtocolEnforcer will extract these to populate MutationPathInternal fields
                let default_example = examples.first()
                    .and_then(|g| g.get("example"))
                    .cloned()
                    .unwrap_or(json!(null));

                Ok(json!({
                    "enum_root_data": {
                        "examples": examples,
                        "default": default_example
                    }
                }))
            }
            MutationExample::EnumChild { example, applicable_variants } => {
                // For enum children, just return the example
                // ProtocolEnforcer will wrap it based on EnumContext
                Ok(example)
            }
        }
    }
}
```

#### Keep Original assemble_from_children Signature

All builders EXCEPT enum builder keep their current signature:
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,  // Stays as Value
) -> Result<Value>  // Stays as Result<Value>
```

Only the enum builder needs special handling internally.

#### Internal MutationExample Helper Method (within new_enum_builder.rs only)

```rust
// This is INTERNAL to new_enum_builder.rs, not exposed outside
impl MutationExample {
    /// Extract a concrete value suitable for embedding in a parent
    fn concrete_value(&self) -> Value {
        match self {
            MutationExample::Simple(val) => val.clone(),
            MutationExample::EnumRoot(groups) => {
                // Type-safe: Pick first unit variant, or first example
                groups.iter()
                    .find(|g| matches!(g.signature, VariantSignature::Unit))
                    .or_else(|| groups.first())
                    .map(|g| g.example.clone())
                    .unwrap_or(json!(null))
            }
            MutationExample::EnumChild { example, .. } => example.clone(),
        }
    }
}
```

### 5. Protocol Enforcer Updates ⚠️ PARTIALLY COMPLETED

**Objective:** Add EnumContext handling and create_mutation_path_internal method

**Files modified:**
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

**Build:** `cargo build && cargo +nightly fmt`
**Dependencies:** Requires Steps 1, 3, 4

#### What WAS Actually Implemented:

1. **EnumContext::Root extraction logic** (in `ProtocolEnforcer::build_paths`):
   - Checks for `EnumContext::Root` in current context
   - Extracts "enum_root_data" marker from assembled example
   - Deserializes examples array into `Vec<ExampleGroup>`
   - Separates default example and enum_root_examples

2. **Child enum detection** (in `ProtocolEnforcer::process_all_children`):
   - Detects when a child type is an enum
   - Sets `EnumContext::Root` on child context for enum children
   - Allows enum children to process their own variants

3. **Helper method for enum examples**:
   - Added `build_mutation_path_internal_with_enum_examples()`
   - Properly passes `enum_root_examples` field to `MutationPathInternal`
   - Wraps existing `build_mutation_path_internal()` method

### 5a. Documentation: What Was NOT Done & Critical Missing Pieces

#### What Was NOT Implemented from Original Step 5:

1. **EnumContext::Child handling** (from original Step 5 specification):
   - Should wrap examples with applicable_variants
   - Should use variant chain flattening
   - Currently this case is completely ignored in the code
   - The `final_example` doesn't get wrapped with variant context

2. **The unified create_mutation_path_internal method**:
   - Plan showed a single method handling all three cases (Root/Child/None)
   - Current implementation spreads logic across build_paths without clean separation
   - Missing the variant chain flattening for EnumContext::Child

3. **PathKind::EnumVariant expansion logic**:
   - No code to detect and expand `PathKind::EnumVariant`
   - Should expand grouped variants into individual child paths
   - Should set EnumContext::Child with variant information

#### CRITICAL MISSING INFRASTRUCTURE:

**The audit found these CRITICAL MISSING PIECES:**

1. **`PathKind::EnumVariant` doesn't exist at all** - See "Case Analysis for Enum Handling" section
2. **Enum builder returns WRONG types** - Currently returns flat `IndexedElement`/`StructField` instead of `PathKind::EnumVariant`
3. **ProtocolEnforcer has NO variant handling** - Missing expansion logic for EnumVariant → child paths
4. **EnumContext::Child is NEVER set** - Variant chains are never created
5. **Step 6 not started** - `enum_root_examples` field is populated but IGNORED by conversion logic

#### EXACT CODE FLOW AND METHOD USAGE

**WHERE `type_name()` IS ACTUALLY USED:**
```rust
// In ProtocolEnforcer::process_child:
let child_type = child_ctx.type_name();  // <-- CALLED HERE
let child_kind = TypeKind::from_schema(child_schema, child_type);
```
For `PathKind::EnumVariant`, this needs to return the enum type so TypeKind can identify it as an enum.

**WHERE `to_mutation_path_descriptor()` IS ACTUALLY USED:**
```rust
// In ProtocolEnforcer::process_all_children:
let child_key = path_kind.to_mutation_path_descriptor();  // <-- CALLED HERE
let (child_paths, child_example) = Self::process_child(&child_key, &child_ctx, depth)?;
child_examples.insert(child_key, child_example);
```
**PROBLEM:** EnumVariant represents MULTIPLE paths, not one. ProtocolEnforcer must detect EnumVariant BEFORE this line and expand it.

**WHERE `description()` IS ACTUALLY USED:**
```rust
// In MutationPath::from_mutation_path_internal:
let description = path.path_kind.description(&type_kind);  // <-- CALLED HERE
```
Used for generating human-readable descriptions in the final output.

#### IMPLEMENTATION REQUIREMENTS

**PathKind::EnumVariant must have:**
```rust
EnumVariant {
    signature: VariantSignature,       // Determines what children to create
    applicable_variants: Vec<String>,  // Goes into variant_chain
    parent_type: BrpTypeName,         // The enum type
}
```

**ProtocolEnforcer must change `process_all_children()` to:**
```rust
for path_kind in child_path_kinds {
    // NEW: Check if this is EnumVariant BEFORE normal processing
    if let PathKind::EnumVariant { signature, applicable_variants, parent_type } = &path_kind {
        // EXPAND EnumVariant into multiple child paths
        match signature {
            VariantSignature::Unit => {
                // No children, but still need to track for examples
            }
            VariantSignature::Tuple(types) => {
                for (index, type_name) in types.iter().enumerate() {
                    // Create IndexedElement with EnumContext::Child
                    let expanded_path = PathKind::IndexedElement { index, type_name, parent_type };
                    let mut child_ctx = ctx.create_recursion_context(expanded_path, ...);
                    child_ctx.enum_context = Some(EnumContext::Child {
                        variant_chain: vec![(parent_type.clone(), applicable_variants.clone())]
                    });
                    // Process this child...
                }
            }
            VariantSignature::Struct(fields) => {
                for (field_name, type_name) in fields {
                    // Create StructField with EnumContext::Child
                    let expanded_path = PathKind::StructField { field_name, type_name, parent_type };
                    let mut child_ctx = ctx.create_recursion_context(expanded_path, ...);
                    child_ctx.enum_context = Some(EnumContext::Child {
                        variant_chain: vec![(parent_type.clone(), applicable_variants.clone())]
                    });
                    // Process this child...
                }
            }
        }
    } else {
        // Normal path processing
    }
}
```

#### ENUM BUILDER CHANGES NEEDED

**Current WRONG implementation in `NewEnumMutationBuilder::collect_children`:**
```rust
// WRONG - creates flat paths
children.push(PathKind::IndexedElement { index, type_name, parent_type });
children.push(PathKind::StructField { field_name, type_name, parent_type });
```

**CORRECT implementation needed:**
```rust
// Group variants by signature
let grouped = group_variants_by_signature(variants);
for (signature, variants_in_group) in grouped {
    let applicable_variants = variants_in_group.iter().map(|v| v.name()).collect();
    children.push(PathKind::EnumVariant {
        signature,
        applicable_variants,
        parent_type: ctx.type_name().clone(),
    });
}
```

#### VARIANT CHAIN FOR NESTED ENUMS

**When already in EnumContext::Child and encountering another enum:**
```rust
// Current context: Some(EnumContext::Child { variant_chain: [(TestEnum, ["Nested"])] })
// Child is NestedConfigEnum (another enum)

// Must EXTEND the chain, not replace:
child_ctx.enum_context = Some(EnumContext::Child {
    variant_chain: existing_chain.clone().extend_with((NestedConfigEnum, ["Conditional"]))
});
// Results in: [(TestEnum, ["Nested"]), (NestedConfigEnum, ["Conditional"])]
```

#### DOT NOTATION FLATTENING

**The function exists in `NewEnumMutationBuilder::flatten_variant_chain` but needs to be called:**
- In `assemble_from_children()` when processing `EnumContext::Child`
- Converts `[(TestEnum, ["Nested"]), (NestedConfigEnum, ["Conditional"])]` → `["Nested.Conditional"]`

#### CRITICAL FILES TO MODIFY

1. `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_kind.rs`
   - Add `EnumVariant` variant to enum
   - Implement required methods

2. `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/new_enum_builder.rs`
   - Fix `collect_children()` to return `PathKind::EnumVariant`
   - Ensure `flatten_variant_chain()` is called properly

3. `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`
   - Modify `process_all_children()` to detect and expand `PathKind::EnumVariant`
   - Set `EnumContext::Child` with proper `variant_chain`

#### THE KEY INSIGHT

`PathKind::EnumVariant` is a **grouped representation** that carries variant information. ProtocolEnforcer **expands** it into regular PathKinds (IndexedElement/StructField) while setting the proper `EnumContext::Child` with `variant_chain`. This allows all children of a variant to know which variant(s) they belong to, enabling proper `applicable_variants` tracking through the entire recursion tree.

**WITHOUT THIS**: The system cannot track which paths belong to which variants, making the entire enum example system broken.

### 5b. Implementation Plan: Completing Step 5

This section outlines the specific code changes needed to properly complete Step 5.

#### 1. Add PathKind::EnumVariant variant

**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_kind.rs`

Add the new variant to the `PathKind` enum:
```rust
pub enum PathKind {
    // ... existing variants ...

    /// Represents a group of enum variants with the same signature
    EnumVariant {
        /// The signature shared by these variants (Unit/Tuple/Struct)
        signature: VariantSignature,
        /// The names of all variants that share this signature
        applicable_variants: Vec<String>,
        /// The enum type these variants belong to
        parent_type: BrpTypeName,
    },
}
```

#### 2. Fix NewEnumMutationBuilder::collect_children

**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/new_enum_builder.rs`

Replace the current implementation that creates flat paths with one that returns PathKind::EnumVariant:

```rust
impl MutationPathBuilder for NewEnumMutationBuilder {
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        let schema = ctx.require_registry_schema()?;

        // Extract all variants from schema
        let variants = extract_enum_variants(schema, &ctx.registry);

        // Group variants by their signature (already handles deduplication)
        let variant_groups = group_variants_by_signature(variants);

        let mut children = Vec::new();

        // Create ONE PathKind::EnumVariant per signature group
        for (signature, variants_in_group) in variant_groups {
            let applicable_variants: Vec<String> = variants_in_group
                .iter()
                .map(|v| v.name().to_string())
                .collect();

            children.push(PathKind::EnumVariant {
                signature,
                applicable_variants,
                parent_type: ctx.type_name().clone(),
            });
        }

        Ok(children)
    }
}
```

**Key changes:**
- No more `deduplicate_variant_signatures` needed - grouping handles it
- Returns `PathKind::EnumVariant` instead of `IndexedElement`/`StructField`
- Preserves ALL variant names in `applicable_variants`
- ProtocolEnforcer will expand these into actual paths

#### 3. Add PathKind::EnumVariant expansion in ProtocolEnforcer

**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

Modify `process_all_children` to detect and expand `PathKind::EnumVariant`:

```rust
fn process_all_children(
    &self,
    ctx: &RecursionContext,
    depth: RecursionDepth,
) -> Result<ChildProcessingResult> {
    let child_path_kinds = self.inner.collect_children(ctx)?;
    let mut all_paths = vec![];
    let mut paths_to_expose = vec![];
    let mut child_examples = HashMap::<MutationPathDescriptor, Value>::new();

    for path_kind in child_path_kinds {
        // Check if this is EnumVariant and expand it
        if let PathKind::EnumVariant { signature, applicable_variants, parent_type } = path_kind {
            // Expand EnumVariant into actual child paths
            let expanded_paths = Self::expand_enum_variant(&signature, &parent_type);

            // Process each expanded path with EnumContext::Child
            for expanded_path in expanded_paths {
                let mut child_ctx = ctx.create_recursion_context(
                    expanded_path.clone(),
                    self.inner.child_path_action()
                );

                // Set EnumContext::Child with variant information
                child_ctx.enum_context = Some(EnumContext::Child {
                    variant_chain: vec![(parent_type.clone(), applicable_variants.clone())]
                });

                // Process using common logic
                Self::process_and_collect_child(
                    &expanded_path,
                    &child_ctx,
                    depth,
                    &mut child_examples,
                    &mut all_paths,
                    &mut paths_to_expose
                )?;
            }
        } else {
            // Normal path processing (existing code stays the same)
            // ...
        }
    }

    Ok(ChildProcessingResult {
        all_paths,
        paths_to_expose,
        child_examples,
    })
}
```

Add helper methods:

```rust
/// Helper to expand EnumVariant into concrete PathKinds
fn expand_enum_variant(signature: &VariantSignature, parent_type: &BrpTypeName) -> Vec<PathKind> {
    match signature {
        VariantSignature::Unit => {
            // Unit variants have no fields to expand
            vec![]
        }
        VariantSignature::Tuple(types) => {
            types.iter()
                .enumerate()
                .map(|(index, type_name)| PathKind::IndexedElement {
                    index,
                    type_name: type_name.clone(),
                    parent_type: parent_type.clone(),
                })
                .collect()
        }
        VariantSignature::Struct(fields) => {
            fields.iter()
                .map(|(field_name, type_name)| PathKind::StructField {
                    field_name: field_name.clone(),
                    type_name: type_name.clone(),
                    parent_type: parent_type.clone(),
                })
                .collect()
        }
    }
}

/// Helper to process a child and collect its results
fn process_and_collect_child(
    path_kind: &PathKind,
    child_ctx: &RecursionContext,
    depth: RecursionDepth,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
    all_paths: &mut Vec<MutationPathInternal>,
    paths_to_expose: &mut Vec<MutationPathInternal>,
) -> Result<()> {
    let child_key = path_kind.to_mutation_path_descriptor();
    let (child_paths, child_example) = Self::process_child(&child_key, child_ctx, depth)?;

    child_examples.insert(child_key, child_example);
    all_paths.extend(child_paths.clone());

    if matches!(child_ctx.path_action, PathAction::Create) {
        paths_to_expose.extend(child_paths);
    }

    Ok(())
}
```

**Key changes:**
- Detects `PathKind::EnumVariant` before normal processing
- Expands it into `IndexedElement`/`StructField` based on signature
- Sets `EnumContext::Child` with variant_chain for expanded paths
- Uses helper methods to avoid code duplication

#### 4. Handle EnumContext::Child in ProtocolEnforcer::build_paths

**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

Fix the EnumContext::Child case that is currently ignored:

```rust
// In build_paths method, replace the example processing section:
let (parent_example, enum_root_examples) = match &ctx.enum_context {
    Some(EnumContext::Root) => {
        // Check if the assembled_example contains enum_root_data marker
        if let Some(enum_data) = assembled_example.get("enum_root_data") {
            let default_example = enum_data.get("default").cloned().unwrap_or(json!(null));
            let examples_json = enum_data.get("examples").cloned().unwrap_or(json!([]));
            let examples: Vec<ExampleGroup> =
                serde_json::from_value(examples_json).unwrap_or_default();
            (default_example, Some(examples))
        } else {
            // Fallback if structure is unexpected
            (assembled_example, None)
        }
    }
    Some(EnumContext::Child { variant_chain }) => {
        // ProtocolEnforcer handles ALL flattening
        let applicable_variants = Self::flatten_variant_chain(variant_chain);

        let wrapped = json!({
            "value": assembled_example,
            "applicable_variants": applicable_variants
        });
        (wrapped, None)
    }
    None => {
        // Regular non-enum types pass through unchanged
        (assembled_example, None)
    }
};
```

Add the flatten helper method to ProtocolEnforcer:

```rust
impl ProtocolEnforcer {
    /// Flatten variant chain into dot notation for nested enums
    fn flatten_variant_chain(variant_chain: &[(BrpTypeName, Vec<String>)]) -> Vec<String> {
        if variant_chain.is_empty() {
            return vec![];
        }

        // Only return the variants from the last level in the chain
        if let Some((_, last_variants)) = variant_chain.last() {
            let prefix_parts: Vec<String> = variant_chain
                .iter()
                .take(variant_chain.len() - 1)
                .filter_map(|(_, v)| v.first().cloned())
                .collect();

            if prefix_parts.is_empty() {
                last_variants.clone()
            } else {
                last_variants
                    .iter()
                    .map(|v| {
                        let mut full_path = prefix_parts.clone();
                        full_path.push(v.clone());
                        full_path.join(".")
                    })
                    .collect()
            }
        } else {
            vec![]
        }
    }
}
```

**Note:** The `flatten_variant_chain` method should be REMOVED from `NewEnumMutationBuilder` since ProtocolEnforcer now owns all path flattening logic.

**Key changes:**
- Handles `EnumContext::Child` case properly
- Wraps example with `applicable_variants`
- Moves `flatten_variant_chain` to ProtocolEnforcer where it belongs
- ProtocolEnforcer owns ALL path handling logic

#### 5. Add variant chain extension for nested enums

**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/protocol_enforcer.rs`

Modify the EnumVariant expansion to extend variant chains for nested enums:

```rust
// In process_all_children, when processing expanded paths from EnumVariant:
for expanded_path in expanded_paths {
    let mut child_ctx = ctx.create_recursion_context(
        expanded_path.clone(),
        self.inner.child_path_action()
    );

    // Build variant chain - either new or extended
    let variant_chain = match &ctx.enum_context {
        Some(EnumContext::Child { variant_chain: parent_chain }) => {
            // We're already in a variant - extend the chain
            let mut extended = parent_chain.clone();
            extended.push((parent_type.clone(), applicable_variants.clone()));
            extended
        }
        _ => {
            // Start a new chain
            vec![(parent_type.clone(), applicable_variants.clone())]
        }
    };

    child_ctx.enum_context = Some(EnumContext::Child { variant_chain });

    // Process using common logic
    Self::process_and_collect_child(
        &expanded_path,
        &child_ctx,
        depth,
        &mut child_examples,
        &mut all_paths,
        &mut paths_to_expose
    )?;
}
```

Also update the normal path processing to preserve chains:

```rust
// In the else branch for normal path processing:
if matches!(child_type_kind, TypeKind::Enum) {
    // This child is an enum
    match &ctx.enum_context {
        Some(EnumContext::Child { .. }) => {
            // We're in a variant and found a nested enum
            // The nested enum gets Root context (to generate examples)
            // The chain will be extended when this enum's variants are expanded
            child_ctx.enum_context = Some(EnumContext::Root);
        }
        _ => {
            // Normal enum as field - gets Root context
            child_ctx.enum_context = Some(EnumContext::Root);
        }
    }
}
```

**Key changes:**
- Extends variant chains when expanding EnumVariant inside another enum's variant
- Preserves parent context when encountering nested enums
- Results in proper chains like `[(TestEnum, ["Nested"]), (NestedConfigEnum, ["Conditional"])]`
- Flattening to dot notation happens later in `flatten_variant_chain`

#### 6. Add PathKind::EnumVariant match arms

**File:** `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_kind.rs`

Add match arms for EnumVariant in all existing methods:

```rust
impl PathKind {
    /// Get the type name being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        match self {
            Self::RootValue { type_name }
            | Self::StructField { type_name, .. }
            | Self::IndexedElement { type_name, .. }
            | Self::ArrayElement { type_name, .. } => type_name,
            Self::EnumVariant { parent_type, .. } => parent_type,
        }
    }

    /// Extract a descriptor suitable for HashMap
    pub fn to_mutation_path_descriptor(&self) -> MutationPathDescriptor {
        match self {
            Self::StructField { field_name, .. } => {
                MutationPathDescriptor::from(field_name.clone())
            }
            Self::IndexedElement { index, .. } | Self::ArrayElement { index, .. } => {
                MutationPathDescriptor::from(index.to_string())
            }
            Self::RootValue { .. } => MutationPathDescriptor::from(String::new()),
            Self::EnumVariant { .. } => {
                // Should never be called - EnumVariant is expanded before processing
                unreachable!("EnumVariant should be expanded before descriptor is needed")
            }
        }
    }

    /// Generate a human-readable description
    pub fn description(&self, type_kind: &TypeKind) -> String {
        // ... existing logic ...
        match self {
            // ... existing cases ...
            Self::EnumVariant { .. } => {
                // Should never be called - EnumVariant is expanded before processing
                unreachable!("EnumVariant should be expanded before description is needed")
            }
        }
    }

    /// Get just the variant name for serialization
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::RootValue { .. } => "RootValue",
            Self::StructField { .. } => "StructField",
            Self::IndexedElement { .. } => "IndexedElement",
            Self::ArrayElement { .. } => "ArrayElement",
            Self::EnumVariant { .. } => "EnumVariant",
        }
    }
}
```

**Notes:**
- `type_name()`: Returns `parent_type` since that's the enum type
- `to_mutation_path_descriptor()`: Uses `unreachable!()` since expansion happens first
- `description()`: Uses `unreachable!()` since expansion happens first
- `variant_name()`: Returns "EnumVariant" for consistency

### 6. Conversion Logic Simplification ⏳ PENDING

**Objective:** Update from_mutation_path_internal, remove JSON parsing

**Files to modify:**
- `/Users/natemccoy/rust/bevy_brp/mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`

**Build:** `cargo build && cargo +nightly fmt`
**Dependencies:** Requires Steps 1, 3, 5

**IMPORTANT - TEMPORARY FIXES FROM STEP 1 TO REMOVE:**
- `MutationPath::from_mutation_path_internal()`: Currently returns empty vec instead of creating proper ExampleGroup for non-array cases
- `MutationPath::from_mutation_path_internal()`: Currently uses `applicable_variants.is_empty()` check, needs proper logic for new ExampleGroup structure
- `MutationPath::convert_signature_groups_array()`: Currently creates dummy VariantSignature values, needs proper parsing of signature strings

#### How to Fix the Temporary Hacks:

1. **Replace entire `from_mutation_path_internal()` method** with the simplified version below that uses the new `enum_root_examples` field from MutationPathInternal (added in Step 3)

2. **Delete `convert_signature_groups_array()` entirely** - it won't be needed anymore because the ExampleGroup objects will come directly from MutationPathInternal

3. **Remove all the old JSON parsing logic** including:
   - The `__variant_context` extraction code
   - The `clean_example` processing
   - The complex if-else chain for determining examples vs example

#### Simplified Conversion Logic

The conversion from `MutationPathInternal` to `MutationPath` is now trivial - just direct field transfer:

```rust
impl MutationPath {
    pub fn from_mutation_path_internal(
        path: &MutationPathInternal,
        registry: &HashMap<BrpTypeName, Value>
    ) -> Self {
        // Direct transfer - no JSON parsing needed!
        let (examples, example) = if let Some(ref enum_examples) = path.enum_root_examples {
            // Enum root: use the examples array
            (enum_examples.clone(), None)
        } else {
            // Everything else: use the example value
            // This includes enum children (with embedded applicable_variants) and regular values
            (vec![], Some(path.example.clone()))
        };

        Self {
            description: path.path_kind.description(&path.type_kind),
            path_info: PathInfo {
                path: path.path.clone(),
                type_name: path.type_name.clone(),
                type_kind: path.type_kind,
                mutation_status: path.mutation_status,
                mutation_status_reason: path.mutation_status_reason.clone(),
            },
            // Note: variants field has been DEPRECATED - removed from MutationPath
            examples,
            example,
            note: None,
        }
    }
}
```

**Key improvements:**
- No JSON parsing or magic markers
- Direct field transfer from `MutationPathInternal` to `MutationPath`
- `variants` field removed entirely - information is in `examples` or embedded in `example`
- Clean separation of concerns


#### Final Cleanup Tasks:

1. **Delete old enum_builder.rs file** - No longer needed after successful migration
2. **Remove deprecated fields** (if any remain):
   - Remove `variants` field from `MutationPath` (deprecated)
   - Remove obsolete `enum_info` field from `TypeGuide` struct (redundant with examples array)
3. **Remove obsolete methods**:
   - Remove `TypeGuide::extract_enum_info()` method and related code
4. **Test with existing test suite** to ensure compatibility

**Note:** The major integration work (moving types, updating builders, protocol enforcer changes) is handled in Steps 1-6. This step only handles final cleanup after successful implementation.

### 7. Complete Validation ⏳ PENDING

**Objective:** Run comprehensive test suite and verify no regressions

**Build:** Full test suite execution
**Dependencies:** Requires Steps 1-7

#### Testing Strategy

- Use @get_guide.md to fetch baseline outputs for all test types
- Compare new output structure to ensure:
  - Root enum paths have proper `examples` array
  - Child paths have `variants` field (now `applicable_variants`)
  - Embedded enums show single concrete value (fix the bug)
- Run mutation tests to verify BRP operations still work

**Expected Results:**
- Confirm enum root paths have proper `examples` array
- Verify child paths have `applicable_variants` in example
- Ensure embedded enums show single concrete value (bug fix validation)
- All existing tests pass
- No regressions in BRP functionality

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Example Enum Structures

*These examples are used throughout the case analysis:*

```rust
// Main enum with multiple variants including nested enum
enum TestEnumWithSerDe {
    Active,
    Inactive,
    Special(String, u32),
    AlsoSpecial(String, u32),  // Second variant with same signature
    Custom {
        enabled: bool,
        name: String,
        value: f32,
    },
    Nested {
        nested_config: NestedConfigEnum,
        other_field: String,
    }
}

// Nested enum used in the Nested variant
enum NestedConfigEnum {
    Always,
    Never,
    Conditional(u32),
}
```

This structure demonstrates:
- Unit variants (Active, Inactive)
- Multiple tuple variants with same signature (Special, AlsoSpecial)
- Struct variant (Custom)
- Nested enum (Nested contains NestedConfigEnum)

## Case Analysis for Enum Handling

*This section to be included in the module documentation for new_enum_builder.rs*

### Case 1: Enum at root (e.g., TestEnumWithSerDe itself)

**Context:**
- Building mutation paths for an enum as the top-level type
- Path: `""`
- PathKind: `RootValue`
- Type: `TestEnumWithSerDe` (TypeKind::Enum)
- EnumContext: `Some(EnumContext::Root)` (set by ProtocolEnforcer when it sees TypeKind::Enum)

**Key Insight:** The enum builder knows it's TypeKind::Enum, which determines its behavior.

**Flow:**
1. `collect_children()` returns `Vec<PathKind>` with:
   ```rust
   PathKind::EnumVariant {
       signature: VariantSignature::Unit,
       parent_type: TestEnumWithSerDe,
       applicable_variants: vec!["Active", "Inactive"],
   },
   PathKind::EnumVariant {
       signature: VariantSignature::Tuple(vec![String, u32]),
       parent_type: TestEnumWithSerDe,
       applicable_variants: vec!["Special"],
   },
   PathKind::EnumVariant {
       signature: VariantSignature::Struct(vec![
           ("enabled", bool),
           ("name", String),
           ("value", f32),
       ]),
       parent_type: TestEnumWithSerDe,
       applicable_variants: vec!["Custom"],
   },
   ```

2. ProtocolEnforcer processes these `PathKind::EnumVariant` entries:
   - For Unit: No child paths created
   - For Tuple: Creates `.0`, `.1` paths with `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Special", "AlsoSpecial"])] })`
   - For Struct: Creates `.enabled`, `.name`, `.value` paths with `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Custom"])] })`
   - **This is where EnumContext changes from `Some(EnumContext::Root)` to `Some(EnumContext::Child { ... })`**

3. `assemble_from_children()` receives the built child examples and:
   - Checks: `EnumContext == Some(EnumContext::Root)`
   - Returns `MutationExample::EnumRoot(Vec<ExampleGroup>)`
   - Transforms child examples into the proper examples array structure

**Result:**
- Root path `""` gets `examples` array with all variant groups
- Child paths `.0`, `.1`, `.enabled`, etc. have `applicable_variants` from their EnumContext

### Case 2: Enum as a field (e.g., .mode in TestComplexComponent)

**Context:**
- Building paths for TestComplexComponent's enum field
- Path: `.mode`
- PathKind: `StructField { field_name: "mode", type_name: TestEnumWithSerDe, parent_type: TestComplexComponent }`
- Type: `TestEnumWithSerDe` (TypeKind::Enum)
- EnumContext: `Some(EnumContext::Root)` (set by ProtocolEnforcer when it sees TypeKind::Enum)

**Key Rule:** Any enum type with `EnumContext == Some(EnumContext::Root)` returns `MutationExample::EnumRoot`.

**Flow:**
1. Struct builder delegates to enum builder for the `mode` field
2. ProtocolEnforcer sets `Some(EnumContext::Root)` for this enum field
3. Enum's `collect_children()` returns `PathKind::EnumVariant` entries
4. ProtocolEnforcer creates `.mode.0`, `.mode.1`, `.mode.enabled` with `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Special", "AlsoSpecial"])] })` or `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Custom"])] })`
5. Enum's `assemble_from_children()`:
   - Sees `EnumContext == Some(EnumContext::Root)`
   - Returns `MutationExample::EnumRoot(Vec<ExampleGroup>)`

**Result:** `.mode` gets full `examples` array just like a root enum.

### Case 3: Enum child paths (e.g., .mode.0, .mode.enabled)

**Context:**
- Building paths for fields within enum variants
- Paths: `.mode.0`, `.mode.1` (tuple variant) or `.mode.enabled`, `.mode.name` (struct variant)
- PathKind: Created from `PathKind::EnumVariant` expansion (specific types TBD based on ProtocolEnforcer logic)
- Types: `String`, `u32`, `bool`, etc. (the field types)
- EnumContext: `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, [...])] })`

**Key Points:**
- These are NOT enum types - they're fields inside enum variants
- The `Some(EnumContext::Child { ... })` stays constant throughout this recursion branch because all paths under a variant apply to the same set of variants

**Flow:**
1. These paths were created by ProtocolEnforcer when it processed `PathKind::EnumVariant`
2. The appropriate builder for the field type is called (string, bool, etc.)
3. These builders see `Some(EnumContext::Child { ... })` and preserve it (it stays the same for the entire branch)
4. When assembling:
   - The `example` value changes as we recurse back up (field examples get assembled into parent structures)
   - But `applicable_variants` stays constant for this branch
   - Non-enum builders return `MutationExample::Simple(value)`
   - If the field itself was an enum, it would return `MutationExample::EnumChild { example, applicable_variants }`

**Result:**
- `.mode.0` gets example `"Hello, World!"` with `variants: ["Special"]`
- `.mode.enabled` gets example `true` with `variants: ["Custom"]`
- The `applicable_variants` information comes from the EnumContext, not the example

### Case 4: Nested enum under another enum

**Context:**
- An enum field inside an enum variant (the `nested_config` field in the `Nested` variant)
- Path: `.mode.nested_config`
- PathKind: Created from parent enum's variant processing
- Type: `NestedConfigEnum` (TypeKind::Enum)
- EnumContext: `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Nested"])] })`

**Key Insight:** Nested enums track the full chain of variant constraints using dot notation in output.

**Flow:**
1. `.mode.nested_config` is processed:
   - ProtocolEnforcer sees TypeKind::Enum
   - Context has parent constraint in variant_chain
   - Sets `Some(EnumContext::Root)` for the nested enum itself (to get examples array)
   - But preserves parent constraints for its children

2. Nested enum's `collect_children()` returns its own `PathKind::EnumVariant` entries

3. For `.mode.nested_config.0` (the u32 in Conditional):
   - Context becomes `Some(EnumContext::Child { variant_chain: [(TestEnumWithSerDe, ["Nested"]), (NestedConfigEnum, ["Conditional"])] })`

**Result:**
- `.mode.nested_config` gets its own `examples` array with `applicable_variants: ["Nested"]`
- `.mode.nested_config.0` gets `applicable_variants: ["Nested.Conditional"]` (flattened with dot notation)
- The conversion flattens the chain: `["Nested", "Conditional"]` → `"Nested.Conditional"`

### Case 5: Providing value for parent assembly

**Context:**
- A parent struct/array/tuple needs a concrete enum value for its root path assembly
- Example: TestComplexComponent needs a value for its `mode` field when building root `""`
- EnumContext: `None` (no enum context established)

**Key Insight:** When enum's `assemble_from_children` sees `enum_context: None`, it knows a non-enum parent needs a concrete value.

**Flow:**
1. TestComplexComponent's struct builder assembles its root path
2. It calls enum's `assemble_from_children` for the `mode` field with `enum_context: None`
3. Enum's `assemble_from_children` logic:
   ```rust
   match ctx.enum_context {
       None => MutationExample::Simple(pick_concrete_value()),  // Parent needs a value
       Some(EnumContext::Root) => MutationExample::EnumRoot(examples),
       Some(EnumContext::Child { .. }) => MutationExample::EnumChild { .. },
   }
   ```

**Result:**
- TestComplexComponent's root gets `mode: "Active"` (concrete value)
- Clean separation: `None` means "give me something concrete to embed"
- The `.mode` path itself still gets its full `examples` array when processed as a field

## Risks and Mitigations

- **Risk**: Marker-based communication could be fragile
  - **Mitigation**: Use unique marker names like `__enum_root_marker` that won't collide
  - **Mitigation**: Keep markers as internal implementation details

- **Risk**: Missing some edge case in conversion
  - **Mitigation**: Comprehensive testing against baseline outputs
  - **Mitigation**: Conversion logic handles both old and new formats during transition

- **Risk**: Performance impact from internal enum construction
  - **Mitigation**: Minimal overhead - enum is built and immediately converted
  - **Mitigation**: Still more efficient than current JSON parsing approach

## Implementation Order

1. **Phase 1: Add internal types to enum builder**
   - Add `MutationExample` enum as private type in new_enum_builder.rs
   - Add helper methods for variant example building
   - Keep the public interface unchanged (still returns `Value`)

2. **Phase 2: Add public types to types.rs**
   - Add `ExampleGroup` struct (needed for API)
   - Add `VariantSignature` enum with `Display` trait
   - Add `EnumContext` to recursion_context.rs

3. **Phase 3: Update enum builder implementation**
   - Modify `assemble_from_children` to use internal `MutationExample`
   - Convert to `Value` with markers before returning
   - Test enum builder in isolation

4. **Phase 4: Update conversion logic**
   - Modify `from_mutation_path_internal` to recognize markers
   - Handle both old and new formats for compatibility
   - Verify with existing tests

5. **Phase 5: Activate and cleanup**
   - Remove `USE_NEW_ENUM_BUILDER` flag from type_kind.rs
   - Remove old enum_builder.rs file
   - Run full test suite to ensure no regressions
