# Refactor: Convert EnumVariantKind from Enum to Struct

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol below defines the phase boundaries with validation checkpoints between each step.

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
   cargo build
   ```

   For testing steps:
   ```bash
   /create_mutation_test_json
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

### Step 1: Atomic Refactoring - Convert EnumVariantKind to Struct ✅ COMPLETED

**Objective**: Convert `EnumVariantKind` from enum to struct, eliminating type information redundancy between `EnumVariantKind` and `VariantSignature`.

**Why**: Currently `EnumVariantKind` (enum) and `VariantSignature` (enum) duplicate type structure information. Converting to a struct that composes name + signature eliminates this redundancy.

**Changes**:
- **Section 1**: Type Definition (enum → struct with name + signature fields)
- **Section 2**: Implementation Methods (update variant_name(), name(); remove signature() and short_name())
- **Section 3**: Schema Extraction (complete rewrite of from_schema_variant())
- **Section 3.5**: Helper Function Architectural Change (transform complete constructors → signature extractors)
- **Section 4**: Helper Functions (rename/update extract_tuple_variant_signature and extract_struct_variant_signature)
- **Section 5**: Remove EnumFieldInfo struct
- **Section 6**: Update group_variants_by_signature() to use direct field access
- **Section 7**: Verify no .signature() method calls remain

**Files to Modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Change Type**: Breaking (enum → struct conversion breaks all pattern matching)

**Build Command**:
```bash
cargo build
```

**Notes**: This is a single atomic change - all modifications must be made together to avoid compilation breakage. The enum-to-struct conversion breaks all pattern matching, so we must update type definition, all methods, schema extraction, helper functions, and call sites in one cohesive edit.

---

### Step 2: Verification & Testing ✅ COMPLETED

**Objective**: Verify refactoring correctness and behavior preservation through comprehensive testing.

**Why**: Ensures the structural refactoring maintains identical behavior and catches any edge cases.

**Tasks**:
1. Run mutation test generation to verify output
2. Confirm identical output (241 types, 2200 paths)
3. Run validation commands to check for remnants
4. Verify build succeeds with no warnings

**Build Command**:
```bash
/create_mutation_test_json
```

**Validation Commands**:
```bash
# Check no pattern matching remains on EnumVariantKind
rg "Self::(Unit|Tuple|Struct)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs

# Verify no orphaned EnumFieldInfo references
rg "EnumFieldInfo" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs

# Check all construction uses struct literals
rg "EnumVariantKind::(Unit|Tuple|Struct)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs

# Verify no .signature() method calls remain
rg "\.signature\(\)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs
```

**Success Criteria**:
- All tests pass
- Mutation test generates identical output (zero changes)
- No compilation errors or warnings
- All validation commands return **no matches**

---

## Problem

`EnumVariantKind` and `VariantSignature` contain redundant type information. Currently:

- **`EnumVariantKind`** (enum): Stores variant name + type structure
  - `Unit(VariantName)`
  - `Tuple(VariantName, Vec<BrpTypeName>)`
  - `Struct(VariantName, Vec<EnumFieldInfo>)`

- **`VariantSignature`** (enum): Stores just the type structure
  - `Unit`
  - `Tuple(Vec<BrpTypeName>)`
  - `Struct(Vec<(StructFieldName, BrpTypeName)>)`

The type structure information (tuple types, struct fields) is duplicated. `EnumVariantKind::signature()` extracts the signature part, discarding the name.

## Solution

Convert `EnumVariantKind` from an enum to a struct that composes a name with a signature:

```rust
struct EnumVariantKind {
    name: VariantName,
    signature: VariantSignature,
}
```

This eliminates the redundancy by storing the structure information once in the `signature` field.

## Changes Required

### 1. Type Definition - EnumVariantKind in enum_path_builder.rs

**Before**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
enum EnumVariantKind {
    Unit(VariantName),
    Tuple(VariantName, Vec<BrpTypeName>),
    Struct(VariantName, Vec<EnumFieldInfo>),
}
```

**After**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnumVariantKind {
    name: VariantName,
    signature: VariantSignature,
}
```

### 2. Implementation Methods - EnumVariantKind impl block

**Update `variant_name()`**:
```rust
const fn variant_name(&self) -> &VariantName {
    &self.name
}
```

**Update `name()` method** (consolidate short_name into name):
```rust
fn name(&self) -> &str {
    self.name
        .as_str()
        .rsplit_once("::")
        .map_or_else(|| self.name.as_str(), |(_, name)| name)
}
```

**Note**: Remove `short_name()` method - it's redundant. The `name()` method is more semantically clear (extracts the variant name without enum prefix) and is already the preferred method in the codebase (3 call sites vs 0 direct calls to short_name).

**Remove `signature()` method**: Delete the entire method. Use direct field access `variant.signature` instead.

