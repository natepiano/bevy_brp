use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::mutation_path_internal::MutationPathInternal;
use super::super::path_kind::MutationPathDescriptor;
use super::super::recursion_context::RecursionContext;
use super::super::types::PathAction;
use super::super::{BuilderError, NotMutableReason};
use crate::error::Result;
use crate::json_object::JsonObjectAccess;

/// Trait for building mutation paths for different type kinds
///
/// This trait provides type-directed dispatch for mutation path building,
/// Each type kind gets its own implementation that handles the specific logic needed.
pub trait TypeKindBuilder {
    /// The item type returned by `collect_children` - allows for
    /// `enum_builder` to return `PathKind` with `applicable_variants` where
    ///  all the other builders just return `PathKind`
    type Item;

    /// Iterator type for children
    type Iter<'a>: Iterator<Item = Self::Item>
    where
        Self: 'a;

    /// Build mutation paths with depth tracking for recursion safety
    ///
    /// This method takes a `RecursionContext` which provides all necessary information
    /// including the registry, wrapper info, enum variants, and recursion depth.
    ///
    /// Returns a `Result` containing a vector of `MutationPathInternal` representing
    /// all possible mutation paths, or a `BuilderError` if path building failed.
    fn build_paths(
        &self,
        _ctx: &RecursionContext,
    ) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
        // Implementation details here
        Ok(vec![])
    }

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
    ///
    /// Check if child paths should be included in the final mutation paths result
    ///
    /// Most types return true (default) because their child paths are valid mutation targets.
    /// Container types like Maps return false because they only expose the container itself.
    ///
    /// Example: `HashMap<String, Transform>`
    /// - Returns false: only exposes path "" with complete map {"key": {transform}}
    /// - Does NOT expose ".rotation", ".scale" etc. from the Transform values
    fn child_path_action(&self) -> PathAction {
        PathAction::Create
    }

    /// Collect `PathKind`sfor child elements
    ///
    /// contain the necessary information (field names, indices) for child
    /// identification.
    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>>;

    /// Assemble a parent value from child examples
    ///
    /// Receives `HashMap` where keys are extracted from `PathKinds` by `MutationPathBuilder`:
    /// - `StructField`: uses `field_name` as key
    /// - `IndexedElement`/`ArrayElement`: uses `index.to_string()` as key
    /// - `RootValue`: uses empty string as key
    ///
    /// Builders ONLY assemble examples - mutation status is determined by `MutationPathBuilder`.
    ///
    /// Examples:
    /// - `MapMutationBuilder`: receives {"key": `key_example`, "value": `value_example`}
    /// - `SetMutationBuilder`: receives {"items": `item_example`}
    /// - `StructBuilder`: receives {"field1": `example1`, "field2": `example2`, ...}
    fn assemble_from_children(
        &self,
        _ctx: &RecursionContext,
        _children: HashMap<MutationPathDescriptor, Value>,
    ) -> std::result::Result<Value, BuilderError> {
        // Default - not implemented for MutationPathBuilder
        Ok(json!(null))
    }

    /// Check if a collection element (`HashMap` key or `HashSet` element) is complex
    /// and return `NotMutable` error if it is
    fn check_collection_element_complexity(
        &self,
        element: &Value,
        ctx: &RecursionContext,
    ) -> std::result::Result<(), BuilderError> {
        if element.is_complex_type() {
            return Err(BuilderError::NotMutable(
                NotMutableReason::ComplexCollectionKey(ctx.type_name().clone()),
            ));
        }
        Ok(())
    }
}
