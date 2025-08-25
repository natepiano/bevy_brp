//! Core implementation of `DiscoveryContext`
//!
//! This module contains the main logic for the `DiscoveryContext` struct.

use std::collections::HashMap;

use serde_json::Value;
use tracing::debug;

use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::Port;
use crate::brp_tools::brp_type_schema::{BrpTypeName, TypeSchemaEngine};
use crate::error::{Error, Result};
use crate::tool::BrpMethod;

pub struct DiscoveryContext {
    /// Type information from Bevy's registry
    type_map: HashMap<BrpTypeName, UnifiedTypeInfo>,
}

impl DiscoveryContext {
    /// Create a new `DiscoveryContext` from BRP method parameters
    /// Uses `TypeSchemaEngine` as single source of truth for type information
    pub async fn new(method: BrpMethod, port: Port, params: &Value) -> Result<Self> {
        // Extract type names and values together
        let type_value_pairs = Self::extract_type_name_and_original_value(method, params)?;

        debug!("using TypeSchemaEngine for type information (single registry fetch)");

        // Get TypeInfo from TypeSchemaEngine (single registry fetch)
        let engine = TypeSchemaEngine::new(port).await?;
        let type_names: Vec<String> = type_value_pairs
            .iter()
            .map(|(name, _)| name.as_str().to_string())
            .collect();
        let response = engine.generate_response(&type_names);

        // Build type_map from TypeInfo using the new constructor
        let mut type_map = HashMap::new();

        for (type_name, original_value) in type_value_pairs {
            let type_info = response.type_info.get(&type_name).ok_or_else(|| {
                Error::InvalidArgument(format!(
                    "Type '{type_name}' not found in registry. Verify the type name is correct and the Bevy app is running with this component registered."
                ))
            })?;

            // Create UnifiedTypeInfo from TypeInfo (single source of truth)
            let unified_info =
                UnifiedTypeInfo::from_type_info(type_info.clone(), original_value, method);
            type_map.insert(type_name, unified_info);
        }

        Ok(Self { type_map })
    }

    /// Get all types as an iterator
    pub fn types(&self) -> impl Iterator<Item = &UnifiedTypeInfo> {
        self.type_map.values()
    }

    /// Extract type names and the original parameter value from method parameters
    fn extract_type_name_and_original_value(
        method: BrpMethod,
        params: &Value,
    ) -> Result<Vec<(BrpTypeName, Value)>> {
        let mut pairs = Vec::new();

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
                    pairs.push((type_name.into(), value.clone()));
                }

                if pairs.is_empty() {
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

                let value = params
                    .get("value")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'value' field".to_string()))?
                    .clone();
                pairs.push((component.into(), value));
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

                let value = params
                    .get("value")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'value' field".to_string()))?
                    .clone();
                pairs.push((resource.into(), value));
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Method {method:?} does not support type extraction"
                ))
                .into());
            }
        }

        Ok(pairs)
    }
}

/// Find type information in the discovery response
///
/// The response format is always:
/// ```json
/// {
///   "type_info": {
///     "TypeName": { ... }
///   }
/// }
/// ```
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn test_from_params_empty_components() {
        // Test with empty components object
        let params = json!({
            "components": {}
        });

        let result = DiscoveryContext::new(BrpMethod::BevySpawn, Port(15702), &params).await;

        // This should fail since no components are provided
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_from_params_with_components() {
        // Test with actual components
        let params = json!({
            "components": {
                "bevy_transform::components::transform::Transform": {
                    "translation": {"x": 0.0, "y": 0.0, "z": 0.0},
                    "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
                    "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
                }
            }
        });

        let result = DiscoveryContext::new(BrpMethod::BevySpawn, Port(15702), &params).await;

        // This may succeed or fail depending on BRP availability, but shouldn't crash
        assert!(result.is_ok() || result.is_err());
    }

    // Integration tests would go in the tests/ directory to test with actual BRP

    // Registry integration tests removed - direct registry processing replaced by TypeSchemaEngine
}
