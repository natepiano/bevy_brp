# Plan: Decouple Enum Builder from PathBuilder Trait

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

**Step 1: Create Infrastructure** ✅ COMPLETED
- **Objective**: Add BuildStrategy enum, create EnumPathBuilder struct, add feature flag
- **Files**: builder.rs, enum_path_builder.rs (NEW)
- **Change Type**: Additive
- **Build**: `cargo build && cargo +nightly fmt`
- **Impact**: Sets up parallel implementation without affecting existing code

**Step 2: Extract Shared Logic** ✅ COMPLETED
- **Objective**: Extract core enum functions, update EnumMutationBuilder to use shared functions
- **Files**: enum_builder.rs
- **Change Type**: Additive (refactoring without behavior change)
- **Build**: `cargo build && cargo +nightly fmt`
- **Impact**: Creates single source of truth for enum logic

**Step 3: Implement EnumPathBuilder** ✅ COMPLETED
- **Objective**: Complete EnumPathBuilder implementation using shared functions
- **Files**: enum_path_builder.rs, builder.rs
- **Change Type**: Additive
- **Build**: `cargo build && cargo +nightly fmt`
- **Impact**: New enum processing path ready for testing

**Step 4: Golden Master Baseline** ⏳ PENDING
- **Objective**: Capture current output as baseline before switching
- **Files**: None (testing only)
- **Change Type**: Testing
- **Build**: `.claude/commands/create_mutation_test_json.md`
- **Impact**: Establishes verification baseline for identical output

**Step 5: Enable and Test New Path** ⏳ PENDING
- **Objective**: Enable feature flag, test new implementation, verify identical output
- **Files**: builder.rs (flag change)
- **Change Type**: Configuration
- **Build**: `cargo build && .claude/commands/create_mutation_test_json.md`
- **Impact**: Validates new path produces identical output

**Step 6: Switch Over Permanently** ⏳ PENDING
- **Objective**: Remove PathBuilder impl from EnumMutationBuilder, keep flag enabled
- **Files**: enum_builder.rs, builder.rs
- **Change Type**: Breaking (removes trait implementation) - ATOMIC GROUP
- **Build**: `cargo build && cargo +nightly fmt`
- **Impact**: Breaks dependency on PathBuilder trait for enums

**Step 7: Clean Up** ⏳ PENDING
- **Objective**: Remove MaybeVariants trait, PathKindWithVariants, simplify PathBuilder, remove feature flag
- **Files**: path_builder.rs, enum_builder.rs, builder.rs
- **Change Type**: Removal (unused code)
- **Build**: `cargo build && cargo +nightly fmt && cargo nextest run`
- **Impact**: Final cleanup removes enum-specific complexity from trait system

**Final Step: Complete Validation** ⏳ PENDING
- **Objective**: Run all tests and verify success criteria
- **Files**: None (testing only)
- **Change Type**: Validation
- **Build**: `cargo nextest run && .claude/commands/create_mutation_test_json.md`
- **Impact**: Confirms successful refactoring with identical output

## Goal
Refactor `EnumMutationBuilder` to not implement `PathBuilder`, handling enum types through a separate, dedicated code path. This is a pure refactoring with **zero functional changes** - output must remain byte-for-byte identical.

## Motivation
The enum builder has fundamentally different requirements from other builders:
- Groups variants by signature (unique behavior)
- Produces paths with `applicable_variants` lists
- Requires variant tracking through recursion
- Has context-dependent output (root/child/none)

Forcing it through `PathBuilder` creates unnecessary complexity with associated types (`MaybeVariants`, `PathKindWithVariants`) that only enums need.

## Success Criteria
1. **Identical Output**: Generated type guides must be byte-for-byte identical
2. **No New Features**: This is pure refactoring, no bug fixes or enhancements
3. **Clean Separation**: Enum handling completely separate from `PathBuilder`
4. **All Tests Pass**: No regression in any existing tests

## Architecture Overview

### Current Architecture
```
RecursionContext (with EnumContext)
    ↓
PathBuilder trait
    ↓
EnumMutationBuilder implements PathBuilder
    ↓
collect_children() → HashMap → assemble_from_children()
```

