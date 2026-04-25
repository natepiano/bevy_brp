//! Default `PathBuilder` for simple types
//!
//! Handles simple types that don't need complex logic - just creates a standard mutation path
//!
//! **Recursion**: NO - Default builder handles Value types (primitives like i32, f32, String)
//! which are leaf nodes in the type tree. These cannot be decomposed further and are
//! mutated as atomic values.
use std::collections::HashMap;

use serde_json::Value;

use super::TypeKindBuilder;
use super::ValueMutationBuilder;
use crate::brp_tools::brp_type_guide::mutation_path_builder::BuilderError;
use crate::brp_tools::brp_type_guide::mutation_path_builder::NotMutableReason;
use crate::brp_tools::brp_type_guide::mutation_path_builder::path_example::Example;
use crate::brp_tools::brp_type_guide::mutation_path_builder::path_kind::MutationPathDescriptor;
use crate::brp_tools::brp_type_guide::mutation_path_builder::path_kind::PathKind;
use crate::brp_tools::brp_type_guide::mutation_path_builder::recursion_context::RecursionContext;
use crate::error::Result;

impl TypeKindBuilder for ValueMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn collect_children(&self, _: &RecursionContext) -> Result<Self::Iter<'_>> {
        Ok(vec![].into_iter()) // Leaf type - no children
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        _: HashMap<MutationPathDescriptor, Example>,
    ) -> std::result::Result<Value, BuilderError> {
        // For leaf types without mutation knowledge, return appropriate reason
        Err(BuilderError::NotMutable(
            NotMutableReason::NoExampleAvailable(ctx.type_name().clone()),
        ))
    }
}
