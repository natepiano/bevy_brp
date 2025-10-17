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
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use super::constants::{
    AGENT_GUIDANCE, ENTITY_WARNING, ERROR_GUIDANCE, REFLECT_TRAIT_COMPONENT,
    REFLECT_TRAIT_RESOURCE, TYPE_BEVY_ENTITY,
};
use super::mutation_path_builder::{self, MutationPathExternal};
use super::response_types::{BrpTypeName, SchemaInfo};
use super::type_kind::TypeKind;
use super::type_knowledge::TypeKnowledge;
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
    pub mutation_paths: HashMap<String, MutationPathExternal>,
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
    pub fn build(
        brp_type_name: BrpTypeName,
        registry: Arc<HashMap<BrpTypeName, Value>>,
    ) -> Result<Self> {
        // Look up the type in the registry
        let Some(registry_schema) = registry.get(&brp_type_name) else {
            // Not found is a valid result, not an error
            return Ok(Self::not_found_in_registry(
                brp_type_name,
                "Type not found in registry".to_string(),
            ));
        };

        // Build mutation paths to determine actual mutation capability
        let mutation_paths =
            mutation_path_builder::build_mutation_paths(&brp_type_name, Arc::clone(&registry))?;

        // Extract spawn format if type is spawnable (Component or Resource)
        let spawn_format =
            Self::extract_spawn_format_if_spawnable(registry_schema, &mutation_paths);

        // Extract schema info from registry
        let schema_info = Some(Self::extract_schema_info(registry_schema));

        // Generate agent guidance (with Entity warning if needed)
        let agent_guidance = Self::generate_agent_guidance(&mutation_paths)?;

        Ok(Self {
            type_name: brp_type_name,
            in_registry: true,
            mutation_paths,
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
            spawn_format: None,
            schema_info: None,
            agent_guidance: ERROR_GUIDANCE.to_string(),
            error: Some(error_msg),
        }
    }

    /// Generate agent guidance with Entity warning if type contains Entity fields
    ///
    /// Checks all mutation paths for Entity types and adds a warning about using
    /// valid Entity IDs from the running app.
    fn generate_agent_guidance(
        mutation_paths: &HashMap<String, MutationPathExternal>,
    ) -> Result<String> {
        // Check if any mutation path contains Entity type
        let has_entity = mutation_paths
            .values()
            .any(|path| path.path_info.type_name.as_str().contains(TYPE_BEVY_ENTITY));

        if has_entity {
            // Get the Entity example value from type knowledge
            let entity_example = TypeKnowledge::get_entity_example_value()?;

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
        mutation_paths: &HashMap<String, MutationPathExternal>,
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
            mutation_path_builder::extract_spawn_format(mutation_paths)
        } else {
            None
        }
    }

    /// Extract schema information from registry schema
    fn extract_schema_info(registry_schema: &Value) -> SchemaInfo {
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

        SchemaInfo {
            type_kind,
            properties,
            required,
            module_path,
            crate_name,
            reflect_traits,
        }
    }
}