**Rationale**: Since `EnumVariantKind` will have public fields and is a private internal struct, direct field access is idiomatic. The codebase pattern (see `RecursionContext`, `MutationPathInternal`, `ExampleGroup`) strongly favors public fields with direct access over trivial getters. The method has only one call site, making this change trivial.

### 3. Schema Extraction - from_schema_variant() method

**Complete rewrite of from_schema_variant()**:

The current implementation uses pattern matching on enum variants. Need to rewrite to construct the struct:

```rust
fn from_schema_variant(
    v: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    enum_type: &BrpTypeName,
) -> Option<Self> {
    // Handle Unit variants which show up as simple strings
    if let Some(variant_str) = v.as_str() {
        let type_name = enum_type
            .as_str()
            .rsplit("::")
            .next()
            .unwrap_or(enum_type.as_str());

        let qualified_name = format!("{type_name}::{variant_str}");
        return Some(Self {
            name: VariantName::from(qualified_name),
            signature: VariantSignature::Unit,
        });
    }

    // Extract the fully qualified variant name
    let variant_name = extract_variant_qualified_name(v)?;

    // Check what type of variant this is
    if let Some(signature) = extract_tuple_variant_signature(v, registry) {
        return Some(Self {
            name: variant_name,
            signature,
        });
    }

    if let Some(signature) = extract_struct_variant_signature(v, registry) {
        return Some(Self {
            name: variant_name,
            signature,
        });
    }

    // Unit variant (no fields)
    Some(Self {
        name: variant_name,
        signature: VariantSignature::Unit,
    })
}
```

**Note on `extract_variant_qualified_name()`**: This function already exists in the file and returns `Option<VariantName>`. It does **NOT** need any changes - the rewritten `from_schema_variant()` can use it as-is.

### 3.5. Critical Transformation Details: Helper Function Architectural Change

**IMPORTANT**: This is NOT a simple rename. These helper functions undergo a complete architectural transformation:

#### Responsibility Separation

**OLD (lines 153-195)**: Helper functions are **complete constructors**
- Purpose: Build entire `EnumVariantKind` enum variants
- Input: Schema data + variant name
- Output: Complete `EnumVariantKind::Tuple(name, signature)` or `EnumVariantKind::Struct(name, fields)`
- Role: Single function builds the complete variant object

**NEW (plan lines 140-186)**: Helper functions become **signature extractors**
- Purpose: Extract ONLY the signature information from schema
- Input: Schema data (no variant name)
- Output: ONLY the `VariantSignature` component
- Role: Extract signature component; caller assembles with name into struct

#### Function Signature Transformations

**Tuple Helper Transformation**:
```rust
// BEFORE: Complete constructor
fn extract_tuple_variant_kind(
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
    variant_name: VariantName,              // ← Parameter 3: variant name
) -> Option<EnumVariantKind>                // ← Returns complete variant

// AFTER: Signature extractor
fn extract_tuple_variant_signature(        // ← New function name
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
    // ← Parameter 3 REMOVED (no variant name)
) -> Option<VariantSignature>               // ← Returns ONLY signature
```

**Struct Helper Transformation**:
```rust
// BEFORE: Complete constructor
fn extract_struct_variant_kind(
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
    variant_name: VariantName,              // ← Parameter 3: variant name
) -> Option<EnumVariantKind>                // ← Returns complete variant

// AFTER: Signature extractor
fn extract_struct_variant_signature(       // ← New function name
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
    // ← Parameter 3 REMOVED (no variant name)
) -> Option<VariantSignature>               // ← Returns ONLY signature
```

#### Return Value Construction Changes

**Tuple Variant**:
```rust
// OLD: Builds complete EnumVariantKind enum variant
Some(EnumVariantKind::Tuple(variant_name, tuple_types))

// NEW: Builds only VariantSignature (no variant name)
Some(VariantSignature::Tuple(tuple_types))
```

**Struct Variant**:
```rust
// OLD: Builds complete EnumVariantKind enum variant
Some(EnumVariantKind::Struct(variant_name, struct_fields))

// NEW: Builds only VariantSignature (no variant name)
Some(VariantSignature::Struct(struct_fields))
```

#### Caller Responsibility Shift

**OLD**: Helpers do everything
```rust
// Helpers return complete EnumVariantKind::Tuple(name, types)
if let Some(tuple_variant) = extract_tuple_variant_kind(v, registry, variant_name.clone()) {
    return Some(tuple_variant);  // Already has name + signature
}
```

**NEW**: Caller assembles name + signature
```rust
// Helpers return only VariantSignature::Tuple(types)
if let Some(signature) = extract_tuple_variant_signature(v, registry) {
    return Some(Self {
        name: variant_name,      // Caller adds name
        signature,               // Helper provided signature
    });
}
```

