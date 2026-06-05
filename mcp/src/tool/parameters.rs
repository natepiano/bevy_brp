//! Parameter names, and tools to automatically create parameter definitions for rmcp from our
//! parameter structs
use std::collections::HashSet;
use std::sync::Arc;

use bevy_brp_mcp_macros::ParamStruct;
use schemars::JsonSchema;
use schemars::Schema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use strum::Display;
use strum::EnumString;

use super::constants::VALUE_TYPE_NAME;
use crate::constants::SCHEMA_REF_PREFIX;
use crate::support::IntoStrings;
use crate::support::JsonObjectAccess;
use crate::support::JsonSchemaType;
use crate::support::SchemaField;

/// Trait for parameter types used in tools
///
/// This trait provides a type-level constraint for tool parameter types.
/// It ensures that only valid parameter types can be used as associated types
/// in the `ToolFn` trait.
///
/// The trait is automatically implemented by the `ParamStruct` derive macro
/// for parameter structs.
pub trait ParamStruct:
    Send + Sync + serde::Serialize + serde::de::DeserializeOwned + JsonSchema
{
}

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
    /// `Entity` ID parameter
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

/// Whether a property is required in the JSON schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Required {
    Yes,
    No,
}

impl From<bool> for Required {
    fn from(value: bool) -> Self { if value { Self::Yes } else { Self::No } }
}

/// Which JSON containers a stringified value is allowed to be parsed into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AcceptedJson {
    ObjectOnly,
    ArrayOnly,
    ObjectOrArray,
}

/// Builder for creating JSON schemas for MCP tool registration in rmcp framework
#[derive(Clone, Default)]
pub struct ParameterBuilder {
    properties: Map<String, Value>,
    required:   Vec<String>,
}

impl ParameterBuilder {
    pub(super) fn new() -> Self { Self::default() }

    /// Add a string property to the schema
    fn add_string_property(mut self, name: &str, description: &str, required: Required) -> Self {
        let mut prop = Map::new();
        prop.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::String);
        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        self.mark_required(name, required);
        self
    }

    /// Add a string array property to the schema
    fn add_string_array_property(
        mut self,
        name: &str,
        description: &str,
        required: Required,
    ) -> Self {
        let mut prop = Map::new();
        prop.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Array);

        let mut items = Map::new();
        items.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::String);
        prop.insert_field(SchemaField::Items.as_ref(), items);

        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        self.mark_required(name, required);
        self
    }

    /// Add a number array property to the schema
    fn add_number_array_property(
        mut self,
        name: &str,
        description: &str,
        required: Required,
    ) -> Self {
        let mut prop = Map::new();
        prop.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Array);

        let mut items = Map::new();
        items.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Number);
        prop.insert_field(SchemaField::Items.as_ref(), items);

        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        self.mark_required(name, required);
        self
    }

    /// Add a number property to the schema
    fn add_number_property(mut self, name: &str, description: &str, required: Required) -> Self {
        let mut prop = Map::new();
        prop.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Number);
        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        self.mark_required(name, required);
        self
    }

    /// Add a boolean property to the schema
    fn add_boolean_property(mut self, name: &str, description: &str, required: Required) -> Self {
        let mut prop = Map::new();
        prop.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Boolean);
        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        self.mark_required(name, required);
        self
    }

    /// Add an object property to the schema
    fn add_object_property(mut self, name: &str, description: &str, required: Required) -> Self {
        let mut prop = Map::new();
        prop.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Object);
        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        self.mark_required(name, required);
        self
    }

    /// Add a property that can be any JSON type (object, array, string, number, boolean, null)
    fn add_any_property(mut self, name: &str, description: &str, required: Required) -> Self {
        let mut prop = Map::new();
        // Use anyOf instead of type array to satisfy validators that require
        // array schemas to have an "items" field (e.g., Copilot).
        let mut object_schema = Map::new();
        object_schema.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Object);

        let mut array_schema = Map::new();
        array_schema.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Array);
        array_schema.insert_field(SchemaField::Items.as_ref(), Map::<String, Value>::new());

        let mut string_schema = Map::new();
        string_schema.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::String);

        let mut number_schema = Map::new();
        number_schema.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Number);

        let mut boolean_schema = Map::new();
        boolean_schema.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Boolean);

        let mut null_schema = Map::new();
        null_schema.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Null);

        let any_of = vec![
            Value::Object(object_schema),
            Value::Object(array_schema),
            Value::Object(string_schema),
            Value::Object(number_schema),
            Value::Object(boolean_schema),
            Value::Object(null_schema),
        ];
        prop.insert_field(SchemaField::AnyOf.as_ref(), Value::Array(any_of));
        prop.insert_field(SchemaField::Description.as_ref(), description);
        self.properties.insert_field(name, prop);

        self.mark_required(name, required);
        self
    }

    fn mark_required(&mut self, name: &str, required: Required) {
        match required {
            Required::Yes => self.required.push(name.to_string()),
            Required::No => {},
        }
    }

    /// Build the final schema
    pub(super) fn build(self) -> Arc<Map<String, Value>> {
        let mut schema = Map::new();
        schema.insert_field(SchemaField::Type.as_ref(), JsonSchemaType::Object);
        schema.insert_field(SchemaField::Properties.as_ref(), self.properties);

        if !self.required.is_empty() {
            schema.insert_field(SchemaField::Required.as_ref(), self.required);
        }

        Arc::new(schema)
    }
}

