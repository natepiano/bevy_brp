# Plan: Improved Enum Variant Signature Knowledge - Single Choke Point

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

### **STEP 1: Back Out Current Changes** - ✅ COMPLETED

**Objective**: Remove temporary knowledge application logic from `build_variant_group_example` and restore borrowing pattern

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Changes**:
1. Remove knowledge application logic (lines 636-647)
2. Change function parameter from owned to borrowed HashMap (line 623)
3. Update call site to pass borrowed reference (line ~698)

**Build command**: `cargo build && cargo +nightly fmt`

**Why**: This backs out a temporary fix that applied knowledge too late. The new architecture applies knowledge earlier via `find_knowledge()`.

---

### **STEP 2: Add Parent Variant Signature to Context** - ✅ COMPLETED

**Objective**: Add `parent_variant_signature` field to `RecursionContext` to propagate enum signature information through recursion

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

**Changes**:
1. Add `parent_variant_signature: Option<VariantSignature>` field to struct
2. Initialize to `None` in `new()` method
3. Propagate via clone in `create_recursion_context()`

**Build command**: `cargo build && cargo +nightly fmt`

**Why**: This field allows child paths to know they're part of an enum variant signature, enabling signature-specific knowledge lookup.

**Dependencies**: None (additive change)

---

### **STEP 3: Update find_knowledge() to Check Enum Signature and Return Result** - ✅ COMPLETED

**Objective**: Change `find_knowledge()` to return `Result`, add enum signature matching, and update call site

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs` (find_knowledge method)
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` (call site at line 606)

**Changes**:
1. Change return type from `Option` to `Result<Option<...>, BuilderError>`
2. Add enum signature matching for `PathKind::IndexedElement`
3. Use proper error handling (`BuilderError::SystemError`) for architectural invariant violations
4. Update all return statements to wrap in `Ok(...)`
5. Update call site to use `?` operator

**Build command**: `cargo build && cargo +nightly fmt`

**Why**: This is the "single choke point" where ALL knowledge application happens, including enum signature-specific knowledge.

**Dependencies**: Requires Step 2 (parent_variant_signature field must exist)

**Notes**: ATOMIC GROUP - both find_knowledge AND its call site must be updated together to compile

---

### **STEP 4: Set Parent Variant Signature in Enum Builder** - ⏳ PENDING

**Objective**: Set `parent_variant_signature` when processing enum variant children

**Files to modify**:
- `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Changes**:
1. Add `signature: &VariantSignature` parameter to `process_signature_path` function
2. Set `child_ctx.parent_variant_signature = Some(signature.clone())` after creating child context
3. Update call site to pass `signature` parameter

**Build command**: `cargo build && cargo +nightly fmt`

**Why**: This connects the enum builder to the knowledge system, ensuring child paths know their parent's signature.

**Dependencies**: Requires Step 2 (field exists) and Step 3 (find_knowledge can use it)

---

### **STEP 5: Complete Validation** - ⏳ PENDING

**Objective**: Verify all changes work correctly

**Testing commands**:
```bash
# Run tests
cargo nextest run

# Verify AlphaMode2d type guide
# Check: type_guide["bevy_sprite::sprite::AlphaMode2d"].mutation_paths[".0"].example == 0.5

