//! Parameter definitions for our MCP tools
use std::collections::HashSet;

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use strum::{Display, EnumString};

use crate::constants::VALID_PORT_RANGE;
use crate::tool::mcp_tool_schema::ParameterBuilder;

/// Deserialize and validate port numbers
///
/// This function ensures that all port parameters are within the valid range (1024-65534).
/// It's used as a serde `deserialize_with` attribute on port fields.
pub fn deserialize_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let port = u16::deserialize(deserializer)?;

    if VALID_PORT_RANGE.contains(&port) {
        Ok(port)
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid port {}: must be in range {}-{}",
            port,
            VALID_PORT_RANGE.start(),
            VALID_PORT_RANGE.end()
        )))
    }
}

/// Unified parameter names combining all BRP and local tool parameters
/// Entries are alphabetically sorted for easy maintenance
/// serialized into parameter names provided to the rcmp mcp tool framework
#[derive(Display, EnumString, Clone, Copy, Debug, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ParameterName {
    /// Application name
    AppName,
    /// Component type for mutations
    Component,
    /// Components parameter for operations
    Components,
    /// Data parameter for queries
    Data,
    /// Duration in milliseconds
    DurationMs,
    /// Boolean enabled flag
    Enabled,
    /// Multiple entities for batch operations
    Entities,
    /// Entity ID parameter
    Entity,
    /// Example name
    ExampleName,
    /// Log filename
    Filename,
    /// Filter parameter for queries
    Filter,
    /// Keys array for input simulation
    Keys,
    /// Keyword for filtering
    Keyword,
    /// Tracing level
    Level,
    /// Method name for dynamic execution
    Method,
    /// Age threshold in seconds
    OlderThanSeconds,
    /// Parameters for dynamic method execution
    Params,
    /// Parent entity for reparenting
    Parent,
    /// Path for field mutations or file paths
    Path,
    /// Build profile (debug/release)
    Profile,
    /// Resource type name parameter
    Resource,
    /// Strict mode flag for queries
    Strict,
    /// Number of lines to tail
    TailLines,
    /// Types parameter for discovery
    Types,
    /// Value for mutations and inserts
    Value,
    /// Verbose output flag
    Verbose,
    /// Watch ID for stopping watches
    WatchId,
    /// Include specific crates in schema
    WithCrates,
    /// Exclude specific crates from schema
    WithoutCrates,
    /// Include specific reflect types
    WithTypes,
    /// Exclude specific reflect types
    WithoutTypes,
}

/// Parameter field types for schema generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParameterType {
    /// A string field
    String,
    /// A numeric field (typically u64)
    Number,
    /// A boolean field
    Boolean,
    /// An array of strings
    StringArray,
    /// An array of numbers
    NumberArray,
    /// Any JSON value (object, array, etc.)
    Any,
}

fn map_schema_type_to_parameter_type(schema: &Schema) -> ParameterType {
    let Some(obj) = schema.as_object() else {
        return ParameterType::Any;
    };

    // Get the "type" field
    let Some(type_value) = obj.get("type") else {
        return ParameterType::Any;
    };

    match type_value {
        Value::String(type_str) => match type_str.as_str() {
            "string" => ParameterType::String,
            "integer" | "number" => ParameterType::Number,
            "boolean" => ParameterType::Boolean,
            "array" => {
                // Check items schema for array element type
                obj.get("items")
                    .and_then(|items| items.as_object())
                    .and_then(|items_obj| items_obj.get("type"))
                    .and_then(|item_type| item_type.as_str())
                    .map_or(ParameterType::Any, |item_type_str| match item_type_str {
                        "string" => ParameterType::StringArray,
                        "integer" | "number" => ParameterType::NumberArray,
                        _ => ParameterType::Any,
                    })
            }
            _ => ParameterType::Any,
        },
        Value::Array(types) => {
            // Handle Option<T> types which generate ["T", "null"] schemas
            let non_null_types: Vec<&str> = types
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|&t| t != "null")
                .collect();

            if non_null_types.len() == 1 {
                match non_null_types.first() {
                    Some(&"string") => ParameterType::String,
                    Some(&"integer" | &"number") => ParameterType::Number,
                    Some(&"boolean") => ParameterType::Boolean,
                    _ => ParameterType::Any,
                }
            } else {
                ParameterType::Any
            }
        }
        _ => ParameterType::Any,
    }
}

/// Build parameters from a `JsonSchema` type directly into a `ParameterBuilder`
/// All tools with parameters derive `JsonSchema` making it possible for us
/// to build the parameters from the schema
pub fn extract_parameters<T: JsonSchema>() -> ParameterBuilder {
    let schema = schemars::schema_for!(T);
    let mut builder = ParameterBuilder::new();

    let Some(root_obj) = schema.as_object() else {
        return builder;
    };

    let Some(properties) = root_obj.get("properties").and_then(|p| p.as_object()) else {
        return builder;
    };

    let required_fields: HashSet<String> = root_obj
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    for (field_name, field_value) in properties {
        let required = required_fields.contains(field_name);

        // Convert the JSON value to a Schema for processing
        let field_schema = if let Value::Object(obj) = field_value {
            Schema::from(obj.clone())
        } else if let Value::Bool(b) = field_value {
            Schema::from(*b)
        } else {
            continue; // Skip non-schema values
        };
        let param_type = map_schema_type_to_parameter_type(&field_schema);

        // Extract description from schema if available
        let description = field_value
            .as_object()
            .and_then(|obj| obj.get("description"))
            .and_then(|d| d.as_str())
            .unwrap_or(field_name.as_str());

        // Add to builder based on type
        builder = match param_type {
            ParameterType::String => builder.add_string_property(field_name, description, required),
            ParameterType::Number => builder.add_number_property(field_name, description, required),
            ParameterType::Boolean => {
                builder.add_boolean_property(field_name, description, required)
            }
            ParameterType::StringArray => {
                builder.add_string_array_property(field_name, description, required)
            }
            ParameterType::NumberArray => {
                builder.add_number_array_property(field_name, description, required)
            }
            ParameterType::Any => builder.add_any_property(field_name, description, required),
        };
    }

    builder
}
