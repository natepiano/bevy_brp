//! V2 engine for type schema generation
//!
//! This module provides the new parallel implementation of type schema generation
//! that will eventually replace the original engine. It uses the complete registry
//! approach instead of recursive discovery.

use std::collections::HashMap;

use serde_json::Value;

use super::registry_cache::get_full_registry;
use super::result_types::TypeSchemaResponseV2;
use super::schema_processor::SchemaProcessor;
use super::type_discovery::{determine_supported_operations, extract_reflect_types};
use super::types::BrpTypeName;
use crate::brp_tools::Port;
use crate::error::Result;

/// V2 engine for type schema generation using complete registry approach
pub struct TypeSchemaEngineV2 {
    port:     Port,
    registry: HashMap<BrpTypeName, Value>,
}

impl TypeSchemaEngineV2 {
    /// Create a new V2 engine instance
    pub async fn new(port: Port) -> Result<Self> {
        let registry = get_full_registry(port).await?;
        Ok(Self { port, registry })
    }

    /// Generate response for requested types using the V2 approach
    pub fn generate_response(&self, requested_types: &[String]) -> TypeSchemaResponseV2 {
        let mut spawn_formats = HashMap::new();
        let mut mutation_info = HashMap::new();
        let mut supported_operations = HashMap::new();
        let mut schemas = HashMap::new();

        let mut discovered_count = 0;

        for type_name in requested_types {
            let brp_type_name = BrpTypeName::from(type_name);

            if let Some(type_schema) = self.registry.get(&brp_type_name) {
                // Use SchemaProcessor for this type
                let processor = SchemaProcessor::new(type_schema, type_name, self.port);

                // Build spawn format
                let spawn_format = processor.build_spawn_format();
                spawn_formats.insert(type_name.clone(), Value::Object(spawn_format));

                // Build mutation info
                let mutation_paths = processor.build_mutation_paths();
                mutation_info.insert(type_name.clone(), mutation_paths);

                // Extract reflection traits from the schema
                let reflect_types = extract_reflect_types(type_schema);

                // Determine supported operations based on traits
                let operations = determine_supported_operations(&reflect_types);

                // Convert to strings for the response
                let operations_strings: Vec<String> =
                    operations.iter().map(|op| op.to_string()).collect();

                supported_operations.insert(type_name.clone(), operations_strings);

                // Add schema
                schemas.insert(type_name.clone(), type_schema.clone());

                discovered_count += 1;
            }
        }

        TypeSchemaResponseV2 {
            spawn_format: spawn_formats,
            mutation_info,
            supported_operations,
            reflection_traits: HashMap::new(),
            discovered_count,
            schemas,
        }
    }
}
