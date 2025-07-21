//! Schema utilities for converting `JsonSchema` types to parameter definitions

use std::collections::HashSet;

use schemars::JsonSchema;
use schemars::schema::{InstanceType, Schema, SingleOrVec};

use crate::field_extraction::ParameterFieldType;
use crate::tool::mcp_tool_schema::ParameterBuilder;

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
                        obj.array.as_ref().map_or(ParameterFieldType::Any, |array| {
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
                        })
                    }
                    _ => ParameterFieldType::Any,
                },
                Some(SingleOrVec::Vec(types)) => {
                    // Handle Option<T> types which generate [T, null] schemas
                    let non_null_types: Vec<_> = types.iter()
                        .filter(|t| !matches!(t, InstanceType::Null))
                        .collect();
                    
                    if non_null_types.len() == 1 {
                        match non_null_types[0] {
                            InstanceType::String => ParameterFieldType::String,
                            InstanceType::Integer | InstanceType::Number => ParameterFieldType::Number,
                            InstanceType::Boolean => ParameterFieldType::Boolean,
                            _ => ParameterFieldType::Any,
                        }
                    } else {
                        ParameterFieldType::Any
                    }
                },
                _ => ParameterFieldType::Any,
            }
        }
        Schema::Bool(_) => ParameterFieldType::Any,
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

/// Build parameters from a `JsonSchema` type directly into a `ParameterBuilder`
pub fn parameters_from_schema<T: JsonSchema>() -> ParameterBuilder {
    let schema = schemars::schema_for!(T);
    let mut builder = ParameterBuilder::new();

    let Some(object) = &schema.schema.object else {
        return builder;
    };

    let required_fields: HashSet<String> = object.required.iter().cloned().collect();

    for (field_name, field_schema) in &object.properties {
        let required = required_fields.contains(field_name);
        let param_type = map_schema_type_to_parameter_type(field_schema);

        // Extract description from schema metadata if available
        let description = if let Schema::Object(obj) = field_schema {
            obj.metadata
                .as_ref()
                .and_then(|m| m.description.as_ref())
                .map_or(field_name.as_str(), std::string::String::as_str)
        } else {
            field_name.as_str()
        };

        // Add to builder based on type
        builder = match param_type {
            ParameterFieldType::String => {
                builder.add_string_property(field_name, description, required)
            }
            ParameterFieldType::Number => {
                builder.add_number_property(field_name, description, required)
            }
            ParameterFieldType::Boolean => {
                builder.add_boolean_property(field_name, description, required)
            }
            ParameterFieldType::StringArray => {
                builder.add_string_array_property(field_name, description, required)
            }
            ParameterFieldType::NumberArray => {
                builder.add_number_array_property(field_name, description, required)
            }
            ParameterFieldType::Any | ParameterFieldType::DynamicParams => {
                builder.add_any_property(field_name, description, required)
            }
        };
    }

    builder
}
