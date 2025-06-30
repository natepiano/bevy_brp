//! Shared support functionality for format discovery
//!
//! This module contains common functionality used across format discovery phases
//! to avoid code duplication.

use serde_json::{Map, Value};

use super::engine::ParameterLocation;

/// Get the parameter location for a given method
pub fn get_parameter_location(method: &str) -> ParameterLocation {
    match method {
        crate::tools::BRP_METHOD_MUTATE_COMPONENT => ParameterLocation::ComponentValue,
        crate::tools::BRP_METHOD_INSERT_RESOURCE | crate::tools::BRP_METHOD_MUTATE_RESOURCE => {
            ParameterLocation::ResourceValue
        }
        _ => ParameterLocation::Components,
    }
}

/// Extract type items from parameters based on location
pub fn extract_type_items(params: &Value, location: ParameterLocation) -> Vec<(String, Value)> {
    match location {
        ParameterLocation::Components => {
            // For spawn/insert methods
            params
                .get("components")
                .and_then(|c| c.as_object())
                .map(|components| {
                    components
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                })
                .unwrap_or_default()
        }
        ParameterLocation::ComponentValue => {
            // For mutate_component method
            if let (Some(component), Some(value)) = (
                params.get("component").and_then(|c| c.as_str()),
                params.get("value"),
            ) {
                vec![(component.to_string(), value.clone())]
            } else {
                Vec::new()
            }
        }
        ParameterLocation::ResourceValue => {
            // For insert_resource/mutate_resource methods
            if let (Some(resource), Some(value)) = (
                params.get("resource").and_then(|r| r.as_str()),
                params.get("value"),
            ) {
                vec![(resource.to_string(), value.clone())]
            } else {
                Vec::new()
            }
        }
    }
}

/// Apply type corrections to parameters based on location
pub fn apply_corrections(
    params: &Value,
    location: ParameterLocation,
    corrected_items: &[(String, Value)],
) -> Value {
    let mut corrected_params = params.clone();

    match location {
        ParameterLocation::Components => {
            // For spawn/insert methods - update the components object
            if let Some(params_obj) = corrected_params.as_object_mut() {
                let mut corrected_components = Map::new();
                for (type_name, type_value) in corrected_items {
                    corrected_components.insert(type_name.clone(), type_value.clone());
                }
                params_obj.insert(
                    "components".to_string(),
                    Value::Object(corrected_components),
                );
            }
        }
        ParameterLocation::ComponentValue => {
            // For mutate_component method - update the value field
            if let Some(params_obj) = corrected_params.as_object_mut() {
                if let Some((_, corrected_value)) = corrected_items.first() {
                    params_obj.insert("value".to_string(), corrected_value.clone());
                }
            }
        }
        ParameterLocation::ResourceValue => {
            // For insert_resource/mutate_resource methods - update the value field
            if let Some(params_obj) = corrected_params.as_object_mut() {
                if let Some((_, corrected_value)) = corrected_items.first() {
                    params_obj.insert("value".to_string(), corrected_value.clone());
                }
            }
        }
    }

    corrected_params
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use serde_json::json;

    use super::*;

    #[test]
    fn test_get_parameter_location() {
        assert!(matches!(
            get_parameter_location("bevy/mutate_component"),
            ParameterLocation::ComponentValue
        ));
        assert!(matches!(
            get_parameter_location("bevy/insert_resource"),
            ParameterLocation::ResourceValue
        ));
        assert!(matches!(
            get_parameter_location("bevy/spawn"),
            ParameterLocation::Components
        ));
    }

    #[test]
    fn test_extract_type_items_components() {
        let params = json!({
            "components": {
                "bevy_transform::Transform": {"x": 0, "y": 0, "z": 0},
                "bevy_sprite::Sprite": {"color": [1.0, 0.0, 0.0, 1.0]}
            }
        });

        let items = extract_type_items(&params, ParameterLocation::Components);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_extract_type_items_component_value() {
        let params = json!({
            "component": "bevy_transform::Transform",
            "value": {"x": 0, "y": 0, "z": 0}
        });

        let items = extract_type_items(&params, ParameterLocation::ComponentValue);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "bevy_transform::Transform");
    }

    #[test]
    fn test_apply_corrections_components() {
        let params = json!({
            "components": {
                "OldComponent": {"field": "value"}
            }
        });

        let corrected_items = vec![
            ("Component1".to_string(), json!({"x": 1})),
            ("Component2".to_string(), json!({"y": 2})),
        ];

        let result = apply_corrections(&params, ParameterLocation::Components, &corrected_items);
        assert!(
            result.get("components").is_some(),
            "Expected components field"
        );
        assert!(
            result["components"].is_object(),
            "Expected components to be an object"
        );
        let components = result["components"].as_object();
        assert!(
            components.is_some(),
            "Expected components to be an object after validation"
        );
        let components = components.unwrap(); // Safe after assertion
        assert_eq!(components.len(), 2);
        assert!(components.contains_key("Component1"));
        assert!(components.contains_key("Component2"));
    }
}