# Verify Transform still uses π
# Check: type_guide["bevy_transform::components::transform::Transform"].mutation_paths[".translation.x"].example == 3.1415927...
```

**Success criteria**:
- All tests pass
- AlphaMode2d `.0` path shows `0.5` (not π)
- Transform `.translation.x/y/z` still show π
- No architectural invariant violation crashes

---

## Problem with Current Implementation

The current implementation applies knowledge in `build_variant_group_example` which:
1. Only affects the HashMap used for root example assembly
2. Does NOT update the child path's own `.example` field
3. Results in `.0` path showing π (3.14...) instead of 0.5
4. Requires checking knowledge in two places (ugly, no "taste")

**Current state**: Root example correctly shows `{"Mask": 0.5}`, but `.0` path's example still shows `3.1415927...`

## Root Cause

The f32 child gets its example from `RecursionContext::find_knowledge()` during recursion, but at that point it doesn't know it's part of an enum signature. The enum builder applies knowledge later during assembly, but the child path object already has the wrong value.

## Solution: Single Choke Point via Context Propagation

Add parent enum signature information to `RecursionContext` so that when the child calls `find_knowledge()`, it can check signature-specific knowledge first. This creates a single point where ALL knowledge is applied.

### Benefits
1. **Single source of truth**: Knowledge checked in ONE place (`find_knowledge`)
2. **Automatic propagation**: Both child path example AND parent assembly get the correct value
3. **Clean code**: No manual syncing, no duplicate logic
4. **Extensible**: Works for struct variant fields too

## Migration Strategy

**Migration Strategy: Phased**

This collaborative plan uses phased implementation by design. The Collaborative Execution Protocol above defines the phase boundaries with validation checkpoints between each step.

## Design Review Skip Notes

This plan has completed a comprehensive gap analysis review:
- 7 gaps identified and fixed
- All gaps addressed proper error handling, complete signatures, and concrete testing steps
- Ready for implementation

## Implementation Steps

### Step 1: Back Out Current Changes

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

**Current state (lines 620-658)**: The function currently has:
1. Parameter: `mut child_examples: HashMap<MutationPathDescriptor, Value>` (owns the HashMap)
2. Logic at lines 636-647 that mutates child_examples to apply signature-specific knowledge
3. Helper function `check_signature_element_knowledge` at lines 588-617

**Remove the knowledge application logic from `build_variant_group_example`** (lines 636-647):

```rust
// REMOVE these lines:
        // Apply signature-specific knowledge to override child examples
        // Mutate in place since we own child_examples now
        if let VariantSignature::Tuple(types) = signature {
            for (index, _type_name) in types.iter().enumerate() {
                if let Some(knowledge_value) =
                    check_signature_element_knowledge(ctx.type_name(), signature, index)?
                {
                    let descriptor = MutationPathDescriptor::from(index.to_string());
                    child_examples.insert(descriptor, knowledge_value);
                }
            }
        }
```

**Change function signature back to borrowing** (line 623):
```rust
// Current (line 623):
mut child_examples: HashMap<MutationPathDescriptor, Value>,  // owns

// New:
child_examples: &HashMap<MutationPathDescriptor, Value>,  // borrows
```

**Why restore borrowing**: With the new architecture, knowledge is applied during child creation in `find_knowledge()`, so child_examples already contains the correct values. We no longer need ownership to mutate it in this function.

**Update call site** in `process_children` (around line 698):
```rust
// Current:
let example = build_variant_group_example(
    signature,
    variants_in_group,
    child_examples,  // pass ownership
    signature_status,
    ctx,
)?;

// New:
let example = build_variant_group_example(
    signature,
    variants_in_group,
    &child_examples,  // borrow
    signature_status,
    ctx,
)?;
```

**Keep the `check_signature_element_knowledge` helper function** (lines 588-617) but note that it will no longer be called from `build_variant_group_example` since knowledge application now happens earlier in the recursion via `find_knowledge()`. This function can remain for potential future use or be removed in a later cleanup.

### Step 2: Add Parent Variant Signature to Context

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

Add field to `RecursionContext` to track parent enum variant signature:

```rust
pub struct RecursionContext {
    pub path_kind:          PathKind,
    pub registry:           Arc<HashMap<BrpTypeName, Value>>,
    pub full_mutation_path: FullMutationPath,
    pub path_action:        PathAction,
    pub variant_chain:      Vec<VariantName>,
    pub depth:              RecursionDepth,
    /// Parent enum variant signature (only set when processing enum variant children)
    /// The enum type is available via path_kind.parent_type - no need to store it redundantly
    pub parent_variant_signature: Option<VariantSignature>,  // NEW
}
```

Update `new()` method:
```rust
pub fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
    Self {
        path_kind,
        registry,
        full_mutation_path: FullMutationPath::from(""),
        path_action: PathAction::Create,
        variant_chain: Vec::new(),
        depth: RecursionDepth::ZERO,
        parent_variant_signature: None,  // NEW
    }
}
```

Update `create_recursion_context()` to propagate parent variant signature:
```rust
pub fn create_recursion_context(
    &self,
    path_kind: PathKind,
    child_path_action: PathAction,
) -> std::result::Result<Self, BuilderError> {
    // ... existing depth checking ...

    Ok(Self {
        path_kind,
        registry: Arc::clone(&self.registry),
        full_mutation_path: new_path_prefix,
        path_action,
        variant_chain: self.variant_chain.clone(),
        depth: new_depth,
        parent_variant_signature: self.parent_variant_signature.clone(),  // NEW: inherit from parent
    })
}
```

### Step 3: Update find_knowledge() to Check Enum Signature and Return Result

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/recursion_context.rs`

