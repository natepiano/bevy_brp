//! Common types for BRP tools

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::constants::{
    BRP_ERROR_ACCESS_ERROR, BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE, JSON_RPC_ERROR_INTERNAL_ERROR,
    JSON_RPC_ERROR_INVALID_PARAMS,
};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, ParameterName};

/// Configuration trait for BRP tools to control enhanced error handling
pub trait BrpToolConfig {
    /// Whether this tool should use enhanced error handling with `type_guide` embedding
    const ADD_TYPE_GUIDE_TO_ERROR: bool = false;
}

/// Extension trait for `ResultStruct` types that handle BRP responses
pub trait ResultStructBrpExt: Sized {
    type Args;

    /// Construct from BRP client response
    fn from_brp_client_response(args: Self::Args) -> Result<Self>;
}

/// Error information from BRP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrpClientError {
    pub code:    i32,
    pub message: String,
    pub data:    Option<Value>,
}

impl BrpClientError {
    /// Get the error code
    pub const fn get_code(&self) -> i32 {
        self.code
    }

    /// Get the error message
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// Check if this error indicates a format issue that can be recovered
    /// This function was constructed through trial and error via vibe coding with claude
    /// There is a bug in `bevy_remote` right now that we get a spurious "Unknown component type"
    /// when a Component doesn't have Serialize/Deserialize traits - this doesn't affect
    /// Resources so the first section is probably correct.
    /// the second section I think is less correct but it will take some time to validate that
    /// moving to an "error codes only" approach doesn't have other issues
    pub const fn has_format_error_code(&self) -> bool {
        // Common format error codes that indicate type issues
        matches!(
            self.code,
            JSON_RPC_ERROR_INVALID_PARAMS
                | JSON_RPC_ERROR_INTERNAL_ERROR
                | BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE
                | BRP_ERROR_ACCESS_ERROR
        )
    }
}

impl std::fmt::Display for BrpClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Raw BRP JSON-RPC response structure
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct BrpClientCallJsonResponse {
    pub jsonrpc: String,
    pub id:      u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<JsonRpcError>,
}

/// Raw BRP error structure from JSON-RPC response
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JsonRpcError {
    pub code:    i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:    Option<Value>,
}

/// Status of a BRP operation - determines `status` field in the `ToolCallJsonResponse`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Successful operation with optional data
    Success(Option<Value>),
    /// Error with code, message and optional data
    Error(BrpClientError),
}

/// Status of format correction attempts
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatCorrectionStatus {
    /// Format discovery was not enabled for this request
    NotApplicable,
    /// No format correction was attempted
    NotAttempted,
    /// Format correction was applied and the operation succeeded
    Succeeded,
}

