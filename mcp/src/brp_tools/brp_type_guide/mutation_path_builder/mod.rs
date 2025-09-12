mod builders;
mod mutation_knowledge;
mod not_mutatable_reason;
mod path_kind;
mod protocol_enforcer;
mod recursion_context;
mod type_kind;
mod types;

use std::collections::HashMap;

pub use builders::EnumVariantInfo;
pub use mutation_knowledge::KnowledgeKey;
pub use not_mutatable_reason::NotMutableReason;
pub use path_kind::PathKind;
pub use recursion_context::RecursionContext;
use serde_json::{Value, json};
pub use type_kind::TypeKind;
pub use types::{MutationPath, MutationPathInternal, MutationStatus, PathAction};

use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

/// Trait for building mutation paths for different type kinds
///
/// This trait provides type-directed dispatch for mutation path building,
/// replacing the large conditional match statement with clean separation of concerns.
/// Each type kind gets its own implementation that handles the specific logic needed.
pub trait MutationPathBuilder {
    /// Build mutation paths with depth tracking for recursion safety
    ///
    /// This method takes a `MutationPathContext` which provides all necessary information
    /// including the registry, wrapper info, and enum variants, plus a `RecursionDepth`
    /// parameter to track recursion depth and prevent infinite loops.
    ///
    /// Returns a `Result` containing a vector of `MutationPathInternal` representing
    /// all possible mutation paths, or an error if path building failed.
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>>;

    /// Build example using depth-first traversal - ensures children complete before parent
    /// Default implementation handles knowledge lookup and enforces traversal ordering
    ///
    /// **DEPTH-FIRST PATTERN Critical for Correctness**:
    /// - STEP 1: RECURSE TO ALL CHILDREN FIRST - complete child traversal before parent assembly
    /// - STEP 2: CONSTRUCT PARENT AFTER CHILD COMPLETION - bottom-up assembly
    ///
    /// **Examples of bottom-up construction**:
    /// - Array `[f32; 3]`: Build f32 example (10.5), then construct [10.5, 10.5, 10.5]
    /// - Struct `Person`: Build name ("John") and address subfields first, then assemble {"name":
    ///   "John", "address": {...}}
    /// - Vec<Transform>: Build Transform example first, then wrap in Vec [transform1, transform2]
    ///
    /// **CRITICAL**: This example represents the complete subtree from this level down
    /// - If this is `.address.street`, the example is just "123 Main St"
    /// - If this is `.address`, the example is the complete address object {"street": "123 Main
    ///   St", "city": "Portland"}
    /// - If this is root level, the example becomes the spawn format
    // fn build_example_with_knowledge(&self, ctx: &RecursionContext, depth: RecursionDepth) ->
    // Value {     // First check BRP_MUTATION_KNOWLEDGE for hardcoded examples
    //     if let Some(example) = KnowledgeKey::find_example_for_type(ctx.type_name()) {
    //         return example;
    //     }
    //     self.build_schema_example(ctx, depth)
    // }
    ///   Build example from schema - implemented by each builder for their specific type.
    ///
    /// Each builder focuses ONLY on type-specific assembly logic.
    /// The trait's `build_example_with_knowledge()` handles all common patterns:
    /// - Knowledge lookup (`BRP_MUTATION_KNOWLEDGE`)
    /// - Depth checking and recursion
    /// - Type dispatch to child builders
    fn build_schema_example(&self, ctx: &RecursionContext, depth: RecursionDepth) -> Value {
        // Default: delegate to ExampleBuilder for now
        use super::example_builder::ExampleBuilder;
        ExampleBuilder::build_example(ctx.type_name(), &ctx.registry, depth)
    }

    // NEW METHODS FOR PROTOCOL MIGRATION

    /// Indicates if this builder has been migrated to the new protocol
    fn is_migrated(&self) -> bool {
        false // Default: not migrated
    }

    /// Check if child paths should be included in the final mutation paths result
    ///
    /// Most types return true (default) because their child paths are valid mutation targets.
    /// Container types like Maps return false because they only expose the container itself.
    ///
    /// Example: `HashMap<String, Transform>`
    /// - Returns false: only exposes path "" with complete map {"key": {transform}}
    /// - Does NOT expose ".rotation", ".scale" etc. from the Transform values
    fn include_child_paths(&self) -> bool {
        true // Default: most types want child paths for field mutation
    }

    /// Collect `PathKinds` for child elements
    ///
    /// Migrated builders should return `PathKind` without creating contexts.
    /// `PathKind` contain the necessary information (field names, indices) for child
    /// identification.
    fn collect_children(&self, _ctx: &RecursionContext) -> Result<Vec<PathKind>> {
        Ok(vec![]) // Default: no children (leaf types)
    }

    /// Assemble a parent value from child examples
    ///
    /// Receives `HashMap` where keys are extracted from `PathKinds` by `ProtocolEnforcer`:
    /// - `StructField`: uses `field_name` as key
    /// - `IndexedElement`/`ArrayElement`: uses `index.to_string()` as key
    /// - `RootValue`: uses empty string as key
    ///
    /// Builders ONLY assemble examples - mutation status is determined by `ProtocolEnforcer`.
    ///
    /// Examples:
    /// - `MapMutationBuilder`: receives {"key": `key_example`, "value": `value_example`}
    /// - `SetMutationBuilder`: receives {"items": `item_example`}
    /// - `StructBuilder`: receives {"field1": `example1`, "field2": `example2`, ...}
    fn assemble_from_children(
        &self,
        _ctx: &RecursionContext,
        _children: HashMap<String, Value>,
    ) -> Result<Value> {
        // Default: return null for unmigrated builders
        Ok(json!(null))
    }

    /// Check if a collection element (`HashMap` key or `HashSet` element) is complex
    /// and return `NotMutatable` error if it is
    fn check_collection_element_complexity(
        &self,
        element: &Value,
        ctx: &RecursionContext,
    ) -> Result<()> {
        use crate::error::Error;
        use crate::json_object::JsonObjectAccess;
        if element.is_complex_type() {
            return Err(Error::NotMutatable(NotMutableReason::ComplexCollectionKey(
                ctx.type_name().clone(),
            ))
            .into());
        }
        Ok(())
    }
}
