//! Type registry access utilities for format discovery
//!
//! This module provides centralized functions for accessing Bevy's type registry
//! and retrieving type information needed for format discovery operations.

use bevy::prelude::*;
use bevy::reflect::TypeInfo;
use serde_json::{Value, json};

/// Errors that can occur during registry operations
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Type '{type_name}' not found in registry")]
    TypeNotFound { type_name: String },
}

impl RegistryError {
    /// Convert to a JSON error map for BRP responses
    pub fn to_json_error(&self) -> serde_json::Map<String, Value> {
        let mut error_info = serde_json::Map::new();
        match self {
            Self::TypeNotFound { type_name } => {
                error_info.insert("reason".to_string(), json!("Type not found in registry"));
                error_info.insert(
                    "details".to_string(),
                    json!(format!(
                        "Type '{type_name}' is not registered with Bevy's type registry"
                    )),
                );
            }
        }
        error_info
    }
}

/// Get type info from the type registry with unified error handling
pub fn get_type_info_from_registry(
    world: &World,
    type_name: &str,
    debug_info: &mut Vec<String>,
) -> Result<TypeInfo, RegistryError> {
    debug_info.push(format!("Getting type info for: {type_name}"));
    let type_registry = world.resource::<AppTypeRegistry>();

    // Get type info within a smaller scope to release the registry lock early
    let registry = type_registry.read();
    if let Some(registration) = registry.get_with_type_path(type_name) {
        debug_info.push(format!("Found type in registry: {type_name}"));
        Ok(registration.type_info().clone())
    } else {
        debug_info.push(format!("Type not found in registry: {type_name}"));
        Err(RegistryError::TypeNotFound {
            type_name: type_name.to_string(),
        })
    }
}