/// Type of BRP operation being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Operations that create or replace entire components/resources
    /// Includes: `BevySpawn`, `BevyInsert`, `BevyInsertResource`
    /// Serializes as: `spawn_insert`
    SpawnInsert {
        /// Which parameter name to use when building requests
        /// Components for `BevySpawn`/`BevyInsert`, Value for `BevyInsertResource`
        #[serde(skip)]
        parameter_name: ParameterName,
    },
    /// Operations that modify specific fields
    /// Includes: `BevyMutateComponent`, `BevyMutateResource`
    /// Serializes as: `mutate`
    Mutate {
        /// Which parameter name to use when building requests
        /// Component for `BevyMutateComponent`, Resource for `BevyMutateResource`
        #[serde(skip)]
        parameter_name: ParameterName,
    },
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use consistent string representation for both variants
        let s = match self {
            Self::SpawnInsert { .. } => "spawn_insert",
            Self::Mutate { .. } => "mutate",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<BrpMethod> for Operation {
    type Error = Error;

    fn try_from(method: BrpMethod) -> std::result::Result<Self, Self::Error> {
        match method {
            BrpMethod::WorldSpawnEntity | BrpMethod::WorldInsertComponents => {
                Ok(Self::SpawnInsert {
                    parameter_name: ParameterName::Components,
                })
            }

            BrpMethod::WorldInsertResources => Ok(Self::SpawnInsert {
                parameter_name: ParameterName::Value,
            }),

            BrpMethod::WorldMutateComponents => Ok(Self::Mutate {
                parameter_name: ParameterName::Component,
            }),

            BrpMethod::WorldMutateResources => Ok(Self::Mutate {
                parameter_name: ParameterName::Resource,
            }),

            _ => Err(Error::InvalidArgument(format!(
                "Method {method:?} is not supported for format discovery"
            ))),
        }
    }
}

impl Operation {
    /// Extract type names from parameters based on the operation type
    pub fn extract_type_names(self, params: &Value) -> Vec<String> {
        match self {
            Self::SpawnInsert { parameter_name } => match parameter_name {
                ParameterName::Components => {
                    // Extract from params.components object keys
                    extract_from_components_object(params)
                }
                ParameterName::Value => {
                    // Extract from params.resource field
                    extract_from_resource_field(params)
                }
                _ => Vec::new(),
            },
            Self::Mutate { parameter_name } => match parameter_name {
                ParameterName::Component => {
                    // Extract from params.component string field
                    extract_single_component_type(params)
                }
                ParameterName::Resource => {
                    // Extract from params.resource string field
                    extract_single_resource_type(params)
                }
                _ => Vec::new(),
            },
        }
    }
}

/// Extract type names from components object keys in spawn/insert operations
fn extract_from_components_object(params: &Value) -> Vec<String> {
    params
        .get("components")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Extract type name from resource field in resource operations
fn extract_from_resource_field(params: &Value) -> Vec<String> {
    params
        .get("resource")
        .and_then(|v| v.as_str())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}

/// Extract single component type from component field in mutation operations
fn extract_single_component_type(params: &Value) -> Vec<String> {
    params
        .get("component")
        .and_then(|v| v.as_str())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}

/// Extract single resource type from resource field in mutation operations
fn extract_single_resource_type(params: &Value) -> Vec<String> {
    params
        .get("resource")
        .and_then(|v| v.as_str())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_extract_from_components_object() {
        // Test with valid components object
        let params = json!({
            "components": {
                "bevy_transform::components::transform::Transform": {
                    "translation": [1.0, 2.0, 3.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0]
                },
                "bevy_sprite::sprite::Sprite": {
                    "color": [1.0, 1.0, 1.0, 1.0]
                }
            }
        });

        let types = extract_from_components_object(&params);
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"bevy_transform::components::transform::Transform".to_string()));
        assert!(types.contains(&"bevy_sprite::sprite::Sprite".to_string()));
    }

    #[test]
    fn test_extract_from_components_object_empty() {
        // Test with missing components field
        let params = json!({"entity": 123});
        let types = extract_from_components_object(&params);
        assert!(types.is_empty());

        // Test with null components field
        let params = json!({"components": null});
        let types = extract_from_components_object(&params);
        assert!(types.is_empty());

        // Test with empty components object
        let params = json!({"components": {}});
        let types = extract_from_components_object(&params);
        assert!(types.is_empty());
    }

    #[test]
    fn test_extract_from_resource_field() {
        // Test with valid resource field
        let params = json!({
            "resource": "bevy_time::time::Time",
            "value": {"elapsed_secs": 123.45}
        });

        let types = extract_from_resource_field(&params);
        assert_eq!(types, vec!["bevy_time::time::Time"]);
    }

    #[test]
    fn test_extract_from_resource_field_empty() {
        // Test with missing resource field
        let params = json!({"value": {}});
        let types = extract_from_resource_field(&params);
        assert!(types.is_empty());

        // Test with null resource field
        let params = json!({"resource": null});
        let types = extract_from_resource_field(&params);
        assert!(types.is_empty());
    }

    #[test]
    fn test_extract_single_component_type() {
        // Test with valid component field
        let params = json!({
            "entity": 123,
            "component": "bevy_transform::components::transform::Transform",
            "path": "translation.x",
            "value": 10.0
        });

        let types = extract_single_component_type(&params);
        assert_eq!(
            types,
            vec!["bevy_transform::components::transform::Transform"]
        );
    }

    #[test]
    fn test_extract_single_component_type_empty() {
        // Test with missing component field
        let params = json!({"entity": 123, "path": "translation.x"});
        let types = extract_single_component_type(&params);
        assert!(types.is_empty());
    }

    #[test]
    fn test_extract_single_resource_type() {
        // Test with valid resource field
        let params = json!({
            "resource": "my_game::config::GameConfig",
            "path": "settings.volume",
            "value": 0.8
        });

        let types = extract_single_resource_type(&params);
        assert_eq!(types, vec!["my_game::config::GameConfig"]);
    }

    #[test]
    fn test_extract_single_resource_type_empty() {
        // Test with missing resource field
        let params = json!({"path": "settings.volume", "value": 0.8});
        let types = extract_single_resource_type(&params);
        assert!(types.is_empty());
    }

    #[test]
    fn test_operation_extract_type_names_spawn_insert() {
        use crate::tool::ParameterName;

        // Test spawn/insert operation with components
        let operation = Operation::SpawnInsert {
            parameter_name: ParameterName::Components,
        };

        let params = json!({
            "components": {
                "bevy_transform::components::transform::Transform": {},
                "bevy_sprite::sprite::Sprite": {}
            }
        });

        let types = operation.extract_type_names(&params);
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"bevy_transform::components::transform::Transform".to_string()));
        assert!(types.contains(&"bevy_sprite::sprite::Sprite".to_string()));
    }

    #[test]
    fn test_operation_extract_type_names_insert_resource() {
        use crate::tool::ParameterName;

        // Test insert resource operation
        let operation = Operation::SpawnInsert {
            parameter_name: ParameterName::Value,
        };

        let params = json!({
            "resource": "bevy_time::time::Time",
            "value": {"elapsed_secs": 123.45}
        });

        let types = operation.extract_type_names(&params);
        assert_eq!(types, vec!["bevy_time::time::Time"]);
    }

    #[test]
    fn test_operation_extract_type_names_mutate_component() {
        use crate::tool::ParameterName;

        // Test mutate component operation
        let operation = Operation::Mutate {
            parameter_name: ParameterName::Component,
        };

        let params = json!({
            "entity": 123,
            "component": "bevy_transform::components::transform::Transform",
            "path": "translation.x",
            "value": 10.0
        });

        let types = operation.extract_type_names(&params);
        assert_eq!(
            types,
            vec!["bevy_transform::components::transform::Transform"]
        );
    }

    #[test]
    fn test_operation_extract_type_names_mutate_resource() {
        use crate::tool::ParameterName;

        // Test mutate resource operation
        let operation = Operation::Mutate {
            parameter_name: ParameterName::Resource,
        };

        let params = json!({
            "resource": "my_game::config::GameConfig",
            "path": "settings.volume",
            "value": 0.8
        });

        let types = operation.extract_type_names(&params);
        assert_eq!(types, vec!["my_game::config::GameConfig"]);
    }

    #[test]
    fn test_brp_client_error_display() {
        let error = BrpClientError {
            code:    -32602,
            message: "Invalid params".to_string(),
            data:    None,
        };
        assert_eq!(error.to_string(), "Invalid params");
    }

    #[test]
    fn test_brp_client_error_is_format_error() {
        let format_error = BrpClientError {
            code:    JSON_RPC_ERROR_INVALID_PARAMS,
            message: "Invalid params".to_string(),
            data:    None,
        };
        assert!(format_error.has_format_error_code());

        let unknown_component_error = BrpClientError {
            code:    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
            message: "Unknown component type".to_string(),
            data:    None,
        };
        assert!(unknown_component_error.has_format_error_code());

        let non_format_error = BrpClientError {
            code:    -32601, // Method not found
            message: "Method not found".to_string(),
            data:    None,
        };
        assert!(!non_format_error.has_format_error_code());
    }
}
