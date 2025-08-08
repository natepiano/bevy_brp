//! Parameter names, and tools to automatically create parameter definitions for rmcp from our
//! parameter structs
use std::collections::HashSet;
use std::sync::Arc;

use schemars::{JsonSchema, Schema};
use serde_json::{Map, Value};
use strum::{Display, EnumString};

use crate::string_traits::IntoStrings;

/// Trait for parameter types used in tools
///
/// This trait provides a type-level constraint for tool parameter types.
/// It ensures that only valid parameter types can be used as associated types
/// in the `ToolFn` trait.
///
/// The trait is automatically implemented by the `ParamStruct` derive macro
/// for parameter structs.
pub trait ParamStruct: Send + Sync + serde::Serialize {}

/// Implementation for unit type to support parameterless tools
impl ParamStruct for () {}

/// Unified parameter names combining all BRP and local tool parameters
/// Entries are alphabetically sorted for easy maintenance
/// serialized into parameter names provided to the rcmp mcp tool framework
#[derive(
    Display,
    EnumString,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::AsRefStr,
    strum::IntoStaticStr,
)]
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
    /// Port number for connections
    Port,
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

/// Builder for creating JSON schemas for MCP tool registration in rmcp framework
#[derive(Clone, Default)]
pub struct ParameterBuilder {
    properties: Map<String, Value>,
    required:   Vec<String>,
}

impl ParameterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a string property to the schema
    pub fn add_string_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "string".into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a string array property to the schema
    pub fn add_string_array_property(
        mut self,
        name: &str,
        description: &str,
        required: bool,
    ) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "array".into());

        let mut items = Map::new();
        items.insert("type".to_string(), "string".into());
        prop.insert("items".to_string(), items.into());

        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a number array property to the schema
    pub fn add_number_array_property(
        mut self,
        name: &str,
        description: &str,
        required: bool,
    ) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "array".into());

        let mut items = Map::new();
        items.insert("type".to_string(), "number".into());
        prop.insert("items".to_string(), items.into());

        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a number property to the schema
    pub fn add_number_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "number".into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a boolean property to the schema
    pub fn add_boolean_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "boolean".into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a property that can be any type (object, array, null, etc.)
    pub fn add_any_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), vec!["object", "array", "null"].into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Build the final schema
    pub fn build(self) -> Arc<Map<String, Value>> {
        let mut schema = Map::new();
        schema.insert("type".to_string(), "object".into());
        schema.insert("properties".to_string(), self.properties.into());

        if !self.required.is_empty() {
            schema.insert("required".to_string(), self.required.into());
        }

        Arc::new(schema)
    }
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
pub fn build_parameters_from<T: JsonSchema>() -> ParameterBuilder {
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
                .into_strings()
                .into_iter()
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

impl From<ParameterName> for String {
    fn from(param: ParameterName) -> Self {
        param.as_ref().to_string()
    }
}
