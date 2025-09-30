# Using shrinkwraprs for NewType Implementations in bevy_brp

**Migration Strategy: Phased**

## Current NewType Implementations

The bevy_brp workspace has several NewType wrapper structs that follow a similar pattern:

### 1. Port (mcp/src/brp_tools/port.rs)
```rust
pub struct Port(pub u16);
```
Currently implements: `Deref`, `Default`, `Display`, custom `Deserialize`, `Serialize`, `JsonSchema`

### 2. InstanceCount (mcp/src/app_tools/instance_count.rs)
```rust
pub struct InstanceCount(pub usize);
```
Currently implements: `Deref`, `Default`, `Display`, custom `Deserialize`, `Serialize`, `JsonSchema`

### 3. BrpTypeName (mcp/src/brp_tools/brp_type_guide/brp_type_name.rs)
```rust
pub struct BrpTypeName(String);
```
Currently implements: `Display`, `From` conversions, custom methods, `Serialize`, `Deserialize`

### 4. RecursionDepth (mcp/src/brp_tools/brp_type_guide/constants.rs)
```rust
pub struct RecursionDepth(usize);
```

### 5. MutationPathDescriptor (mcp/src/brp_tools/brp_type_guide/mutation_path_builder/path_kind.rs)
```rust
pub struct MutationPathDescriptor(String);
```

### 6. FullMutationPath (mcp/src/brp_tools/brp_type_guide/mutation_path_builder/types.rs)
```rust
pub struct FullMutationPath(String);
```

## How shrinkwraprs Could Simplify These

### Benefits

1. **Automatic Deref Implementation**: All types that manually implement `Deref` could get this for free
2. **AsRef/Borrow Traits**: Additional convenience traits without manual implementation
3. **Less Boilerplate**: Reduce ~20 lines of manual `Deref` implementation per type
4. **Mutable Access**: If needed, can add `#[shrinkwrap(mutable)]` for DerefMut

### Example Refactoring

#### Before (Port type):
```rust
use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize)]
pub struct Port(pub u16);

impl Deref for Port {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Port {
    fn default() -> Self {
        Self(DEFAULT_BRP_EXTRAS_PORT)
    }
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
```

#### After (with shrinkwraprs):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize, Shrinkwrap)]
pub struct Port(pub u16);

impl Default for Port {
    fn default() -> Self {
        Self(DEFAULT_BRP_EXTRAS_PORT)
    }
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
```

### Implementation Strategy

1. **Add dependency** to workspace Cargo.toml:
```toml
[workspace.dependencies]
shrinkwraprs = "0.3"
```

2. **Gradual Migration**: Start with simple types like `RecursionDepth` and `MutationPathDescriptor`

3. **Keep Custom Implementations**: Types with validation logic (Port, InstanceCount) keep their custom Deserialize implementations

4. **Test Coverage**: Ensure all existing tests pass after migration

## Types That Would Benefit Most

### High Value Targets
- **RecursionDepth**: Has custom const methods (ZERO, increment, exceeds_limit) but would still benefit from auto-derived Deref
- **MutationPathDescriptor**: Simple String wrapper
- **FullMutationPath**: Simple String wrapper

### Medium Value Targets
- **Port**: Would save Deref boilerplate but needs custom deserialize
- **InstanceCount**: Same as Port
- **BrpTypeName**: Has many custom methods, but could still benefit from auto-derived traits

## Potential Issues to Consider

1. **Compilation Time**: Adding a derive macro may slightly increase compile times
2. **Transparency**: Manual implementations are more explicit about behavior
3. **Flexibility**: Custom implementations allow fine-tuning behavior if needed later
4. **Dependencies**: Adds another dependency to maintain

## Recommendation

**YES, adopt shrinkwraprs for simple wrapper types** but:

1. Start with the simplest types (RecursionDepth, MutationPathDescriptor, FullMutationPath)
2. Keep manual implementations for types with validation logic
3. Benchmark compile time impact before full adoption
4. Document the decision in code comments

## Example Migration PR Structure

```
Phase 1: Simple types
- RecursionDepth
- MutationPathDescriptor
- FullMutationPath

Phase 2: Types with validation (if Phase 1 successful)
- Port (keep custom Deserialize)
- InstanceCount (keep custom Deserialize)

Phase 3: Complex types (evaluate based on Phase 1-2)
- BrpTypeName (many custom methods, evaluate benefit)
```

## Code Example: Full Migration of RecursionDepth

```rust
// Before
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecursionDepth(usize);

impl RecursionDepth {
    pub const ZERO: Self = Self(0);

    pub const fn increment(self) -> Self {
        Self(self.0 + 1)
    }

    pub const fn exceeds_limit(self) -> bool {
        self.0 > MAX_TYPE_RECURSION_DEPTH
    }
}

impl Deref for RecursionDepth {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// After
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Shrinkwrap)]
pub struct RecursionDepth(usize);

impl RecursionDepth {
    pub const ZERO: Self = Self(0);

    pub const fn increment(self) -> Self {
        Self(self.0 + 1)
    }

    pub const fn exceeds_limit(self) -> bool {
        self.0 > MAX_TYPE_RECURSION_DEPTH
    }
}
// Note: Deref implementation now provided automatically by Shrinkwrap
// Custom methods are preserved alongside the auto-derived traits
```

## Conclusion

shrinkwraprs would reduce boilerplate code by approximately 15-20 lines per NewType, improve maintainability, and provide additional traits like `AsRef` and `Borrow` for free. The migration can be done incrementally with minimal risk.