/// Handle array type schemas and determine the array element type
fn handle_array_type(object: &Map<String, Value>) -> ParameterType {
    object
        .get_field(SchemaField::Items)
        .and_then(|items| items.as_object())
        .and_then(|items_obj| items_obj.get_field(SchemaField::Type))
        .and_then(Value::as_str)
        .map_or(ParameterType::Any, |item_type_str| match item_type_str {
            s if s == JsonSchemaType::String.as_ref() => ParameterType::StringArray,
            s if s == JsonSchemaType::Integer.as_ref() || s == JsonSchemaType::Number.as_ref() => {
                ParameterType::NumberArray
            },
            _ => ParameterType::Any,
        })
}

/// Handle string type values from schema type field
fn handle_string_type(type_str: &str, object: &Map<String, Value>) -> ParameterType {
    match type_str {
        s if s == JsonSchemaType::String.as_ref() => ParameterType::String,
        s if s == JsonSchemaType::Integer.as_ref() || s == JsonSchemaType::Number.as_ref() => {
            ParameterType::Number
        },
        s if s == JsonSchemaType::Boolean.as_ref() => ParameterType::Boolean,
        s if s == JsonSchemaType::Object.as_ref() => ParameterType::Object,
        s if s == JsonSchemaType::Array.as_ref() => handle_array_type(object),
        _ => ParameterType::Any,
    }
}

/// Handle array type values from schema type field (for `Option<T>` types)
fn handle_type_array(types: &[Value], object: &Map<String, Value>) -> ParameterType {
    let non_null_types: Vec<&str> = types
        .iter()
        .filter_map(Value::as_str)
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
            },
            Some(&s) if s == JsonSchemaType::Boolean.as_ref() => ParameterType::Boolean,
            Some(&s) if s == JsonSchemaType::Object.as_ref() => ParameterType::Object,
            Some(&s) if s == JsonSchemaType::Array.as_ref() => handle_array_type(object),
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
            .and_then(Value::as_str)
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

