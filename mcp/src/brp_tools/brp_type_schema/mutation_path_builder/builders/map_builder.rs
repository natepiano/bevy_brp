//! Builder for Map types (`HashMap`, `BTreeMap`, etc.)
//!
//! Like Sets, Maps can only be mutated at the top level (replacing the entire map).
//! Maps don't support individual key mutations through BRP's reflection path system.
//!
//! The BRP reflection parser expects integer indices in brackets (e.g., `[0]`) for arrays,
//! not string keys (e.g., `["key"]`) for maps. Because of this limitation, we generate
//! a single terminal mutation path for the entire map field.

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use super::super::MutationPathBuilder;
use super::super::mutation_support::MutationSupport;
use super::super::recursion_context::RecursionContext;
use super::super::types::{MutationPathInternal, MutationStatus};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use crate::brp_tools::brp_type_schema::response_types::BrpTypeName;
use crate::error::Result;

pub struct MapMutationBuilder;

impl MutationPathBuilder for MapMutationBuilder {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        if ctx.require_schema().is_none() {
            return Ok(vec![Self::build_not_mutatable_path(
                ctx,
                MutationSupport::NotInRegistry(ctx.type_name().clone()),
            )]);
        }

        // Maps can only be mutated at the top level - no individual key access
        Ok(vec![Self::build_map_mutation_path(ctx, depth)])
    }
}

impl MapMutationBuilder {
    /// Build map example using extracted logic - creates example key-value pairs
    /// This is the static method version that calls TypeInfo for key/value types
    pub fn build_map_example_static(
        _schema: &Value,
        _registry: &HashMap<BrpTypeName, Value>,
        _depth: RecursionDepth,
    ) -> Value {
        // Maps are complex - for now just return a simple example
        // TODO: Extract key/value types from schema if needed
        let mut map = Map::new();
        map.insert("example_key".to_string(), json!("example_value"));
        json!(map)
    }

    /// Build a mutation path for the entire Map field
    fn build_map_mutation_path(
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> MutationPathInternal {
        use crate::brp_tools::brp_type_schema::type_info::TypeInfo;

        // Generate example value for the Map type
        let example = TypeInfo::build_type_example(ctx.type_name(), &ctx.registry, depth);

        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example,
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutatable,
            error_reason: None,
        }
    }

    /// Build a not-mutatable path with structured error details
    fn build_not_mutatable_path(
        ctx: &RecursionContext,
        support: MutationSupport,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path:            ctx.mutation_path.clone(),
            example:         json!({
                "NotMutatable": format!("{support}"),
                "agent_directive": format!("This map type cannot be mutated - {support}")
            }),
            type_name:       ctx.type_name().clone(),
            path_kind:       ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutatable,
            error_reason:    Option::<String>::from(&support),
        }
    }
}
