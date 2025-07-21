//! Schema utilities for converting JsonSchema types to Parameter definitions

use std::collections::HashSet;
use std::str::FromStr;

use schemars::JsonSchema;
use schemars::schema::{InstanceType, Schema, SingleOrVec};

use crate::field_extraction::{Parameter, ParameterFieldType, ParameterName};

/// Convert a JsonSchema-enabled struct to Parameter definitions
pub fn schema_to_parameters<T: JsonSchema>() -> Vec<Parameter> {
    let schema = schemars::schema_for!(T);

    let object = match &schema.schema.object {
        Some(obj) => obj,
        None => return vec![],
    };

    let required_fields: HashSet<String> = object.required.iter().cloned().collect();

    object
        .properties
        .iter()
        .map(|(field_name, field_schema)| {
            // Convert snake_case field name to ParameterName enum
            let param_name = ParameterName::from_str(field_name)
                .unwrap_or_else(|_| panic!("Unknown parameter name: {field_name}"));

            // Check if field is required (not wrapped in Option<T>)
            let required = required_fields.contains(field_name);

            // Map JSON schema types to ParameterFieldType
            let param_type = map_schema_type_to_parameter_type(field_schema);

            // Extract description from schema (doc comments become descriptions)
            // Note: For now we'll use a static string. In a real implementation,
            // we'd need to handle the lifetime issue or change Parameter to own the string.
            let description = "Parameter description from schema";

            // Use the appropriate Parameter constructor based on the type
            match param_type {
                ParameterFieldType::String => Parameter::string(param_name, description, required),
                ParameterFieldType::Number => Parameter::number(param_name, description, required),
                ParameterFieldType::Boolean => {
                    Parameter::boolean(param_name, description, required)
                }
                ParameterFieldType::StringArray => {
                    Parameter::string_array(param_name, description, required)
                }
                ParameterFieldType::NumberArray => {
                    Parameter::number_array(param_name, description, required)
                }
                ParameterFieldType::Any => Parameter::any(param_name, description, required),
                ParameterFieldType::DynamicParams => {
                    Parameter::any(param_name, description, required)
                }
            }
        })
        .collect()
}

fn map_schema_type_to_parameter_type(schema: &Schema) -> ParameterFieldType {
    match schema {
        Schema::Object(obj) => {
            match &obj.instance_type {
                Some(SingleOrVec::Single(instance_type)) => match **instance_type {
                    InstanceType::String => ParameterFieldType::String,
                    InstanceType::Integer | InstanceType::Number => ParameterFieldType::Number,
                    InstanceType::Boolean => ParameterFieldType::Boolean,
                    InstanceType::Array => {
                        // Determine array element type from items schema
                        if let Some(array) = &obj.array {
                            match &array.items {
                                Some(SingleOrVec::Single(item_schema)) => {
                                    if is_string_schema(item_schema) {
                                        ParameterFieldType::StringArray
                                    } else if is_number_schema(item_schema) {
                                        ParameterFieldType::NumberArray
                                    } else {
                                        ParameterFieldType::Any
                                    }
                                }
                                _ => ParameterFieldType::Any,
                            }
                        } else {
                            ParameterFieldType::Any
                        }
                    }
                    _ => ParameterFieldType::Any,
                },
                _ => ParameterFieldType::Any,
            }
        }
        _ => ParameterFieldType::Any,
    }
}

fn is_string_schema(schema: &Schema) -> bool {
    if let Schema::Object(obj) = schema {
        matches!(
            &obj.instance_type,
            Some(SingleOrVec::Single(instance_type)) if matches!(**instance_type, InstanceType::String)
        )
    } else {
        false
    }
}

fn is_number_schema(schema: &Schema) -> bool {
    if let Schema::Object(obj) = schema {
        matches!(
            &obj.instance_type,
            Some(SingleOrVec::Single(instance_type)) if matches!(**instance_type, InstanceType::Integer | InstanceType::Number)
        )
    } else {
        false
    }
}
