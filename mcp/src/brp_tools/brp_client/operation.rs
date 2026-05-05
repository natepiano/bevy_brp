//! BRP operation classification and type-name extraction.

use std::fmt::Display;
use std::fmt::Formatter;

use serde_json::Value;

use crate::error::Error;
use crate::tool::BrpMethod;
use crate::tool::ParameterName;

/// Type of BRP operation being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Operation {
    /// Operations that create or replace entire components/resources
    /// Includes: `BevySpawn`, `BevyInsert`, `BevyInsertResource`
    /// Serializes as: `spawn_insert`
    SpawnInsert {
        /// Which parameter name to use when building requests
        /// Components for `BevySpawn`/`BevyInsert`, `Value` for `BevyInsertResource`
        #[serde(skip)]
        parameter_name: ParameterName,
    },
    /// Operations that modify specific fields
    /// Includes: `BevyMutateComponent`, `BevyMutateResource`
    /// Serializes as: `mutate`
    Mutate {
        /// Which parameter name to use when building requests
        /// `Component` for `BevyMutateComponent`, `Resource` for `BevyMutateResource`
        #[serde(skip)]
        parameter_name: ParameterName,
    },
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
            },

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
    pub(super) fn extract_type_names(self, params: &Value) -> Vec<String> {
        match self {
            Self::SpawnInsert { parameter_name } => match parameter_name {
                ParameterName::Components => {
                    // Extract from params.components object keys
                    extract_from_components_object(params)
                },
                ParameterName::Value => {
                    // Extract from params.resource field
                    extract_from_resource_field(params)
                },
                _ => Vec::new(),
            },
            Self::Mutate { parameter_name } => match parameter_name {
                ParameterName::Component => {
                    // Extract from params.component string field
                    extract_single_component_type(params)
                },
                ParameterName::Resource => {
                    // Extract from params.resource string field
                    extract_single_resource_type(params)
                },
                _ => Vec::new(),
            },
        }
    }
}

/// Extract type names from components object keys in spawn/insert operations
fn extract_from_components_object(params: &Value) -> Vec<String> {
    params
        .get("components")
        .and_then(Value::as_object)
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Extract type name from resource field in resource operations
fn extract_from_resource_field(params: &Value) -> Vec<String> {
    params
        .get("resource")
        .and_then(Value::as_str)
        .map(|value| vec![String::from(value)])
        .unwrap_or_default()
}

/// Extract single component type from component field in mutation operations
fn extract_single_component_type(params: &Value) -> Vec<String> {
    params
        .get("component")
        .and_then(Value::as_str)
        .map(|value| vec![String::from(value)])
        .unwrap_or_default()
}

/// Extract single resource type from resource field in mutation operations
fn extract_single_resource_type(params: &Value) -> Vec<String> {
    params
        .get("resource")
        .and_then(Value::as_str)
        .map(|value| vec![String::from(value)])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::tool::ParameterName;

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
}
