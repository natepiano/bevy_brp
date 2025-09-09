//! Default builder for simple types
//!
//! Handles simple types that don't need complex logic - just creates a standard mutation path
//!
//! **Recursion**: NO - Default builder handles Value types (primitives like i32, f32, String)
//! which are leaf nodes in the type tree. These cannot be decomposed further and are
//! mutated as atomic values.
use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::MutationPathBuilder;
use super::super::recursion_context::RecursionContext;
use super::super::types::MutationPathInternal;
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

pub struct DefaultMutationBuilder;

impl MutationPathBuilder for DefaultMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        _depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        tracing::error!(
            "DefaultMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
            ctx.type_name()
        );
        panic!(
            "DefaultMutationBuilder::build_paths() called directly! This should never happen when is_migrated() = true. Type: {}",
            ctx.type_name()
        );
    }

    fn is_migrated(&self) -> bool {
        true // MIGRATED!
    }

    fn collect_children(&self, _ctx: &RecursionContext) -> Vec<(String, RecursionContext)> {
        vec![] // Leaf type - no children
    }

    fn assemble_from_children(
        &self,
        _ctx: &RecursionContext,
        _children: HashMap<String, Value>,
    ) -> Value {
        // For leaf types with no children, just return null
        // Knowledge check already handled by ProtocolEnforcer
        json!(null)
    }
}
