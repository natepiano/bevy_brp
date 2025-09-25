# Plan: Add StructFieldName Newtype

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
- `AsRef<str>` - used in `children.get()` calls for HashMap lookups

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
   - Update line 72: map to use `StructFieldName` in tuple
   - Update line 174: construct `EnumFieldInfo` with `StructFieldName::from(field_name)`
   - Update line 303: create descriptor using field name
   - Update line 305: use field name for insertion
   - Update line 531: construct `PathKind::StructField` with `StructFieldName`

### Builder Files
4. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`**
   - Line 68: Update to create `StructFieldName::from(field_name)`

5. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`**
   - Lines 73, 78: Update to use `StructFieldName::from(SchemaField::Key.to_string())`
   - Lines 95, 103: May need adjustment for HashMap lookups

6. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`**
   - Line 47: Update to use `StructFieldName::from(SchemaField::Items.to_string())`
   - Line 60: May need adjustment for HashMap lookup

### Context and Display
7. **`mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`**
   - Line with `PathKind::StructField`: Update pattern match and format string usage

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

1. Add the newtype definition first
2. Update type definitions (`PathKind`, `VariantSignature`, `EnumFieldInfo`)
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
- `AsRef<str>` implementation needed for HashMap lookups with `SchemaField::Key.as_ref()`
- Keep implementation minimal - only add traits we actually use