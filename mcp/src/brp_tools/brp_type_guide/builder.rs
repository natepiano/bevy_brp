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
    AGENT_GUIDANCE, ENTITY_WARNING, ERROR_GUIDANCE, REFLECT_TRAIT_COMPONENT,
    REFLECT_TRAIT_RESOURCE, RecursionDepth, TYPE_BEVY_ENTITY,
};
use super::mutation_path_builder;
use super::mutation_path_builder::{
    MutationPath, MutationPathInternal, PathKind, RecursionContext, recurse_mutation_paths,
};
use super::response_types::{BrpTypeName, SchemaInfo};
use super::type_kind::TypeKind;
use crate::error::Result;
use crate::json_object::{IntoStrings, JsonObjectAccess};
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
        registry: Arc<HashMap<BrpTypeName, Value>>,
    ) -> Result<Self> {
        // Look up the type in the registry
        let registry_schema = match registry.get(&brp_type_name) {
            Some(schema) => schema,
            None => {
                // Not found is a valid result, not an error
                return Ok(Self::not_found_in_registry(
                    brp_type_name,
                    "Type not found in registry".to_string(),
                ));
            }
        };

        // Build mutation paths to determine actual mutation capability
        let mutation_paths_vec =
            Self::build_mutation_paths(&brp_type_name, registry_schema, Arc::clone(&registry))?;

        let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec, &registry);

        // Extract spawn format if type is spawnable (Component or Resource)
        let spawn_format =
            Self::extract_spawn_format_if_spawnable(registry_schema, &mutation_paths);

        // Extract schema info from registry
        let schema_info = Self::extract_schema_info(registry_schema);

        // Generate agent guidance (with Entity warning if needed)
        let agent_guidance = Self::generate_agent_guidance(&mutation_paths)?;

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

    /// Generate agent guidance with Entity warning if type contains Entity fields
    ///
    /// Checks all mutation paths for Entity types and adds a warning about using
    /// valid Entity IDs from the running app.
    fn generate_agent_guidance(mutation_paths: &HashMap<String, MutationPath>) -> Result<String> {
        // Check if any mutation path contains Entity type
        let has_entity = mutation_paths
            .values()
            .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));

        if has_entity {
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
            Ok(format!("{AGENT_GUIDANCE}{entity_suffix}"))
        } else {
            Ok(AGENT_GUIDANCE.to_string())
        }
    }

    /// Extract spawn format if the type is spawnable (Component or Resource)
    ///
    /// Encapsulates the logic for determining whether a type should have a spawn format
    /// and extracting it from the mutation paths.
    fn extract_spawn_format_if_spawnable(
        registry_schema: &Value,
        mutation_paths: &HashMap<String, MutationPath>,
    ) -> Option<Value> {
        // Check if type is spawnable (has Component or Resource trait)
        let reflect_types = registry_schema
            .get_field_array(SchemaField::ReflectTypes)
            .map(|arr| arr.iter().filter_map(Value::as_str).into_strings())
            .unwrap_or_default();

        let is_spawnable = reflect_types.iter().any(|trait_name| {
            trait_name == REFLECT_TRAIT_COMPONENT || trait_name == REFLECT_TRAIT_RESOURCE
        });

        if is_spawnable {
            Self::extract_spawn_format_from_paths(mutation_paths)
        } else {
            None
        }
    }

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

    /// Build mutation paths for a type
    fn build_mutation_paths(
        brp_type_name: &BrpTypeName,
        registry_schema: &Value,
        registry: Arc<HashMap<BrpTypeName, Value>>,
    ) -> Result<Vec<MutationPathInternal>> {
        let type_kind = TypeKind::from_schema(registry_schema);

        // Create root `PathKind` for this type - the root has a mutation path of ""
        let path_kind = PathKind::new_root_value(brp_type_name.clone());

        let ctx = RecursionContext::new(path_kind, Arc::clone(&registry));

        // Use the single, recursive dispatch point for all `TypeKind`s
        let result = recurse_mutation_paths(type_kind, &ctx, RecursionDepth::ZERO)?;

        Ok(result)
    }

    /// Convert `Vec<MutationPath>` to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(
        paths: &[MutationPathInternal],
        registry: &HashMap<BrpTypeName, Value>,
    ) -> HashMap<String, MutationPath> {
        paths
            .iter()
            .map(|path| {
                let path_info = MutationPath::from_mutation_path_internal(path, registry);
                // Keep empty path as empty for root mutations
                // BRP expects empty string for root replacements, not "."
                let key = (*path.full_mutation_path).clone();
                (key, path_info)
            })
            .collect()
    }

    /// Extract schema information from registry schema
    fn extract_schema_info(registry_schema: &Value) -> Option<SchemaInfo> {
        let type_kind = registry_schema
            .get_field_str(SchemaField::Kind)
            .and_then(|s| TypeKind::from_str(s).ok());

        let properties = registry_schema.get_field(SchemaField::Properties).cloned();

        let required = registry_schema
            .get_field_array(SchemaField::Required)
            .map(|arr| arr.iter().filter_map(Value::as_str).into_strings());

        let module_path = registry_schema.get_field_string(SchemaField::ModulePath);

        let crate_name = registry_schema.get_field_string(SchemaField::CrateName);

        // Extract reflection traits
        let reflect_traits = registry_schema
            .get_field_array(SchemaField::ReflectTypes)
            .map(|arr| arr.iter().filter_map(Value::as_str).into_strings());

        Some(SchemaInfo {
            type_kind,
            properties,
            required,
            module_path,
            crate_name,
            reflect_traits,
        })
    }
}
