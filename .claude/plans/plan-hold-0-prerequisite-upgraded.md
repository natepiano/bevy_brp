# Plan 0 Prerequisite: Pass Signature Grouping Data Through Associated Types

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

### Step 1: Add Associated Type and prepare_children_data Method ⏳ PENDING
**Objective**: Add infrastructure to PathBuilder trait for builder-specific data passing
**Changes**: Add ChildrenData associated type with default and prepare_children_data method
**Files**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`
**Build**: `cargo build`
**Type**: SAFE (Additive - no breaking changes)

### Step 2: Create EnumChildrenData Structure ⏳ PENDING
**Objective**: Define the data structure for passing enum-specific information
**Changes**: Add EnumChildrenData struct that holds both examples and signature groups
**Files**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
**Build**: `cargo build`
**Type**: SAFE (Additive - new type definition)

### Step 3: Update All Builder Implementations ⏳ PENDING
**Objective**: Change assemble_from_children signature across all builders
**Changes**: Update trait and all 8 builder implementations to use Self::ChildrenData
**Files**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/array_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/list_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/map_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/set_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/struct_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/tuple_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/value_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs`
**Build**: `cargo build`
**Type**: ATOMIC GROUP (Must update trait and all implementations together)

### Step 4: Implement EnumMutationBuilder Specialization ⏳ PENDING
**Objective**: Add enum-specific data handling to eliminate redundant variant extraction
**Changes**: Add type alias, prepare_children_data implementation, update assemble logic
**Files**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builders/enum_builder.rs`
**Build**: `cargo build`
**Type**: SAFE (Enhancement - builds on previous changes)

### Step 5: Complete Validation ⏳ PENDING
**Objective**: Verify the refactoring produces identical output
**Changes**: None - validation only
**Commands**:
```bash
cargo nextest run
# Generate type guides and compare output
```
**Type**: VALIDATION

## Goal
Eliminate redundant variant extraction and grouping in `enum_builder.rs` by passing the already-computed signature groups from `collect_children()` to `assemble_from_children()` via associated types.

## Current Problem
Currently, we extract and group enum variants **twice**:
1. In `collect_children()` - lines 350-353
2. In `assemble_from_children()` - lines 410-411

This is inefficient and violates DRY principles.

## Solution: Use Associated Types for Builder-Specific Data

### 1. Update PathBuilder Trait

```rust
// In path_builder.rs
pub trait PathBuilder {
    type Item: MaybeVariants;
    type Iter<'a>: Iterator<Item = Self::Item> where Self: 'a;

    // NEW: Each builder can specify what data it needs from collect phase
    type ChildrenData = HashMap<MutationPathDescriptor, Value>; // Default for most builders

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>>;

    // NEW: Transform collected data before assembly
    // Default implementation for builders that don't need special handling
    fn prepare_children_data(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        _ctx: &RecursionContext,
    ) -> Result<Self::ChildrenData> {
        Ok(children)  // Most builders just pass through
    }

    // CHANGED: Now accepts builder-specific data type
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: Self::ChildrenData,  // Now builder-specific!
    ) -> Result<Value>;
}
```

### 2. Define EnumChildrenData Type

```rust
// In enum_builder.rs
/// Data passed from collect phase to assembly phase for enum builders
#[derive(Debug)]
struct EnumChildrenData {
    /// The assembled examples by descriptor (from recursion)
    examples: HashMap<MutationPathDescriptor, Value>,

    /// The signature grouping extracted once during collect_children
    /// Map from signature to all variants with that signature
    signature_groups: HashMap<VariantSignature, Vec<EnumVariantInfo>>,
}
```

### 3. Update All Non-Enum Builders (Simple Signature Change)

For all builders EXCEPT enum_builder, only the `assemble_from_children` signature needs to change.

**Example - array_builder.rs (lines 59-62):**

**Before:**
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: HashMap<MutationPathDescriptor, Value>,
) -> Result<Value> {
    // ... existing logic unchanged ...
}
```

**After:**
```rust
fn assemble_from_children(
    &self,
    ctx: &RecursionContext,
    children: Self::ChildrenData,  // Only this line changes!
) -> Result<Value> {
    // ... existing logic unchanged ...
}
```

**Files requiring this simple change:**
- `array_builder.rs` - line 59
- `list_builder.rs` - (check line number)
- `map_builder.rs` - (check line number)
- `set_builder.rs` - (check line number)
- `struct_builder.rs` - (check line number)
- `tuple_builder.rs` - (check line number)
- `value_builder.rs` - (check line number)

Note: These builders will use the default `type ChildrenData = HashMap<MutationPathDescriptor, Value>` from the trait, so `Self::ChildrenData` IS the same HashMap type they already use. No logic changes needed!

### 4. Implement Full Specialization for EnumMutationBuilder

Only enum_builder needs the full implementation with custom data type:

```rust
impl PathBuilder for EnumMutationBuilder {
    type Item = PathKindWithVariants;
    type Iter<'a> = std::vec::IntoIter<PathKindWithVariants> where Self: 'a;

    // NEW: Specify we need enriched data (only enum_builder does this!)
    type ChildrenData = EnumChildrenData;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        // ... existing collect_children unchanged ...
    }

    fn prepare_children_data(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        ctx: &RecursionContext,
    ) -> Result<Self::ChildrenData> {
        let schema = ctx.require_registry_schema()?;

        // Extract and group variants ONCE (we do it here since we have ctx)
        let variants = extract_enum_variants(schema, &ctx.registry);
        let signature_groups = group_variants_by_signature(variants);

        Ok(EnumChildrenData {
            examples: children,
            signature_groups,
        })
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: Self::ChildrenData,  // Now EnumChildrenData!
    ) -> Result<Value> {
        // NO MORE redundant extraction! Use children.signature_groups directly
        let variant_groups = &children.signature_groups;
        let child_examples = &children.examples;

        let mutation_example = match &ctx.enum_context {
            Some(EnumContext::Root) => {
                let mut examples = Vec::new();

                for (signature, variants_in_group) in variant_groups {
                    let representative = variants_in_group.first().ok_or_else(|| {
                        Report::new(Error::InvalidState("Empty variant group".to_string()))
                    })?;

                    let example = Self::build_variant_example(
                        signature,
                        representative.name(),
                        child_examples,  // Use from EnumChildrenData
                        ctx.type_name(),
                    );

                    // ... rest of example building ...
                }

                MutationExample::EnumRoot(examples)
            }
            // ... other cases also use children.signature_groups ...
        }

        // ... rest of method ...
    }
}
```

### 5. Update builder.rs to Call prepare_children_data

```rust
// In builder.rs, wherever we call assemble_from_children
fn process_all_children(&mut self, ctx: &RecursionContext) -> Result<()> {
    // ... existing code to recurse and build HashMap ...

    // NEW: Let builder prepare its specific data type
    let prepared_children = self.builder.prepare_children_data(child_examples, ctx)?;

    // Call assemble with prepared data
    let assembled = self.builder.assemble_from_children(ctx, prepared_children)?;

    // ... rest of processing ...
}
```

## Benefits

1. **No Redundant Work**: Variants extracted and grouped only once
2. **Type Safety**: Each builder specifies exactly what data it needs
3. **Backward Compatible**: Default implementation means non-enum builders unchanged
4. **Clean Separation**: Data preparation logic separated from assembly logic
5. **No Output Changes**: This is pure refactoring - output remains identical

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Implementation Steps

### Phase 1: Add Trait Infrastructure (Additive - Safe)
1. Add associated type `ChildrenData` to `PathBuilder` trait with default
2. Add `prepare_children_data` method with default implementation

### Phase 2: Update All Implementations (Atomic - Must be done together)
3. Update `assemble_from_children` signature in trait to use `Self::ChildrenData`
4. Update `assemble_from_children` signature in ALL 8 builders:
   - `array_builder.rs` - change line 59-62
   - `list_builder.rs` - change assemble_from_children signature
   - `map_builder.rs` - change assemble_from_children signature
   - `set_builder.rs` - change assemble_from_children signature
   - `struct_builder.rs` - change assemble_from_children signature
   - `tuple_builder.rs` - change assemble_from_children signature
   - `value_builder.rs` - change assemble_from_children signature
   - `enum_builder.rs` - change assemble_from_children signature
5. Update builder.rs to call `prepare_children_data` before `assemble_from_children`

### Phase 3: Specialize EnumMutationBuilder (Enhancement - Safe)
6. Create `EnumChildrenData` struct in enum_builder.rs
7. Add `type ChildrenData = EnumChildrenData` for `EnumMutationBuilder`
8. Implement `prepare_children_data` for `EnumMutationBuilder`
9. Update enum_builder's `assemble_from_children` to use `children.signature_groups`
10. Remove redundant variant extraction from enum_builder's `assemble_from_children`

### Phase 4: Validation
11. Run `cargo build` to verify compilation
12. Run `cargo nextest run` to verify all tests pass
13. Generate type guides for TestVariantChainEnum and Color - verify identical output

## Testing

Since this is pure refactoring with no output changes:
1. Run existing tests - all should pass
2. Generate type guide for `TestVariantChainEnum` - should be identical
3. Generate type guide for `Color` enum - should be identical
4. Memory usage should be slightly better (one less HashMap created)

## Notes

- We extract variants in `prepare_children_data` rather than storing them from `collect_children` because we need the `RecursionContext` to access the schema
- The default implementation of `prepare_children_data` ensures other builders continue to work unchanged
- This sets the foundation for Plan 0's variant chain tracking by establishing the data flow pattern