**Change function signature** to return `Result` (line 256):
```rust
// Old:
pub fn find_knowledge(&self) -> Option<&'static super::mutation_knowledge::MutationKnowledge>

// New:
pub fn find_knowledge(&self) -> std::result::Result<Option<&'static super::mutation_knowledge::MutationKnowledge>, BuilderError>
```

**Update the method implementation** to check enum signature knowledge for `IndexedElement`:

```rust
pub fn find_knowledge(&self) -> std::result::Result<Option<&'static super::mutation_knowledge::MutationKnowledge>, BuilderError> {
    match &self.path_kind {
        PathKind::StructField { field_name, parent_type, .. } => {
            // Existing struct field logic - UPDATE returns to Ok(Some(...))
        }
        PathKind::IndexedElement { index, parent_type, .. } => {
            // NEW: Check if we're a child of an enum variant signature
            if let Some(signature) = &self.parent_variant_signature {
                match signature {
                    VariantSignature::Tuple(_types) => {
                        // Architectural guarantee: The index was created by enumerating
                        // this signature's types, so bounds are guaranteed valid

                        let key = KnowledgeKey::enum_variant_signature(
                            parent_type.clone(),  // enum type from PathKind
                            signature.clone(),
                            *index,
                        );

                        if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&key) {
                            tracing::debug!(
                                "Found enum signature knowledge for {parent_type}[{index}]: {:?}",
                                knowledge.example()
                            );
                            return Ok(Some(knowledge));
                        }
                    }
                    VariantSignature::Struct(_) | VariantSignature::Unit => {
                        // ARCHITECTURAL INVARIANT VIOLATION
                        // IndexedElement should only occur with Tuple signatures
                        // create_paths_for_signature() creates StructField for Struct, nothing for Unit
                        return Err(BuilderError::SystemError(Report::new(Error::InvalidState(
                            format!(
                                "IndexedElement path kind with {:?} variant signature for type {}. This indicates a bug in path generation logic.",
                                signature,
                                parent_type.display_name()
                            )
                        ))));
                    }
                }
            }
            // Fall through to exact type match
        }
        PathKind::RootValue { .. } | PathKind::ArrayElement { .. } => {
            // Existing logic - UPDATE returns to Ok(Some(...))
        }
    }

    // Exact type match as fallback
    let exact_key = KnowledgeKey::exact(self.type_name());
    Ok(BRP_MUTATION_KNOWLEDGE.get(&exact_key))
}
```

**Update all return statements** in the existing logic:
- Change `return Some(knowledge);` to `return Ok(Some(knowledge));`
- Change final return from `.map_or_else(|| None, Some)` to `Ok(BRP_MUTATION_KNOWLEDGE.get(&exact_key))`

**Update call site** in `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/builder.rs` (line 606):

```rust
// Old:
if let Some(knowledge) = ctx.find_knowledge() {
    let example = knowledge.example().clone();
    // ... rest of logic
}

// New:
let knowledge_result = ctx.find_knowledge()?;
if let Some(knowledge) = knowledge_result {
    let example = knowledge.example().clone();
    // ... rest of logic
}
```

**Why this is the correct approach**:

1. **Proper error handling**: Uses the well-established `BuilderError::SystemError(Report::new(Error::InvalidState(...)))` pattern used throughout the codebase
2. **Error propagation**: The `?` operator propagates system errors up through the call stack
3. **Semantic clarity**: Distinguishes between three cases:
   - `Ok(Some(knowledge))` = knowledge found
   - `Ok(None)` = no knowledge exists (expected, valid state)
   - `Err(BuilderError)` = system error during lookup (invalid state, bug)
4. **No bounds checking needed**: The index is architecturally guaranteed valid by construction
5. **Single call site**: Only one location (`builder.rs:606`) needs updating, making this a safe change

### Step 4: Set Parent Variant Signature in Enum Builder

**File**: `mcp/src/brp_tools/brp_type_guide/mutation_path_builder/enum_path_builder.rs`

Update `process_signature_path` to set the parent variant signature when creating child contexts.

**Current function signature (line 479-484)**:
```rust
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError>
```