/// Handle `anyOf` schemas (typically `Option<T>` types)
fn handle_any_of_schema(any_of: &[Value]) -> ParameterType {
    for variant in any_of {
        if let Some(variant_obj) = variant.as_object() {
            // Skip null variants (from Option<T>)
            if variant_obj
                .get_field(SchemaField::Type)
                .and_then(Value::as_str)
                .is_some_and(|t| t == JsonSchemaType::Null.as_ref())
            {
                continue;
            }

            // Check if this is a $ref type
            if let Some(ref_str) = variant_obj
                .get_field(SchemaField::Ref)
                .and_then(Value::as_str)
            {
                // serde_json::Value refs should fall through to Any
                if ref_str.contains(VALUE_TYPE_NAME) {
                    continue;
                }
                // For other $ref types (like `BrpQueryFilter`), treat as Object
                // since they're references to custom structs
                return ParameterType::Object;
            }

            // Try to map the variant directly
            let variant_schema = Schema::from(variant_obj.clone());
            let variant_type = map_schema_type_to_parameter_type(&variant_schema);
            if variant_type != ParameterType::Any {
                return variant_type;
            }
        }
    }

    // Default to Any - handles Option<Value> and other unrecognized patterns
    // serde_json::Value can be any JSON type (object, array, string, number, boolean, null)
    ParameterType::Any
}

fn map_schema_type_to_parameter_type(schema: &Schema) -> ParameterType {
    let Some(object) = schema.as_object() else {
        return ParameterType::Any;
    };

    // Handle direct "type" field
    if let Some(type_value) = object.get_field(SchemaField::Type) {
        let parameter_type = match type_value {
            Value::String(type_str) => handle_string_type(type_str, object),
            Value::Array(types) => handle_type_array(types, object),
            _ => ParameterType::Any,
        };
        return parameter_type;
    }

    // Handle objects with `additionalProperties` (`HashMap` pattern)
    if object
        .get_field(SchemaField::AdditionalProperties)
        .is_some()
    {
        return ParameterType::Object;
    }

    // Handle objects with only description (typically `serde_json::Value` that has no type field)
    // This should be Any since Value can hold any JSON type
    if object.get_field(SchemaField::Description).is_some()
        && !object.contains_key(SchemaField::Type.as_ref())
        && !object.contains_key(SchemaField::AnyOf.as_ref())
        && !object.contains_key(SchemaField::OneOf.as_ref())
    {
        return ParameterType::Any;
    }

    // Handle "oneOf" schemas (enums like `BrpMethod`)
    if let Some(one_of) = object
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
        && let Some(param_type) = handle_one_of_schema(one_of)
    {
        return param_type;
    }

    // Handle "anyOf" schemas (typically Option<T> types)
    if let Some(any_of) = object
        .get_field(SchemaField::AnyOf)
        .and_then(Value::as_array)
    {
        return handle_any_of_schema(any_of);
    }

    ParameterType::Any
}

fn resolve_schema_value<'a>(
    field_value: &'a Value,
    defs: Option<&'a Map<String, Value>>,
) -> &'a Value {
    field_value
        .as_object()
        .and_then(|object| object.get_field(SchemaField::Ref))
        .and_then(Value::as_str)
        .and_then(|ref_path| {
            ref_path
                .strip_prefix(SCHEMA_REF_PREFIX)
                .and_then(|type_name| defs.and_then(|definitions| definitions.get(type_name)))
        })
        .unwrap_or(field_value)
}

fn schema_from_value(value: &Value) -> Option<Schema> {
    match value {
        Value::Object(obj) => Some(Schema::from(obj.clone())),
        Value::Bool(boolean) => Some(Schema::from(*boolean)),
        _ => None,
    }
}

fn normalize_stringified_json(value: &mut Value, accepted: AcceptedJson) {
    let Value::String(string) = value else {
        return;
    };

    let trimmed = string.trim();
    let accepts_object = matches!(
        accepted,
        AcceptedJson::ObjectOnly | AcceptedJson::ObjectOrArray
    );
    let accepts_array = matches!(
        accepted,
        AcceptedJson::ArrayOnly | AcceptedJson::ObjectOrArray
    );
    let looks_like_object = accepts_object && trimmed.starts_with('{') && trimmed.ends_with('}');
    let looks_like_array = accepts_array && trimmed.starts_with('[') && trimmed.ends_with(']');

    if !(looks_like_object || looks_like_array) {
        return;
    }

    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        *value = parsed;
    }
}

