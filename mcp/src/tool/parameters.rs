//! Parameter names, and tools to automatically create parameter definitions for rmcp from our
//! parameter structs
use std::collections::HashSet;
use std::sync::Arc;

use bevy_brp_mcp_macros::ParamStruct;
use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use strum::{Display, EnumString};

use crate::json_object::{IntoStrings, JsonObjectAccess};
use crate::json_schema::{JsonSchemaType, SchemaField};

/// Trait for parameter types used in tools
///
/// This trait provides a type-level constraint for tool parameter types.
/// It ensures that only valid parameter types can be used as associated types
/// in the `ToolFn` trait.
///
/// The trait is automatically implemented by the `ParamStruct` derive macro
/// for parameter structs.
pub trait ParamStruct: Send + Sync + serde::Serialize + serde::de::DeserializeOwned {}

/// Shared parameter struct for tools that have no parameters
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct NoParams {
    // This struct represents tools with no parameters
}

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
    /// An object field
    Object,
    /// Any JSON value (object, array, etc.)
    Any,
}

/// Builder for creating JSON schemas for MCP tool registration in rmcp framework
#[derive(Clone, Default)]
pub struct ParameterBuilder {
    properties: Map<String, Value>,
    required: Vec<String>,
}

impl ParameterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a string property to the schema
    pub fn add_string_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert_field("type", JsonSchemaType::String);
        prop.insert_field("description", description);
        self.properties.insert_field(name, prop);

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
        prop.insert_field("type", JsonSchemaType::Array);

        let mut items = Map::new();
        items.insert_field("type", JsonSchemaType::String);
        prop.insert_field("items", items);

        prop.insert_field("description", description);
        self.properties.insert_field(name, prop);

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
        prop.insert_field("type", JsonSchemaType::Array);

        let mut items = Map::new();
        items.insert_field("type", JsonSchemaType::Number);
        prop.insert_field("items", items);

        prop.insert_field("description", description);
        self.properties.insert_field(name, prop);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a number property to the schema
    pub fn add_number_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert_field("type", JsonSchemaType::Number);
        prop.insert_field("description", description);
        self.properties.insert_field(name, prop);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a boolean property to the schema
    pub fn add_boolean_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert_field("type", JsonSchemaType::Boolean);
        prop.insert_field("description", description);
        self.properties.insert_field(name, prop);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add an object property to the schema
    pub fn add_object_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Object);
        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a property that can be any type (object, array, null, etc.)
    pub fn add_any_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        // Multiple types require a JSON array value
        prop.insert(
            "type".to_string(),
            vec![
                JsonSchemaType::Object.as_ref(),
                JsonSchemaType::Array.as_ref(),
                JsonSchemaType::Null.as_ref(),
            ]
            .into(),
        );
        prop.insert_field("description", description);
        self.properties.insert_field(name, prop);

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Build the final schema
    pub fn build(self) -> Arc<Map<String, Value>> {
        let mut schema = Map::new();
        schema.insert_field("type", JsonSchemaType::Object);
        schema.insert_field("properties", self.properties);

        if !self.required.is_empty() {
            schema.insert_field("required", self.required);
        }

        Arc::new(schema)
    }
}

/// Handle array type schemas and determine the array element type
fn handle_array_type(obj: &Map<String, Value>) -> ParameterType {
    obj.get_field(SchemaField::Items)
        .and_then(|items| items.as_object())
        .and_then(|items_obj| items_obj.get_field(SchemaField::Type))
        .and_then(|item_type| item_type.as_str())
        .map_or(ParameterType::Any, |item_type_str| match item_type_str {
            s if s == JsonSchemaType::String.as_ref() => ParameterType::StringArray,
            s if s == JsonSchemaType::Integer.as_ref() || s == JsonSchemaType::Number.as_ref() => {
                ParameterType::NumberArray
            }
            _ => ParameterType::Any,
        })
}

/// Handle string type values from schema type field
fn handle_string_type(type_str: &str, obj: &Map<String, Value>) -> ParameterType {
    match type_str {
        s if s == JsonSchemaType::String.as_ref() => ParameterType::String,
        s if s == JsonSchemaType::Integer.as_ref() || s == JsonSchemaType::Number.as_ref() => {
            ParameterType::Number
        }
        s if s == JsonSchemaType::Boolean.as_ref() => ParameterType::Boolean,
        s if s == JsonSchemaType::Object.as_ref() => ParameterType::Object,
        s if s == JsonSchemaType::Array.as_ref() => handle_array_type(obj),
        _ => ParameterType::Any,
    }
}

/// Handle array type values from schema type field (for Option<T> types)
fn handle_type_array(types: &[Value]) -> ParameterType {
    let non_null_types: Vec<&str> = types
        .iter()
        .filter_map(|v| v.as_str())
        .filter(|&t| t != JsonSchemaType::Null.as_ref())
        .collect();

    if non_null_types.len() == 1 {
        match non_null_types.first() {
            Some(&s) if s == JsonSchemaType::String.as_ref() => ParameterType::String,
            Some(&s)
                if s == JsonSchemaType::Integer.as_ref()
                    || s == JsonSchemaType::Number.as_ref() =>
            {
                ParameterType::Number
            }
            Some(&s) if s == JsonSchemaType::Boolean.as_ref() => ParameterType::Boolean,
            _ => ParameterType::Any,
        }
    } else {
        ParameterType::Any
    }
}