**New function signature with signature parameter**:
```rust
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],
    signature: &VariantSignature,  // NEW parameter added here
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError>
```

**Set parent variant signature in function body (after line 485)**:
```rust
fn process_signature_path(
    path: PathKind,
    applicable_variants: &[VariantName],
    signature: &VariantSignature,  // NEW parameter
    ctx: &RecursionContext,
    child_examples: &mut HashMap<MutationPathDescriptor, Value>,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    let mut child_ctx = ctx.create_recursion_context(path.clone(), PathAction::Create)?;

    // NEW: Set parent variant signature context for the child
    // Note: enum type is already in child_ctx.path_kind.parent_type
    child_ctx.parent_variant_signature = Some(signature.clone());

    // Set up enum context for children - just push the variant name
    if let Some(representative_variant) = applicable_variants.first() {
        child_ctx.variant_chain.push(representative_variant.clone());
    }

    // ... rest of function unchanged ...
}
```

**Update call site in `process_children` (line 688-689)**:

Current call:
```rust
let child_paths =
    process_signature_path(path, &applicable_variants, ctx, &mut child_examples)?;
```

New call with signature parameter:
```rust
let child_paths = process_signature_path(
    path,
    &applicable_variants,
    signature,  // NEW: pass signature from variant_groups loop
    ctx,
    &mut child_examples,
)?;
```

## Testing

After implementation, verify the changes with these specific checks:

### 1. Verify AlphaMode2d enum signature knowledge is applied

**Generate type guide:**
```bash
# Use the brp_type_guide tool to get AlphaMode2d's mutation paths
# (Exact command depends on your tooling - adjust as needed)
```

**Check `.0` path example** (the critical fix):
```bash
# JSON path to check:
# type_guide["bevy_sprite::sprite::AlphaMode2d"].mutation_paths[".0"].example

# Expected value: 0.5 (not 3.1415927...)
# This confirms find_knowledge() applied enum signature knowledge for Mask(f32).0
```

**Check root example:**
```bash
# JSON path to check:
# type_guide["bevy_sprite::sprite::AlphaMode2d"].enum_example_groups[].example

# Expected: One group should have example: {"Mask": 0.5}
# This confirms the root example uses the child's corrected value
```

### 2. Verify exact type knowledge still works for other f32 fields

**Generate type guide for Transform:**
```bash
# Get Transform type guide to verify non-enum f32 fields aren't affected
```

**Check that regular f32 fields still use π:**
```bash
# JSON paths to check:
# type_guide["bevy_transform::components::transform::Transform"].mutation_paths[".translation.x"].example
# type_guide["bevy_transform::components::transform::Transform"].mutation_paths[".translation.y"].example
# type_guide["bevy_transform::components::transform::Transform"].mutation_paths[".translation.z"].example

# Expected value for all: 3.1415927... (π)
# This confirms exact type knowledge still works for non-enum contexts
```

### 3. Run mutation test to verify no crashes

```bash
# Test TilemapChunk mutation or similar enum-heavy type
# Expected: Should not crash with architectural invariant violations
# This verifies the error handling for impossible signature matches works correctly
```

### 4. Build verification

```bash
cargo build
# Expected: Compiles successfully with no errors
# Verifies all type changes (Result return type, etc.) are correct
```

## Expected Behavior

**Before (current broken state)**:
- Root example: `{"Mask": 0.5}` ✅ (correct due to HashMap override)
- `.0` path example: `3.1415927...` ❌ (wrong, still has π)

**After (fixed)**:
- Root example: `{"Mask": 0.5}` ✅ (uses child's example during assembly)
- `.0` path example: `0.5` ✅ (gets it from find_knowledge)

## Benefits of This Approach

1. **Single choke point**: ALL knowledge application happens in `find_knowledge()`
2. **Automatic consistency**: Child example and parent assembly always match
3. **Clean code**: No manual syncing, no duplicate logic
4. **Type-safe**: Leverages existing `VariantSignature` type
5. **Extensible**: Will work for struct variant fields when needed
6. **Debuggable**: Single place to add tracing/logging for knowledge application

## Migration Notes

This is a pure refactor with no breaking changes to the knowledge system itself. The `EnumVariantSignature` key and knowledge entries remain unchanged - we're just moving WHERE the knowledge is checked to a better location in the code flow.