fn normalize_argument_value(value: &mut Value, schema: &Schema) {
    match map_schema_type_to_parameter_type(schema) {
        ParameterType::Object => normalize_stringified_json(value, AcceptedJson::ObjectOnly),
        ParameterType::StringArray | ParameterType::NumberArray => {
            normalize_stringified_json(value, AcceptedJson::ArrayOnly);
        },
        ParameterType::Any => normalize_stringified_json(value, AcceptedJson::ObjectOrArray),
        ParameterType::Number | ParameterType::String | ParameterType::Boolean => {},
    }
}

/// Parse stringified JSON values at the MCP boundary for fields whose schema
/// accepts structured JSON. Numeric, string, and boolean fields are left as-is
/// so type mismatches surface as serde errors rather than being silently coerced.
pub(super) fn normalize_arguments_for<T: JsonSchema>(arguments: &mut Map<String, Value>) {
    let schema = schemars::schema_for!(T);
    let Some(root_obj) = schema.as_object() else {
        return;
    };
    let Some(properties) = root_obj.get_properties() else {
        return;
    };
    let defs = root_obj
        .get_field(SchemaField::Defs)
        .and_then(Value::as_object);

    for (field_name, value) in arguments {
        let Some(field_schema_value) = properties.get(field_name) else {
            continue;
        };
        let resolved_schema_value = resolve_schema_value(field_schema_value, defs);
        let Some(field_schema) = schema_from_value(resolved_schema_value) else {
            continue;
        };

        normalize_argument_value(value, &field_schema);
    }
}

/// Build parameters from a `JsonSchema` type directly into a `ParameterBuilder`
/// All tools with parameters derive `JsonSchema` making it possible for us
/// to build the parameters from the schema
pub(super) fn build_parameters_from<T: JsonSchema>() -> ParameterBuilder {
    let schema = schemars::schema_for!(T);
    let mut parameter_builder = ParameterBuilder::new();

    let Some(root_obj) = schema.as_object() else {
        return parameter_builder;
    };

    // let Some(properties) = root_obj
    //     .get_field(SchemaField::Properties)
    //     .and_then(|p| p.as_object())
    // else {
    //     return builder;
    // };

    let Some(properties) = root_obj.get_properties() else {
        return parameter_builder;
    };

    // Get the $defs section for resolving $ref references
    let defs = root_obj.get_field(SchemaField::Defs);

    let required_fields: HashSet<String> = root_obj
        .get_field(SchemaField::Required)
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .into_strings()
                .into_iter()
                .collect()
        })
        .unwrap_or_default();

    for (field_name, field_value) in properties {
        let required = Required::from(required_fields.contains(field_name));

        let resolved_value = resolve_schema_value(field_value, defs.and_then(Value::as_object));

        let Some(field_schema) = schema_from_value(resolved_value) else {
            continue; // Skip non-schema values
        };
        let param_type = map_schema_type_to_parameter_type(&field_schema);

        // Extract description from schema if available
        let description = resolved_value
            .as_object()
            .and_then(|object| object.get_field(SchemaField::Description))
            .and_then(Value::as_str)
            .unwrap_or(field_name.as_str());

        // Add to builder based on type
        parameter_builder = match param_type {
            ParameterType::String => {
                parameter_builder.add_string_property(field_name, description, required)
            },
            ParameterType::Number => {
                parameter_builder.add_number_property(field_name, description, required)
            },
            ParameterType::Boolean => {
                parameter_builder.add_boolean_property(field_name, description, required)
            },
            ParameterType::StringArray => {
                parameter_builder.add_string_array_property(field_name, description, required)
            },
            ParameterType::NumberArray => {
                parameter_builder.add_number_array_property(field_name, description, required)
            },
            ParameterType::Object => {
                parameter_builder.add_object_property(field_name, description, required)
            },
            ParameterType::Any => {
                parameter_builder.add_any_property(field_name, description, required)
            },
        };
    }

    parameter_builder
}

