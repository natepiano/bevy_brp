//! Core implementation of `DiscoveryContext`
//!
//! This module contains the main logic for the `DiscoveryContext` struct.

use std::collections::HashMap;

use serde_json::Value;
use tracing::debug;

use super::type_context::TypeContext;
use crate::brp_tools::Port;
use crate::brp_tools::brp_type_schema::{BrpTypeName, TypeSchemaEngine};
use crate::error::{Error, Result};
use crate::tool::BrpMethod;

pub struct DiscoveryContext {
    /// Type information from Bevy's registry
    type_map: HashMap<BrpTypeName, TypeContext>,
}

impl DiscoveryContext {
    /// Create a new `DiscoveryContext` from BRP method parameters
    /// Uses `TypeSchemaEngine` as single source of truth for type information
    pub async fn new(method: BrpMethod, port: Port, params: &Value) -> Result<Self> {
        // Extract type names, values, and mutation paths
        let tool_arguments = Self::extract_type_name_and_original_value(method, params)?;

        debug!("using TypeSchemaEngine for type information (single registry fetch)");

        // Get TypeInfo from TypeSchemaEngine (single registry fetch)
        let engine = TypeSchemaEngine::new(port).await?;
        let type_names: Vec<String> = tool_arguments
            .iter()
            .map(|(name, _, _)| name.as_str().to_string())
            .collect();
        let response = engine.generate_response(&type_names);

        // Build type_map from TypeInfo using the new constructor
        let mut type_map = HashMap::new();

        for (type_name, original_value, mutation_path) in tool_arguments {
            let type_info = response.type_info.get(&type_name).ok_or_else(|| {
                Error::InvalidArgument(format!(
                    "Type '{type_name}' not found in registry. Verify the type name is correct and the Bevy app is running with this component registered."
                ))
            })?;

            // Create TypeContext from TypeInfo with mutation path
            let unified_info =
                TypeContext::from_type_info(type_info.clone(), original_value, mutation_path);
            type_map.insert(type_name, unified_info);
        }

        Ok(Self { type_map })
    }

    /// Get all types as an iterator
    pub fn types(&self) -> impl Iterator<Item = &TypeContext> {
        self.type_map.values()
    }

    /// Extract type names and the original parameter value from method parameters
    fn extract_type_name_and_original_value(
        method: BrpMethod,
        params: &Value,
    ) -> Result<Vec<(BrpTypeName, Value, Option<String>)>> {
        let mut tool_arguments = Vec::new();

        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
                // Validate components field exists and is an object
                let components = params
                    .get("components")
                    .ok_or_else(|| {
                        Error::InvalidArgument("Missing 'components' field".to_string())
                    })?
                    .as_object()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'components' field must be an object".to_string())
                    })?;

                for (type_name, value) in components {
                    // Validate type name is a valid string (could add more validation here)
                    if type_name.is_empty() {
                        return Err(Error::InvalidArgument(
                            "Empty type name in components".to_string(),
                        )
                        .into());
                    }
                    tool_arguments.push((type_name.into(), value.clone(), None));
                }

                if tool_arguments.is_empty() {
                    return Err(Error::InvalidArgument("No components provided".to_string()).into());
                }
            }
            BrpMethod::BevyMutateComponent => {
                let component = params
                    .get("component")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'component' field".to_string()))?
                    .as_str()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'component' field must be a string".to_string())
                    })?;

                if component.is_empty() {
                    return Err(
                        Error::InvalidArgument("Empty component type name".to_string()).into(),
                    );
                }

                let path = params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let value = params
                    .get("value")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'value' field".to_string()))?
                    .clone();
                tool_arguments.push((component.into(), value, path));
            }
            BrpMethod::BevyInsertResource | BrpMethod::BevyMutateResource => {
                let resource = params
                    .get("resource")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'resource' field".to_string()))?
                    .as_str()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'resource' field must be a string".to_string())
                    })?;

                if resource.is_empty() {
                    return Err(
                        Error::InvalidArgument("Empty resource type name".to_string()).into(),
                    );
                }

                let path = if method == BrpMethod::BevyMutateResource {
                    params
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                } else {
                    None
                };

                let value = params
                    .get("value")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'value' field".to_string()))?
                    .clone();
                tool_arguments.push((resource.into(), value, path));
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Method {method:?} does not support type extraction"
                ))
                .into());
            }
        }

        Ok(tool_arguments)
    }
}
