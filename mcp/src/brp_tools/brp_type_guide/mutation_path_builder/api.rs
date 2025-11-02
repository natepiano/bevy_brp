//! Support functions for the mutation path builder module
//!
//! This module contains the public API functions that external callers use to interact
//! with the mutation path builder system. These functions hide internal implementation
//! details and provide a clean interface.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::super::type_kind::TypeKind;
use super::enum_builder::select_preferred_example;
use super::path_builder::recurse_mutation_paths;
use super::path_example::PathExample;
use super::path_kind::PathKind;
use super::recursion_context::RecursionContext;
use super::types::Example;
use super::types::MutationPathExternal;
use crate::error::Error;
use crate::error::Result;

/// Entry point for building mutation paths from a type name and registry
///
/// This is the public facade that hides internal implementation details (`PathKind`,
/// `RecursionContext`, `MutationPathInternal`) from external callers. It takes simple
/// inputs and returns the final external format ready for use.
pub fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<Vec<MutationPathExternal>> {
    // Look up schema to determine TypeKind
    let schema = registry
        .get(type_name)
        .ok_or_else(|| Error::General(format!("Type {type_name} not found in registry")))?;

    let type_kind = TypeKind::from_schema(schema);

    // Create internal context (hidden from caller)
    let path_kind = PathKind::new_root_value(type_name.clone());
    let ctx = RecursionContext::new(path_kind, Arc::clone(&registry));

    // Dispatch to the recursive builder
    let internal_paths = recurse_mutation_paths(type_kind, &ctx)?;

    // Convert internal representation to external format before returning
    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry)
        })
        .collect();

    Ok(external_paths)
}

/// Extract spawn format from the root mutation path
///
/// This is a helper function for extracting the example value from the root path ("")
/// which is used as the spawn format for types that support spawn/insert operations.
pub fn extract_spawn_format(mutation_paths: &[MutationPathExternal]) -> Option<Value> {
    mutation_paths
        .iter()
        .find(|p| (*p.path).is_empty())
        .and_then(|root_path| match &root_path.path_example {
            PathExample::Simple(example) => match example {
                Example::Json(val) => Some(val.clone()),
                Example::OptionNone => Some(Value::Null),
                Example::NotApplicable => None,
            },
            PathExample::EnumRoot { groups, .. } => {
                select_preferred_example(groups).map(|ex| ex.to_value())
            }
        })
}