### New Architecture
```
RecursionContext (with EnumContext - retained for propagation)
    ↓
BuildStrategy enum determines path
    ├── Simple(PathBuilder) → existing flow
    └── Enum → dedicated EnumPathBuilder
```

## Implementation Details

### Phase 1: Create Infrastructure (Additive - Safe)

#### 1.1 Add BuildStrategy Enum
Create in `builder.rs`:
```rust
enum BuildStrategy {
    Simple(Box<dyn PathBuilder>),
    Enum,  // No PathBuilder trait needed
}
```

#### 1.2 Create EnumPathBuilder Struct
New file `mcp/src/brp_tools/brp_type_guide/enum_path_builder.rs`:
```rust
/// Standalone enum path builder - no PathBuilder dependency
pub struct EnumPathBuilder;

/// Enum path with variant information - replaces PathKindWithVariants
struct EnumPathWithVariants {
    path: Option<PathKind>,
    applicable_variants: Vec<VariantName>,
}

impl EnumPathBuilder {
    /// Process enum type directly, bypassing PathBuilder trait
    pub fn process_enum(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Step 1: Collect enum children (similar to current collect_children)
        let paths_with_variants = self.collect_enum_children(ctx)?;

        // Step 2: Recurse into child types using MutationPathBuilder
        let child_examples = self.recurse_children(paths_with_variants, ctx, depth)?;

        // Step 3: Assemble examples (similar to current assemble_from_children)
        let final_example = self.assemble_enum_examples(child_examples, ctx)?;

        // Step 4: Create MutationPathInternal objects directly
        self.create_mutation_paths(paths_with_variants, final_example, ctx)
    }

    fn collect_enum_children(&self, ctx: &RecursionContext) -> Result<Vec<EnumPathWithVariants>> {
        // Extract current collect_children logic here
    }

    fn recurse_children(
        &self,
        paths: Vec<EnumPathWithVariants>,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<HashMap<MutationPathDescriptor, Value>> {
        // For each path, call regular MutationPathBuilder recursion
    }

    fn assemble_enum_examples(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        ctx: &RecursionContext,
    ) -> Result<Value> {
        // Extract current assemble_from_children logic here
    }

    fn create_mutation_paths(
        &self,
        paths_with_variants: Vec<EnumPathWithVariants>,
        example: Value,
        ctx: &RecursionContext,
    ) -> Result<Vec<MutationPathInternal>> {
        // Create final MutationPathInternal objects
    }
}
```

#### 1.3 Add Feature Flag
In `builder.rs`:
```rust
const USE_DECOUPLED_ENUM: bool = false;  // Start with old path

fn determine_build_strategy(&self, ctx: &RecursionContext) -> BuildStrategy {
    if is_enum_type(ctx.type_name()) {
        if USE_DECOUPLED_ENUM {
            BuildStrategy::Enum
        } else {
            // Keep using old path initially
            BuildStrategy::Simple(Box::new(EnumMutationBuilder::new()))
        }
    } else {
        // ... other builders
    }
}
```

### Phase 2: Extract Shared Logic and Implement EnumPathBuilder (Safe)

#### 2.1 Extract Shared Functions
In `enum_builder.rs`, extract core enum logic into private functions that both implementations can use:

```rust
// Private shared functions in enum_builder.rs
fn extract_and_group_variants(
    ctx: &RecursionContext
) -> Result<HashMap<VariantSignature, Vec<EnumVariantInfo>>> {
    // Extract current collect_children variant processing logic
    let schema = ctx.require_registry_schema()?;
    let variants = extract_enum_variants(schema, &ctx.registry);
    Ok(group_variants_by_signature(variants))
}

fn build_enum_examples(
    variant_groups: &HashMap<VariantSignature, Vec<EnumVariantInfo>>,
    child_examples: HashMap<MutationPathDescriptor, Value>,
    ctx: &RecursionContext,
) -> Result<Value> {
    // Extract current assemble_from_children logic exactly
    match &ctx.enum_context {
        Some(EnumContext::Root) => {
            // Build examples array for enum root path
            // ... current logic from assemble_from_children
        }
        Some(EnumContext::Child) => {
            // Return concrete example for child context
            // ... current logic
        }
        None => {
            // Return concrete example for non-enum parent
            // ... current logic
        }
    }
}
```

