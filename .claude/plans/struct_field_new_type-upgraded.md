# Plan: Add StructFieldName Newtype

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

### Step 1: Add StructFieldName newtype and trait implementations
**Status**: ✅ COMPLETED
**Objective**: Create the newtype with all trait implementations
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
**Build**: `cargo build`

### Step 2-3: Update core type definitions and enum_path_builder [ATOMIC GROUP]
**Status**: ✅ COMPLETED
**Objective**: Update VariantSignature, PathKind, and EnumFieldInfo to use StructFieldName
**Files**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_kind.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`
**Note**: These changes must be done together to avoid compilation errors
**Build**: `cargo build`

### Step 4: Update builder files
**Status**: ✅ COMPLETED
**Objective**: Update all builder files to use StructFieldName
**Files**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`
**Build**: `cargo build`

### Step 5: Update context and display usage
**Status**: ✅ COMPLETED
**Objective**: Update recursion_context.rs pattern matching
**Files**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`
**Build**: `cargo build && cargo +nightly fmt`

### Final Step: Complete Validation
**Status**: ✅ COMPLETED
**Objective**: Verify the implementation is complete and correct
**Commands**:
```bash
cargo nextest run
cargo +nightly fmt
```

## Overview
Create a `StructFieldName` newtype wrapper similar to `BrpTypeName` and `FullMutationPath` to provide type safety for struct field names throughout the mutation path builder system.

## Location
The newtype will be placed in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs` alongside other newtypes like `FullMutationPath` and `VariantName`.

## Implementation Details

### 1. Newtype Definition
```rust
/// A struct field name used in mutation paths and variant signatures
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructFieldName(String);
```

### 2. Required Trait Implementations
Based on actual usage patterns found:

- `From<String>` - for creating from String
- `From<&str>` - for creating from &str
- `Clone` - used in multiple `.clone()` calls
- `Debug`, `PartialEq`, `Eq`, `Hash` - for use in collections and debugging
- `Serialize`, `Deserialize` - for JSON serialization
- `Display` - for formatting in descriptions and error messages
- `Borrow<str>` - enables HashMap<StructFieldName, V> to accept &str for lookups in `children.get()` calls

Optional implementations to add only if needed:
- `Deref` to `str` - only if we find we need transparent string access

### 3. Methods to Implement
```rust
impl StructFieldName {
    /// Get the field name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StructFieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::borrow::Borrow<str> for StructFieldName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<String> for StructFieldName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for StructFieldName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
```

## Files to Update

### Core Type Definitions
1. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs`**
   - Add the `StructFieldName` newtype definition
   - Update `VariantSignature::Struct` from `Vec<(String, BrpTypeName)>` to `Vec<(StructFieldName, BrpTypeName)>`

2. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_kind.rs`**
   - Update `PathKind::StructField` field from `field_name: String` to `field_name: StructFieldName`
   - Update `to_mutation_path_descriptor()` to use `.as_str()` or similar

3. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`**
   - Update `EnumFieldInfo` struct field from `field_name: String` to `field_name: StructFieldName`
   - Update tuple mapping in `signature()` method to use `StructFieldName` in tuple
   - Update `EnumFieldInfo` construction in `extract_struct_fields()` to use `StructFieldName::from(field_name)`
   - Update descriptor creation in `build_variant_example()` to use field name
   - Update field value insertion in `build_variant_example()` for struct variants
   - Update `PathKind::StructField` construction in `create_paths_for_signature()` with `StructFieldName`

### Builder Files
4. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`**
   - Update field name creation in `build()` method to use `StructFieldName::from(field_name)`

5. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`**
   - Update to use `StructFieldName::from(SchemaField::Key.to_string())` and `StructFieldName::from(SchemaField::Value.to_string())` in path creation
   - May need adjustment for HashMap lookups with `SchemaField::Key.as_ref()` and `SchemaField::Value.as_ref()`

6. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`**
   - Update to use `StructFieldName::from(SchemaField::Items.to_string())` in path creation
   - May need adjustment for HashMap lookup with `SchemaField::Items.as_ref()`

### Context and Display
7. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`**
   - Update `PathKind::StructField` pattern match and format string usage in path segment generation

## Call Site Analysis

### Places that create field names:
- From JSON schema field names (struct fields from properties)
- From `SchemaField` enum variants converted to strings (Key, Value, Items)
- From cloning existing field names

### Places that consume field names:
- Building mutation path strings (`.field_name` format)
- HashMap lookups using field names as keys
- Display/formatting in descriptions
- JSON serialization of variant signatures

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

1. Add the newtype definition first
2. Update type definitions (`PathKind`, `VariantSignature`, `EnumFieldInfo`) atomically
3. Update all construction sites to use `StructFieldName::from()`
4. Update all consumption sites to use `.as_str()` where needed
5. Test to ensure no behavioral changes

## Benefits

1. **Type Safety**: Can't accidentally pass wrong string type
2. **Documentation**: Clear intent when a string represents a field name
3. **Consistency**: Matches pattern of `BrpTypeName` and `FullMutationPath`
4. **Future Extensibility**: Easy to add field name validation or transformations

## Notes

- No need for `Deref` initially - explicit `.as_str()` is clearer
- `Borrow<str>` implementation enables HashMap<StructFieldName, V> lookups with &str keys
- Keep implementation minimal - only add traits we actually use

## Design Review Skip Notes

This section tracks design review decisions and items to skip in future reviews.