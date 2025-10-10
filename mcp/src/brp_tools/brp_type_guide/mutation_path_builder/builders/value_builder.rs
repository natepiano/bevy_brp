//! Default `PathBuilder` for simple types
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
use super::super::{BuilderError, NotMutableReason};
use crate::error::Result;

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
    ) -> std::result::Result<Value, BuilderError> {
        // For leaf types without mutation knowledge, return appropriate reason
        Err(BuilderError::NotMutable(
            NotMutableReason::NoExampleAvailable(ctx.type_name().clone()),
        ))
    }
}
