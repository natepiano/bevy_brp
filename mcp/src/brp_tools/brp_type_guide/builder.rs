//! Orchestrates type information assembly for AI agents
//!
//! This module builds `TypeGuide` responses by coordinating multiple subsystems:
//! - Mutation path generation (via `TypeKind` dispatch)
//! - Spawn format extraction
//! - Schema metadata extraction
//! - Entity-aware guidance generation
//!
//! The `TypeGuide` struct is the final assembled response sent to MCP clients.
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc; // unused import for testing // another unused import for testing

use serde::Serialize;
use serde_json::Value;

use super::constants::{
    AGENT_GUIDANCE, ENTITY_WARNING, ERROR_GUIDANCE, RecursionDepth, TYPE_BEVY_ENTITY,
};
use super::mutation_path_builder;
use super::mutation_path_builder::{
    MutationPath, MutationPathInternal, PathKind, RecursionContext, recurse_mutation_paths,
};
use super::response_types::{BrpTypeName, ReflectTrait, SchemaInfo};
use super::type_kind::TypeKind;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// this is all of the information we provide about a type
/// we serialize this to our output - and we call it `type_guide`
/// because that's what's on the tin
#[derive(Debug, Clone, Serialize)]
pub struct TypeGuide {
    /// Guidance for AI agents about using mutation paths
    pub agent_guidance: String,
    /// Fully-qualified type name
    pub type_name:      BrpTypeName,
    /// Whether the type is registered in the Bevy registry
    pub in_registry:    bool,
    /// Mutation paths available for this type - using same format as V1
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub mutation_paths: HashMap<String, MutationPath>,
    /// Example values for spawn/insert operations (currently empty to match V1)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub example_values: HashMap<String, Value>,
    /// Example format for spawn/insert operations when supported
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_format:   Option<Value>,
    /// Schema information from the registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_info:    Option<SchemaInfo>,
    /// Type information for direct fields (struct fields only, one level deep)
    /// Error message if discovery failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:          Option<String>,
}

impl TypeGuide {
    /// Builder method to create ``TypeGuide`` from schema data
    pub fn from_registry_schema(
        brp_type_name: BrpTypeName,
        registry_schema: &Value,
        registry: Arc<HashMap<BrpTypeName, Value>>,
    ) -> Result<Self> {
        // Extract reflection traits
        let reflect_types = Self::extract_reflect_types(registry_schema);

        // Build mutation paths to determine actual mutation capability
        let mutation_paths_vec =
            Self::build_mutation_paths(&brp_type_name, registry_schema, Arc::clone(&registry))?;

        let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec, &registry);

        // Build spawn format from root path mutation example - for components and resources
        let has_component = reflect_types.contains(&ReflectTrait::Component);
        let has_resource = reflect_types.contains(&ReflectTrait::Resource);

        let spawn_format = if has_component || has_resource {
            Self::extract_spawn_format_from_paths(&mutation_paths)
        } else {
            None
        };

        // Extract schema info from registry
        let schema_info = Self::extract_schema_info(registry_schema);