This transformation enables the struct-based design where `EnumVariantKind { name, signature }` stores each component separately, with helpers focusing solely on signature extraction.

### 4. Helper Functions - extract_tuple_variant_signature and extract_struct_variant_signature

**Rename and update `extract_tuple_variant_kind` → `extract_tuple_variant_signature`**:

```rust
fn extract_tuple_variant_signature(
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
) -> Option<VariantSignature> {
    let prefix_items = v.get_field(SchemaField::PrefixItems)?;
    let prefix_array = prefix_items.as_array()?;

    let tuple_types: Vec<BrpTypeName> = prefix_array
        .iter()
        .filter_map(Value::extract_field_type)
        .collect();

    Some(VariantSignature::Tuple(tuple_types))
}
```

**Rename and update `extract_struct_variant_kind` → `extract_struct_variant_signature`**:

```rust
fn extract_struct_variant_signature(
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
) -> Option<VariantSignature> {
    let properties = v.get_field(SchemaField::Properties)?;
    let props_map = properties.as_object()?;

    let struct_fields: Vec<(StructFieldName, BrpTypeName)> = props_map
        .iter()
        .filter_map(|(field_name, field_schema)| {
            field_schema
                .extract_field_type()
                .map(|type_name| (StructFieldName::from(field_name.clone()), type_name))
        })
        .collect();

    if struct_fields.is_empty() {
        return None;
    }

    Some(VariantSignature::Struct(struct_fields))
}
```

**Note**: Remove `EnumFieldInfo` struct - no longer needed since we build `VariantSignature::Struct` directly.

### 5. Remove EnumFieldInfo struct

Delete the entire struct - it's no longer needed:

```rust
// DELETE THIS:
struct EnumFieldInfo {
    field_name: StructFieldName,
    type_name: BrpTypeName,
}
```

### 6. Update group_variants_by_signature() function

**Decision**: Use direct field access instead of method call for clarity.

Change from calling `.signature()` method to accessing `.signature` field directly:

**Before**:
```rust
let signature = variant.signature();
```

**After**:
```rust
let signature = variant.signature.clone();
```

Updated function:
```rust
fn group_variants_by_signature(
    variants: Vec<EnumVariantKind>,
) -> BTreeMap<VariantSignature, Vec<EnumVariantKind>> {
    let mut groups = BTreeMap::new();
    for variant in variants {
        groups
            .entry(variant.signature.clone())  // Direct field access
            .or_insert_with(Vec::new)
            .push(variant);
    }
    groups
}
```

**Rationale**: Since `EnumVariantKind` fields are public and we're changing from an enum to a struct, direct field access is clearer than maintaining a getter method that just returns a reference.

### 7. Verify No `.signature()` Method Calls Remain

After deletion of the `signature()` method, verify no orphaned calls remain:

```bash
rg "\.signature\(\)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs
```

Should return **no matches** (all uses should be direct field access `.signature`).

## Files to Modify

1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`**
   - Primary file with all changes above
   - Remove `EnumFieldInfo` struct
   - Convert `EnumVariantKind` enum → struct
   - Update all methods
   - Rewrite schema extraction
   - Update helper functions

2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`**
   - No changes needed (VariantSignature remains unchanged)

3. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/mutation_knowledge.rs`**
   - Check if there are any dependencies on `EnumVariantKind`
   - Likely no changes needed

## Testing Strategy

After refactoring:

1. **Build verification**:
   ```bash
   cargo build
   ```

2. **Type guide generation test**:
   ```bash
   /create_mutation_test_json
   ```
   Should produce identical output (241 types, 2200 paths)

3. **Specific enum tests**:
   - Verify mixed signature enums (unit + tuple + struct variants)
   - Verify signature grouping still works correctly
   - Check mutation knowledge lookup still functions

4. **Verify all transformations complete**:
   ```bash
   # Check no pattern matching remains on EnumVariantKind
   rg "Self::(Unit|Tuple|Struct)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs

   # Verify no orphaned EnumFieldInfo references
   rg "EnumFieldInfo" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs

   # Check all construction uses struct literals
   rg "EnumVariantKind::(Unit|Tuple|Struct)" mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs
   ```

   All three commands should return **no matches** after successful refactoring.

## Benefits

1. **Eliminates redundancy**: Type structure stored once, not twice
2. **Clearer data model**: Explicit composition of name + signature
3. **Simpler method implementations**: Direct field access instead of pattern matching
4. **Same functionality**: All existing behavior preserved

## Risks

- Schema extraction logic becomes more imperative (less pattern-matching-based)
- Need to carefully verify all call sites are updated
- Must ensure signature comparison still works for grouping

## Success Criteria

- All tests pass
- Mutation test generates identical output (zero changes)
- No compilation errors or warnings
- Code is clearer and more maintainable