#### 2.2 Update Current EnumMutationBuilder to Use Shared Functions
Refactor existing `PathBuilder` implementation to use shared functions:

```rust
impl PathBuilder for EnumMutationBuilder {
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        let variant_groups = extract_and_group_variants(ctx)?;

        // Convert to PathKindWithVariants (existing pattern preserved)
        let mut children = Vec::new();
        for (signature, variants_in_group) in variant_groups {
            // ... existing conversion logic uses variant_groups
        }
        Ok(children.into_iter())
    }

    fn assemble_from_children(&self, ctx: &RecursionContext, children: HashMap<...>) -> Result<Value> {
        let variant_groups = extract_and_group_variants(ctx)?;
        build_enum_examples(&variant_groups, children, ctx)
    }
}
```

#### 2.3 Implement EnumPathBuilder Using Shared Functions
Create new `enum_path_builder.rs` that uses the same shared functions:
```rust
use super::enum_builder::{extract_and_group_variants, build_enum_examples};

impl EnumPathBuilder {
    fn collect_enum_children(&self, ctx: &RecursionContext) -> Result<Vec<EnumPathWithVariants>> {
        let variant_groups = extract_and_group_variants(ctx)?;

        // Convert to EnumPathWithVariants (new pattern)
        let mut children = Vec::new();

        for (signature, variants_in_group) in variant_groups {
            let applicable_variants = variants_in_group
                .iter()
                .map(|v| v.variant_name().clone())
                .collect();

            match signature {
                VariantSignature::Unit => {
                    children.push(EnumPathWithVariants {
                        path: None,
                        applicable_variants,
                    });
                }
                VariantSignature::Tuple(types) => {
                    for (index, type_name) in types.iter().enumerate() {
                        children.push(EnumPathWithVariants {
                            path: Some(PathKind::IndexedElement {
                                index,
                                type_name: type_name.clone(),
                                parent_type: ctx.type_name().clone(),
                            }),
                            applicable_variants: applicable_variants.clone(),
                        });
                    }
                }
                VariantSignature::Struct(fields) => {
                    for (field_name, type_name) in fields {
                        children.push(EnumPathWithVariants {
                            path: Some(PathKind::StructField {
                                field_name: field_name.clone(),
                                type_name: type_name.clone(),
                                parent_type: ctx.type_name().clone(),
                            }),
                            applicable_variants: applicable_variants.clone(),
                        });
                    }
                }
            }
        }

        Ok(children)
    }

    fn recurse_children(
        &self,
        paths_with_variants: Vec<EnumPathWithVariants>,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<HashMap<MutationPathDescriptor, Value>> {
        let mut child_examples = HashMap::new();

        for path_info in paths_with_variants {
            if let Some(path_kind) = path_info.path {
                // Create child context for recursion
                let child_ctx = create_child_context(&path_kind, ctx);

                // Recurse using regular MutationPathBuilder for non-enum field
                let child_result = recurse_mutation_paths(&child_ctx, depth)?;

                // Extract descriptor and store result
                let descriptor = extract_path_descriptor(&path_kind);
                child_examples.insert(descriptor, child_result.example);
            }
        }

        Ok(child_examples)
    }

    fn assemble_enum_examples(
        &self,
        children: HashMap<MutationPathDescriptor, Value>,
        ctx: &RecursionContext,
    ) -> Result<Value> {
        let variant_groups = extract_and_group_variants(ctx)?;
        build_enum_examples(&variant_groups, children, ctx)
    }
}
```

### Phase 3: Implement Switching Logic (Integration - Safe)

#### 3.1 Update MutationPathBuilder
```rust
impl MutationPathBuilder {
    fn process_type(&mut self, ctx: &RecursionContext, depth: RecursionDepth) -> Result<()> {
        match self.determine_build_strategy(ctx)? {
            BuildStrategy::Simple(builder) => {
                self.process_with_path_builder(builder, ctx, depth)
            }
            BuildStrategy::Enum => {
                self.process_enum_directly(ctx, depth)
            }
        }
    }

    fn process_enum_directly(&mut self, ctx: &RecursionContext, depth: RecursionDepth) -> Result<()> {
        // Create enum path builder and process directly
        let processor = EnumPathBuilder;
        let paths = processor.process_enum(ctx, depth)?;

        // Integrate results into builder
        self.paths.extend(paths);
        Ok(())
    }
}
```

