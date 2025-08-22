//! V2 engine for type schema generation
//!
//! This module provides the new parallel implementation of type schema generation
//! that will eventually replace the original engine. It uses the complete registry
//! approach instead of recursive discovery.

use std::collections::HashMap;

use serde_json::Value;

use super::registry_cache::get_full_registry;
use super::result_types::{TypeInfo, TypeSchemaResponse, TypeSchemaSummary};
use super::types::BrpTypeName;
use crate::brp_tools::Port;
use crate::error::Result;

/// V2 engine for type schema generation using complete registry approach
pub struct TypeSchemaEngine {
    registry: HashMap<BrpTypeName, Value>,
}

impl TypeSchemaEngine {
    /// Create a new V2 engine instance by fetching the complete registry
    ///
    /// If `refresh_cache` is true, the registry cache will be cleared before fetching,
    /// ensuring fresh type information is retrieved.
    pub async fn new(port: Port, refresh_cache: bool) -> Result<Self> {
        let registry = get_full_registry(port, refresh_cache).await?;
        Ok(Self { registry })
    }

    /// Generate response for requested types using the V2 approach
    pub fn generate_response(&self, requested_types: &[String]) -> TypeSchemaResponse {
        let mut response = TypeSchemaResponse {
            discovered_count: 0,
            requested_types:  requested_types.to_vec(),
            summary:          TypeSchemaSummary {
                failed_discoveries:     0,
                successful_discoveries: 0,
                total_requested:        requested_types.len(),
            },
            type_info:        HashMap::new(),
        };

        for brp_type_name in requested_types.iter().map(BrpTypeName::from) {
            let type_info = if let Some(type_schema) = self.registry.get(&brp_type_name) {
                response.discovered_count += 1;
                response.summary.successful_discoveries += 1;
                self.build_type_info(brp_type_name.clone(), type_schema)
            } else {
                response.summary.failed_discoveries += 1;
                TypeInfo::not_found(brp_type_name.clone())
            };

            response.type_info.insert(brp_type_name, type_info);
        }

        response
    }

    /// Build `TypeInfo` for a single type
    fn build_type_info(&self, brp_type_name: BrpTypeName, type_schema: &Value) -> TypeInfo {
        TypeInfo::from_schema(brp_type_name, type_schema, &self.registry)
    }
}
