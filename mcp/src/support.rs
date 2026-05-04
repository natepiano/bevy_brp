//! JSON schema helpers, JSON access traits, and shared serde utilities.

use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use strum::AsRefStr;
use strum::Display;
use strum::EnumString;

use crate::brp_tools::BrpTypeName;
use crate::constants::SCHEMA_REF_PREFIX;

/// JSON schema type names for type schema generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, Serialize, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum JsonSchemaType {
    Object,
    Array,
    String,
    Number,
    Integer,
    Boolean,
    Null,
}

impl From<JsonSchemaType> for Value {
    fn from(schema_type: JsonSchemaType) -> Self { Self::String(schema_type.as_ref().to_string()) }
}

/// Registry schema field names.
///
/// This enum provides type-safe field names for JSON schema structures. It's
/// used throughout the codebase to avoid hardcoded strings when accessing
/// schema fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr)]
#[strum(serialize_all = "camelCase")]
pub(crate) enum SchemaField {
    /// The `additionalProperties` field for `HashMap` types.
    AdditionalProperties,
    /// The `anyOf` field for union types.
    #[strum(serialize = "anyOf")]
    AnyOf,
    /// The `const` field for constant values.
    Const,
    /// The crate name field.
    CrateName,
    /// The `$defs` field for schema definitions.
    #[strum(serialize = "$defs")]
    Defs,
    /// The description field.
    Description,
    /// The `items` field for array types.
    Items,
    /// Map key.
    Key,
    /// The `keyType` field for map types.
    KeyType,
    /// The kind field for type categories.
    Kind,
    /// The module path field.
    ModulePath,
    /// The `oneOf` field for enum variants.
    OneOf,
    /// The `prefixItems` field for tuple types.
    PrefixItems,
    /// The `properties` field for object types.
    Properties,
    /// The `$ref` field for type references.
    #[strum(serialize = "$ref")]
    Ref,
    /// The reflect types field.
    ReflectTypes,
    /// The `required` field for object types.
    Required,
    /// The short path field.
    ShortPath,
    /// The type field.
    Type,
    /// The type path field (e.g., "`bevy_color::color::Color::Srgba`").
    TypePath,
    /// Map value.
    Value,
    /// The `valueType` field for map types.
    ValueType,
}

/// Extension trait for type-safe JSON field access.
pub(crate) trait JsonObjectAccess {
    /// Get a field value using any type that can be borrowed as `str`.
    fn get_field<T: AsRef<str>>(&self, field: T) -> Option<&Value>;

    /// Get a field value as `str`.
    fn get_field_str<T: AsRef<str>>(&self, field: T) -> Option<&str>;

    /// Get a field value as an owned `String`.
    fn get_field_string<T: AsRef<str>>(&self, field: T) -> Option<String> {
        self.get_field_str(field).map(String::from)
    }

    /// Get a field value as an array.
    fn get_field_array<T: AsRef<str>>(&self, field: T) -> Option<&[Value]> {
        self.get_field(field)
            .and_then(Value::as_array)
            .map(Vec::as_slice)
    }

    /// Insert a field with a value using types that convert to `String` and `Value`.
    fn insert_field<F, V>(&mut self, field: F, value: V)
    where
        F: Into<String>,
        V: Into<Value>;

    /// Extract a `BrpTypeName` from a field definition that contains a `type.$ref`
    /// structure.
    ///
    /// This method expects the JSON value to have the structure:
    /// ```json
    /// { "type": { "$ref": "#/$defs/SomeType" } }
    /// ```
    /// and extracts "`SomeType`" as a `BrpTypeName`.
    fn extract_field_type(&self) -> Option<BrpTypeName> {
        self.get_field(SchemaField::Type)
            .and_then(|ty| ty.get_field(SchemaField::Ref))
            .and_then(Value::as_str)
            .and_then(|ref_str| ref_str.strip_prefix(SCHEMA_REF_PREFIX))
            .map(BrpTypeName::from)
    }

    /// Extract a single type reference from a schema field such as `Items`,
    /// `KeyType`, or `ValueType`.
    fn get_type(&self, field: SchemaField) -> Option<BrpTypeName> {
        let field_value = self.get_field(field)?;
        field_value.extract_field_type()
    }

    /// Get the `Properties` field as a `Map`.
    fn get_properties(&self) -> Option<&Map<String, Value>> {
        self.get_field(SchemaField::Properties)
            .and_then(Value::as_object)
    }

    /// Check whether this JSON value represents a complex (non-primitive) type.
    ///
    /// Complex types (`Array`, `Object`) cannot be used as `HashMap` keys or
    /// `HashSet` elements in BRP.
    fn is_complex_type(&self) -> bool;
}

impl JsonObjectAccess for Value {
    fn get_field<T: AsRef<str>>(&self, field: T) -> Option<&Self> { self.get(field.as_ref()) }

    fn get_field_str<T: AsRef<str>>(&self, field: T) -> Option<&str> {
        self.get(field.as_ref()).and_then(Self::as_str)
    }

    fn insert_field<F, V>(&mut self, field: F, value: V)
    where
        F: Into<String>,
        V: Into<Self>,
    {
        if let Some(obj) = self.as_object_mut() {
            obj.insert(field.into(), value.into());
        }
    }

    fn is_complex_type(&self) -> bool { matches!(self, Self::Array(_) | Self::Object(_)) }
}

impl JsonObjectAccess for Map<String, Value> {
    fn get_field<T: AsRef<str>>(&self, field: T) -> Option<&Value> { self.get(field.as_ref()) }

    fn get_field_str<T: AsRef<str>>(&self, field: T) -> Option<&str> {
        self.get(field.as_ref()).and_then(Value::as_str)
    }

    fn insert_field<F, V>(&mut self, field: F, value: V)
    where
        F: Into<String>,
        V: Into<Value>,
    {
        self.insert(field.into(), value.into());
    }

    fn is_complex_type(&self) -> bool {
        // A `Map` is always a complex type (`Object`).
        true
    }
}

/// Extension trait for converting iterators to `Vec<String>`.
///
/// This trait provides a convenient way to collect iterators of
/// string-convertible items into a vector of strings, replacing the common
/// `.map(String::from).collect()` pattern with a more expressive
/// `.into_strings()` call.
///
/// # Examples
///
/// ```
/// use json_traits::IntoStrings;
///
/// // Convert iterator of &str to Vec<String>
/// let strings = ["a", "b", "c"].iter().into_strings();
///
/// // Works with filter chains
/// let filtered = ["hello", "", "world"]
///     .iter()
///     .filter(|s| !s.is_empty())
///     .into_strings();
///
/// // Works with enums that implement Into<String>
/// let variants = enum_values.iter().into_strings();
/// ```
pub(crate) trait IntoStrings<T> {
    /// Convert an iterator of items that can become strings into a `Vec<String>`.
    fn into_strings(self) -> Vec<String>;
}

impl<I, T> IntoStrings<T> for I
where
    I: Iterator<Item = T>,
    T: Into<String>,
{
    fn into_strings(self) -> Vec<String> { self.map(Into::into).collect() }
}