        // Generate agent warning based on whether Entity type is present
        let has_entity = mutation_paths
            .values()
            .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));
        let agent_guidance = if has_entity {
            // Get the Entity example value from mutation knowledge
            use super::mutation_path_builder::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
            let entity_example = BRP_MUTATION_KNOWLEDGE
                .get(&KnowledgeKey::exact(TYPE_BEVY_ENTITY))
                .and_then(|knowledge| knowledge.example().as_u64())
                .ok_or_else(|| {
                    crate::error::Error::InvalidState(
                        "Entity type knowledge missing or invalid in BRP_MUTATION_KNOWLEDGE"
                            .to_string(),
                    )
                })?;

            let entity_suffix = ENTITY_WARNING.replace("{}", &entity_example.to_string());
            format!("{AGENT_GUIDANCE}{entity_suffix}")
        } else {
            AGENT_GUIDANCE.to_string()
        };

        Ok(Self {
            type_name: brp_type_name,
            in_registry: true,
            mutation_paths,
            example_values: HashMap::new(), // V1 always has this empty
            spawn_format,
            schema_info,
            agent_guidance,
            error: None,
        })
    }

    /// Builder method to create `TypeGuide` for type not found in registry
    pub fn not_found_in_registry(type_name: BrpTypeName, error_msg: String) -> Self {
        Self {
            type_name,
            in_registry: false,
            mutation_paths: HashMap::new(),
            example_values: HashMap::new(),
            spawn_format: None,
            schema_info: None,
            agent_guidance: AGENT_GUIDANCE.to_string(),
            error: Some(error_msg),
        }
    }

    /// Builder method to create `TypeGuide` for type that failed during processing
    ///
    /// This is used when a type is found in the registry but `from_registry_schema()`
    /// fails during mutation path building or other processing steps.
    pub fn processing_failed(type_name: BrpTypeName, error_msg: String) -> Self {
        Self {
            type_name,
            in_registry: true, // Type WAS found in registry
            mutation_paths: HashMap::new(),
            example_values: HashMap::new(),
            spawn_format: None,
            schema_info: None,
            agent_guidance: ERROR_GUIDANCE.to_string(),
            error: Some(error_msg),
        }
    }

    // Private helper methods

    /// Extract spawn format from root mutation path
    /// Uses the root path `""` example as the spawn format for consistency
    /// Should only be called for types that support spawn/insert operations
    fn extract_spawn_format_from_paths(
        mutation_paths: &HashMap<String, MutationPath>,
    ) -> Option<Value> {
        mutation_paths.get("").and_then(|root_path| {
            // Handle both the new `example` field and the legacy `examples` array
            root_path.example.as_ref().map_or_else(
                || {
                    // Use the shared utility to prefer non-unit variants
                    mutation_path_builder::select_preferred_example(&root_path.examples)
                },
                |example| Some(example.clone()),
            )
        })
    }

    /// Build mutation paths for a type using the trait system
    fn build_mutation_paths(
        brp_type_name: &BrpTypeName,
        registry_schema: &Value,
        registry: Arc<HashMap<BrpTypeName, Value>>,
    ) -> Result<Vec<MutationPathInternal>> {
        let type_kind = TypeKind::from_schema(registry_schema);

        tracing::debug!(
            "build_mutation_paths: {} determined as TypeKind::{:?}",
            brp_type_name,
            type_kind
        );

        // Create root context for this type
        let path_kind = PathKind::new_root_value(brp_type_name.clone());
        let ctx = RecursionContext::new(path_kind, Arc::clone(&registry));

        // Use the single dispatch point
        let result = recurse_mutation_paths(type_kind, &ctx, RecursionDepth::ZERO)?;

        Ok(result)
    }

    /// Convert `Vec<MutationPath>` to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(
        paths: &[MutationPathInternal],
        registry: &HashMap<BrpTypeName, Value>,
    ) -> HashMap<String, MutationPath> {
        let mut result = HashMap::new();

        for path in paths {
            // Debug logging for enum root examples
            if path.full_mutation_path.is_empty() && path.enum_example_groups.is_some() {
                tracing::debug!(
                    "Converting root path for {} with {} enum examples",
                    path.type_name,
                    path.enum_example_groups.as_ref().map_or(0, Vec::len)
                );
            }

            // Create MutationPathInfo from MutationPath
            let path_info = MutationPath::from_mutation_path_internal(path, registry);

            // Debug log the result
            if path.full_mutation_path.is_empty() && !path_info.examples.is_empty() {
                tracing::debug!(
                    "After conversion: root path has {} examples in MutationPath",
                    path_info.examples.len()
                );
            }

            // Keep empty path as empty for root mutations
            // BRP expects empty string for root replacements, not "."
            let key = (*path.full_mutation_path).clone();

            result.insert(key, path_info);
        }

        result
    }

    /// Extract enum information from schema
    /// Extract reflect types from a registry schema
    fn extract_reflect_types(registry_schema: &Value) -> Vec<ReflectTrait> {
        registry_schema
            .get_field(SchemaField::ReflectTypes)
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| s.parse::<ReflectTrait>().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract schema information from registry schema
    fn extract_schema_info(registry_schema: &Value) -> Option<SchemaInfo> {
        let type_kind = registry_schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| TypeKind::from_str(s).ok());

        let properties = registry_schema.get_field(SchemaField::Properties).cloned();

        let required = registry_schema
            .get_field(SchemaField::Required)
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });

        let module_path = registry_schema
            .get_field(SchemaField::ModulePath)
            .and_then(Value::as_str)
            .map(String::from);

        let crate_name = registry_schema
            .get_field(SchemaField::CrateName)
            .and_then(Value::as_str)
            .map(String::from);

        // Extract reflection traits
        let reflect_types = Self::extract_reflect_types(registry_schema);

        // Only return SchemaInfo if we have at least some information
        if type_kind.is_some()
            || properties.is_some()
            || required.is_some()
            || module_path.is_some()
            || crate_name.is_some()
            || !reflect_types.is_empty()
        {
            Some(SchemaInfo {
                type_kind,
                properties,
                required,
                module_path,
                crate_name,
                reflect_types: Some(reflect_types),
            })
        } else {
            None
        }
    }
}
