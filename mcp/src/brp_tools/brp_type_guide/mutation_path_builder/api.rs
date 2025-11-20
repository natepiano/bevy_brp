//! Support functions for the mutation path builder module
//!
//! This module contains the public API functions that external callers use to interact
//! with the mutation path builder system. These functions hide internal implementation
//! details and provide a clean interface.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::super::constants::INSERT_RESOURCE_GUIDANCE;
use super::super::constants::NO_COMPONENT_EXAMPLE_TEMPLATE;
use super::super::constants::NO_RESOURCE_EXAMPLE_TEMPLATE;
use super::super::constants::OPERATION_INSERT;
use super::super::constants::OPERATION_SPAWN;
use super::super::constants::REFLECT_TRAIT_COMPONENT;
use super::super::constants::REFLECT_TRAIT_RESOURCE;
use super::super::constants::SPAWN_COMPONENT_GUIDANCE;
use super::super::type_kind::TypeKind;
use super::enum_builder::select_preferred_example;
use super::path_builder::recurse_mutation_paths;
use super::path_example::PathExample;
use super::path_kind::PathKind;
use super::recursion_context::RecursionContext;
use super::types::Example;
use super::types::MutationPathExternal;
use super::types::SpawnInsertExample;
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

/// Extract spawn/insert example with guidance for AI agents
///
/// Returns `None` if the type is neither a Component nor a Resource.
/// Otherwise, returns a `SpawnInsertExample` with appropriate guidance and example.
pub fn extract_spawn_insert_example(
    mutation_paths: &[MutationPathExternal],
    reflect_traits: &[String],
) -> Option<SpawnInsertExample> {
    // Check if type is Component or Resource
    let is_component = reflect_traits.iter().any(|t| t == REFLECT_TRAIT_COMPONENT);
    let is_resource = reflect_traits.iter().any(|t| t == REFLECT_TRAIT_RESOURCE);

    if !is_component && !is_resource {
        return None;
    }

    // Extract root path example
    let root_path = mutation_paths.iter().find(|p| (*p.path).is_empty())?;

    let example = match &root_path.path_example {
        PathExample::Simple(ex) => ex.clone(),
        PathExample::EnumRoot { groups, .. } => {
            select_preferred_example(groups).unwrap_or(Example::NotApplicable)
        },
    };

    // Build appropriate variant based on type
    if is_component {
        let agent_guidance = if matches!(example, Example::NotApplicable) {
            NO_COMPONENT_EXAMPLE_TEMPLATE.replace("{}", OPERATION_SPAWN)
        } else {
            SPAWN_COMPONENT_GUIDANCE.to_string()
        };

        Some(SpawnInsertExample::SpawnExample {
            agent_guidance,
            example,
        })
    } else {
        let agent_guidance = if matches!(example, Example::NotApplicable) {
            NO_RESOURCE_EXAMPLE_TEMPLATE.replace("{}", OPERATION_INSERT)
        } else {
            INSERT_RESOURCE_GUIDANCE.to_string()
        };

        Some(SpawnInsertExample::ResourceExample {
            agent_guidance,
            example,
        })
    }
}