### Phase 4: Verification (Testing - Safe)

#### 4.1 Run Golden Master Test - Baseline
Before enabling the new path, capture current baseline:
```bash
# Execute create_mutation_test_json to capture baseline
.claude/commands/create_mutation_test_json.md
```

#### 4.2 Enable Feature Flag and Test
```rust
const USE_DECOUPLED_ENUM: bool = true;  // Switch to new path
```

Run golden master test again:
```bash
.claude/commands/create_mutation_test_json.md
```

The comparison will detect any differences between old and new paths.

#### 4.3 Test All Enum Types
Verify identical output for:
- `TestVariantChainEnum`
- `Color`
- `Option<T>` special cases
- Simple enums (unit variants only)
- Complex nested enums

### Phase 5: Switch Over (Atomic Change)

#### 5.1 Enable New Path Permanently
```rust
const USE_DECOUPLED_ENUM: bool = true;  // Permanent switch
```

#### 5.2 Remove PathBuilder Implementation
Remove from `enum_builder.rs`:
```rust
// Delete this entire impl block
impl PathBuilder for EnumMutationBuilder { ... }
```

### Phase 6: Cleanup (Final - Safe)

#### 6.1 Remove Enum-Specific Traits and Types
- Remove `MaybeVariants` trait (only enums needed it)
- Remove `PathKindWithVariants` struct (replaced by `EnumPathWithVariants`)
- Remove feature flag constant

#### 6.2 Simplify PathBuilder Trait
```rust
pub trait PathBuilder {
    type Item = PathKind;  // No more MaybeVariants complexity
    type Iter<'a>: Iterator<Item = PathKind>;  // Simplified associated type

    // Methods remain the same but simpler signatures
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>>;
    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> Result<Value>;
}
```

#### 6.3 Remove EnumMutationBuilder
Since it no longer implements `PathBuilder`, remove the struct entirely:
```rust
// Remove entire struct and implementation
// pub struct EnumMutationBuilder;
```

## Testing Strategy

### Golden Master Testing
Use the existing `create_mutation_test_json.md` command as the comprehensive golden master test:

1. **Before Phase 4**: Run to capture baseline output
2. **After Phase 4**: Run to verify new path produces identical output
3. **After Phase 5**: Run to verify permanent switch maintains output
4. **After Phase 6**: Run final verification after cleanup

### Build Validation
After each phase, run:
```bash
cargo build && cargo +nightly fmt && cargo nextest run
```

### Regression Testing
Pay special attention to:
- Nested enum tests
- Option<T> formatting
- Enum variant path generation
- Complex enum structures

## Migration Strategy
**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Rollback Strategy

Each phase is designed to be safe and reversible:
- **Phase 1-2**: Purely additive, old code untouched
- **Phase 3-4**: Feature flag allows instant rollback
- **Phase 5**: Can be reverted as atomic commit
- **Phase 6**: Optional cleanup, can be deferred

## Risk Assessment

### Low Risk
- Output format unchanged (verified by golden master test)
- Gradual migration with feature flag
- Extensive testing at each phase
- Familiar collect/recurse/assemble pattern maintained

### Potential Issues
1. **Recursion state management** - Need careful handling of context passing
2. **HashMap key collisions** - Must preserve current collision avoidance
3. **Special cases** - Option<T> has special handling that must be preserved

## Final Validation

The refactoring is complete when:
1. ✓ All tests pass
2. ✓ Output is byte-for-byte identical
3. ✓ EnumMutationBuilder doesn't implement PathBuilder
4. ✓ EnumContext remains in RecursionContext but enum processing is decoupled
5. ✓ Code is cleaner and more maintainable
6. ✓ PathBuilder trait simplified without enum-specific complexity
7. ✓ MaybeVariants and PathKindWithVariants removed

## Timeline Estimate

- **Phase 1**: 1-2 hours (create infrastructure)
- **Phase 2**: 2-3 hours (implement processor logic)
- **Phase 3**: 1 hour (integration)
- **Phase 4**: 1-2 hours (testing and verification)
- **Phase 5**: 30 minutes (permanent switch)
- **Phase 6**: 1 hour (cleanup)

**Total: 6-8 hours of focused work**