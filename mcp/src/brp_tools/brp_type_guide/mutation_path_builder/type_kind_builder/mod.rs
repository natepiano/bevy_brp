mod array_builder;
mod list_builder;
mod map_builder;
mod set_builder;
mod struct_builder;
mod tuple_builder;
mod value_builder;

use std::collections::HashMap;

pub(super) use array_builder::ArrayMutationBuilder;
pub(super) use list_builder::ListMutationBuilder;
pub(super) use map_builder::MapMutationBuilder;
use serde_json::Value;
use serde_json::json;
pub(super) use set_builder::SetMutationBuilder;
pub(super) use struct_builder::StructMutationBuilder;
pub(super) use tuple_builder::TupleMutationBuilder;
pub(super) use value_builder::ValueMutationBuilder;

use super::BuilderError;
use super::NotMutableReason;
use super::mutation_path_internal::MutationPathInternal;
use super::path_example::Example;
use super::path_kind::MutationPathDescriptor;
use super::recursion_context::PathAction;
use super::recursion_context::RecursionContext;
use crate::error::Result;
use crate::support::JsonObjectAccess;

/// Trait for building mutation paths for different type kinds.
///
/// This trait is the contract for the closed `type_kind_builders` family:
/// the child modules in this folder provide the concrete implementations.
pub(super) trait TypeKindBuilder {
    /// The item type returned by `collect_children` - allows for
    /// `enum_builder` to return `PathKind` with `applicable_variants` where
    /// all the other builders just return `PathKind`
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
        _: &RecursionContext,
    ) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
        Ok(vec![])
    }

    /// Check if child paths should be included in the final mutation paths result.
    fn child_path_action(&self) -> PathAction { PathAction::Create }

    /// Collect `PathKind`s for child elements.
    fn collect_children(&self, context: &RecursionContext) -> Result<Self::Iter<'_>>;

    /// Assemble a parent value from child examples.
    fn assemble_from_children(
        &self,
        _: &RecursionContext,
        _: HashMap<MutationPathDescriptor, Example>,
    ) -> std::result::Result<Value, BuilderError> {
        Ok(json!(null))
    }

    /// Check if a collection element (`HashMap` key or `HashSet` element) is complex
    /// and return `NotMutable` error if it is.
    fn check_collection_element_complexity(
        &self,
        element: &Value,
        context: &RecursionContext,
    ) -> std::result::Result<(), BuilderError> {
        if element.is_complex_type() {
            return Err(BuilderError::NotMutable(
                NotMutableReason::ComplexCollectionKey(context.type_name().clone()),
            ));
        }
        Ok(())
    }
}
