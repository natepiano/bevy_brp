//! Default builder for simple types
//!
//! Handles simple types that don't need complex logic - just creates a standard mutation path
//!
//! **Recursion**: NO - Default builder handles Value types (primitives like i32, f32, String)
//! which are leaf nodes in the type tree. These cannot be decomposed further and are
//! mutated as atomic values.
use std::collections::HashMap;

use serde_json::Value;

use super::super::path_builder::PathBuilder;
use super::super::path_kind::{MutationPathDescriptor, PathKind};
use super::super::recursion_context::RecursionContext;
use crate::error::{Error, Result};

pub struct ValueMutationBuilder;

impl PathBuilder for ValueMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn collect_children(&self, _ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        Ok(vec![].into_iter()) // Leaf type - no children
    }

    fn assemble_from_children(
        &self,
        ctx: &RecursionContext,
        _children: HashMap<MutationPathDescriptor, Value>,
    ) -> Result<Value> {
        // Check if this Value type has serialization support
        // This is critical for the new protocol pattern - unmigrated code checks this
        // in TypeKind::build_paths but that path is never reached for migrated builders
        if !ctx.value_type_has_serialization(ctx.type_name()) {
            // Return NotMutable error for types without serialization
            // MutationPathBuilder will catch this and create the appropriate NotMutable path
            return Err(Error::NotMutable(
                super::super::NotMutableReason::MissingSerializationTraits(ctx.type_name().clone()),
            )
            .into());
        }

        // For leaf types with no children that have serialization, return NoExampleAvailable error
        // This should only be reached by types that don't have knowledge entries
        // Types with knowledge entries and TreatAsValue guidance stop recursion before getting here
        Err(
            Error::NotMutable(super::super::NotMutableReason::NoExampleAvailable(
                ctx.type_name().clone(),
            ))
            .into(),
        )
    }
}
