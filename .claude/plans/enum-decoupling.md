# Enum Decoupling Plan

## Status: DRAFT - To be reviewed after adhoc review completion

## Problem: Implicit Variant Item Detection

### Current Situation
The code uses `applicable_variants()` as a sentinel to detect whether an item is a variant field, which is indirect and confusing:

```rust
// Current inference-based approach
let variant_info = item.applicable_variants().map(|v| v.to_vec());
if let Some(variants) = variant_info {
    // Inferred: this must be a variant item
    child_ctx.enum_context = Some(EnumContext::Child);
}
```

### Issues with Current Approach
1. **Indirect inference** - Using presence of variant data to infer item type
2. **Not explicit intent** - `applicable_variants()` doesn't directly state "I am a variant field"
3. **Confusing dual mechanism**:
   - `applicable_variants()` - marks immediate variant children (not propagated)
   - `variant_chain` - tracks the full path of variants needed (IS propagated)

### Proposed Solution: Explicit Item Context

#### Option 1: ItemContext Enum
```rust
enum ItemContext {
    VariantField(Vec<VariantName>),  // Direct child of enum variant
    RegularField,                     // Normal struct/tuple field
}

trait MaybeVariants {
    fn item_context(&self) -> ItemContext;
    // Remove applicable_variants() method
}
```

#### Option 2: Discriminated PathBuilderItem
```rust
enum PathBuilderItem {
    VariantItem {
        path_kind: PathKind,
        variants: Vec<VariantName>,
    },
    RegularItem {
        path_kind: PathKind,
    },
}
```

Then in `process_all_children`:
```rust
match item {
    PathBuilderItem::VariantItem { path_kind, variants } => {
        // Explicitly a variant item - set EnumContext::Child
    }
    PathBuilderItem::RegularItem { path_kind } => {
        // Check if it's an enum type, apply normal logic
    }
}
```

### Benefits
1. **Explicit intent** - Clear distinction between variant fields and regular fields
2. **Type safety** - Compiler enforces handling of both cases
3. **Self-documenting** - Code clearly shows what each item represents
4. **No inference needed** - Direct pattern matching instead of checking for Some/None

## Next Steps
1. Complete adhoc review of current call flow
2. Revisit this plan with full context
3. Decide between Option 1 and Option 2
4. Implement chosen solution