impl From<ParameterName> for String {
    fn from(param: ParameterName) -> Self { param.as_ref().to_string() }
}
#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::app_tools::LaunchBevyBinaryParams;
    use crate::brp_tools::MutateComponentsParams;

    #[test]
    fn normalize_arguments_for_does_not_coerce_numeric_strings() {
        let mut arguments = Map::new();
        arguments.insert(
            String::from("instance_count"),
            Value::String(String::from("3")),
        );
        arguments.insert(String::from("port"), Value::String(String::from("15702")));
        arguments.insert(
            String::from("target_name"),
            Value::String(String::from("42")),
        );

        normalize_arguments_for::<LaunchBevyBinaryParams>(&mut arguments);

        assert_eq!(
            arguments.get("instance_count"),
            Some(&Value::String(String::from("3")))
        );
        assert_eq!(
            arguments.get("port"),
            Some(&Value::String(String::from("15702")))
        );
        assert_eq!(
            arguments.get("target_name"),
            Some(&Value::String(String::from("42")))
        );
    }

    #[test]
    fn normalize_arguments_for_mutation_params_parses_stringified_json() {
        let mut arguments = Map::new();
        arguments.insert(String::from("entity"), serde_json::json!(1));
        arguments.insert(String::from("component"), Value::String(String::from("42")));
        arguments.insert(
            String::from("value"),
            Value::String(String::from(r#"{"nested":true}"#)),
        );
        arguments.insert(String::from("port"), serde_json::json!(15702));

        normalize_arguments_for::<MutateComponentsParams>(&mut arguments);

        assert_eq!(
            arguments.get("component"),
            Some(&Value::String(String::from("42")))
        );
        assert_eq!(
            arguments.get("value"),
            Some(&serde_json::json!({ "nested": true }))
        );
        assert_eq!(arguments.get("port"), Some(&serde_json::json!(15702)));
    }

    /// Regression test: `add_any_property` must emit anyOf where the array branch
    /// includes an "items" key. Without this, Copilot rejects the schema with:
    ///   "400 Invalid schema: array schema missing items"
    #[test]
    fn add_any_property_array_branch_has_items() -> Result<(), Box<dyn Error>> {
        let schema = ParameterBuilder::new()
            .add_any_property("value", "Any JSON value", Required::Yes)
            .build();

        // 使用 .ok_or(...)? 代替 .expect(...)
        let any_of = schema["properties"]["value"]["anyOf"]
            .as_array()
            .ok_or("anyOf must be an array")?;

        let array_branch = any_of
            .iter()
            .find(|v| v.get("type").and_then(Value::as_str) == Some("array"))
            .ok_or("anyOf must contain an array branch")?;

        assert!(
            array_branch.get("items").is_some(),
            "array branch in anyOf must have an 'items' key (Copilot schema validation requirement)"
        );

        Ok(())
    }

    /// Verify `add_any_property` covers all six JSON primitive types in anyOf.
    #[test]
    fn add_any_property_covers_all_json_types() -> Result<(), Box<dyn Error>> {
        let schema = ParameterBuilder::new()
            .add_any_property("value", "Any JSON value", Required::Yes)
            .build();

        let any_of = schema["properties"]["value"]["anyOf"]
            .as_array()
            .ok_or("anyOf must be an array")?;

        let types: Vec<&str> = any_of
            .iter()
            .filter_map(|v| v.get("type")?.as_str())
            .collect();

        for expected in &["object", "array", "string", "number", "boolean", "null"] {
            assert!(
                types.contains(expected),
                "anyOf must include type '{expected}'"
            );
        }

        Ok(())
    }
}