/// Handle oneOf schemas (typically enums)
fn handle_one_of_schema(one_of: &[Value]) -> Option<ParameterType> {
    let all_string_consts = one_of.iter().all(|variant| {
        variant
            .as_object()
            .and_then(|v| v.get_field(SchemaField::Type))
            .and_then(|t| t.as_str())
            .is_some_and(|t| t == JsonSchemaType::String.as_ref())
            && variant
                .as_object()
                .and_then(|v| v.get_field(SchemaField::Const))
                .is_some()
    });

    if all_string_consts {
        Some(ParameterType::String)
    } else {
        None
    }
}

/// Handle anyOf schemas (typically Option<T> types)
fn handle_any_of_schema(any_of: &[Value]) -> ParameterType {
    let mut has_null = false;
    let mut has_value_ref = false;

    for variant in any_of {
        if let Some(variant_obj) = variant.as_object() {
            // Check for null variants
            if variant_obj
                .get_field(SchemaField::Type)
                .and_then(|t| t.as_str())
                .is_some_and(|t| t == JsonSchemaType::Null.as_ref())
            {
                has_null = true;
                continue;
            }

            // Check if this is a $ref to Value type (serde_json::Value)
            if let Some(ref_str) = variant_obj
                .get_field(SchemaField::Ref)
                .and_then(|r| r.as_str())
            {
                // If we have a reference to Value type, it's an object parameter
                if ref_str.contains("Value") {
                    has_value_ref = true;
                    continue;
                }
                // For other $ref types, we can't easily resolve without full schema context
                continue;
            }

            // Try to map the variant directly
            let variant_schema = Schema::from(variant_obj.clone());
            let variant_type = map_schema_type_to_parameter_type(&variant_schema);
            if variant_type != ParameterType::Any {
                return variant_type;
            }
        }
    }

    // Special case: Option<Value> should be treated as Object for BRP parameters
    if has_null && has_value_ref {
        ParameterType::Object
    } else {
        ParameterType::Any
    }
}

fn map_schema_type_to_parameter_type(schema: &Schema) -> ParameterType {
    let Some(obj) = schema.as_object() else {
        return ParameterType::Any;
    };

    // Handle direct "type" field
    if let Some(type_value) = obj.get_field(SchemaField::Type) {
        return match type_value {
            Value::String(type_str) => handle_string_type(type_str, obj),
            Value::Array(types) => handle_type_array(types),
            _ => ParameterType::Any,
        };
    }

    // Handle objects with additionalProperties (HashMap pattern)
    if obj.get_field(SchemaField::AdditionalProperties).is_some() {
        return ParameterType::Object;
    }

    // Handle objects with only description (typically Option<Value> that has no type field)
    if obj.get_field(SchemaField::Description).is_some()
        && !obj.contains_key("type")
        && !obj.contains_key("anyOf")
        && !obj.contains_key("oneOf")
    {
        return ParameterType::Object;
    }

    // Handle "oneOf" schemas (enums like BrpMethod)
    if let Some(one_of) = obj.get_field(SchemaField::OneOf).and_then(|v| v.as_array())
        && let Some(param_type) = handle_one_of_schema(one_of)
    {
        return param_type;
    }

    // Handle "anyOf" schemas (typically Option<T> types)
    if let Some(any_of) = obj.get_field(SchemaField::AnyOf).and_then(|v| v.as_array()) {
        return handle_any_of_schema(any_of);
    }

    ParameterType::Any
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

    // let Some(properties) = root_obj
    //     .get_field(SchemaField::Properties)
    //     .and_then(|p| p.as_object())
    // else {
    //     return builder;
    // };

    let Some(properties) = root_obj.get_properties() else {
        return builder;
    };

    // Get the $defs section for resolving $ref references
    let defs = root_obj.get_field(SchemaField::Defs);

    let required_fields: HashSet<String> = root_obj
        .get_field(SchemaField::Required)
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

        // Resolve $ref if present
        let resolved_value = field_value
            .as_object()
            .and_then(|o| o.get_field(SchemaField::Ref))
            .and_then(|r| r.as_str())
            .and_then(|ref_path| {
                ref_path.strip_prefix("#/$defs/").and_then(|type_name| {
                    defs.and_then(|d| d.as_object())
                        .and_then(|d| d.get(type_name))
                })
            })
            .unwrap_or(field_value);

        // Convert the resolved JSON value to a Schema for processing
        let field_schema = if let Value::Object(obj) = resolved_value {
            Schema::from(obj.clone())
        } else if let Value::Bool(b) = resolved_value {
            Schema::from(*b)
        } else {
            continue; // Skip non-schema values
        };
        let param_type = map_schema_type_to_parameter_type(&field_schema);

        // Extract description from schema if available
        let description = resolved_value
            .as_object()
            .and_then(|obj| obj.get_field(SchemaField::Description))
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
            ParameterType::Object => builder.add_object_property(field_name, description, required),
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
