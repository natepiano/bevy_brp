//! Example value for a mutation path
//!
//! This enum ensures we cannot accidentally use the wrong example format for a path.
//! Enum roots MUST use `EnumRoot` variant, non-enum paths MUST use `Simple` variant.
use serde::Deserialize;
use serde::Serialize;

use super::types::Example;
use super::types::ExampleGroup;

#[derive(Debug, Clone)]
pub enum PathExample {
    /// Simple value example used by non-enum types
    ///
    /// Examples:
    /// - Structs: `{"field1": value1, "field2": value2}`
    /// - Primitives: `42`, `"text"`, `true`
    /// - `Option::None`: `null` (special case for Option enum)
    Simple(Example),

    /// Enum root with variant groups and parent assembly value
    ///
    /// Only used for enum root paths.
    /// The `for_parent` field provides the simplified example that parent types
    /// use when assembling their own examples.
    EnumRoot {
        /// All variant groups for this enum (the `examples` array in JSON output)
        groups: Vec<ExampleGroup>,
        /// Simplified example for parent assembly
        for_parent: Example,
    },
}

impl PathExample {
    /// Get the value to use for parent assembly
    ///
    /// For `Simple`, returns the example directly.
    /// For `EnumRoot`, returns the `for_parent` field.
    ///
    /// This is the ONLY helper method provided. All other usage should use explicit
    /// pattern matching to maintain type safety and force exhaustive handling of both cases.
    pub const fn for_parent(&self) -> &Example {
        match self {
            Self::Simple(ex) => ex,
            Self::EnumRoot { for_parent, .. } => for_parent,
        }
    }
}

/// Custom serialization for `PathExample` that flattens into parent struct
///
/// This produces the correct JSON format for `MutationPathExternal`:
/// - `Simple(example)` → `"example": <value>` (skipped if value is null)
/// - `EnumRoot { groups, .. }` → `"examples": <groups>`
impl Serialize for PathExample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Simple(example) => {
                let value = example.to_value();
                // Skip serializing null examples to match V1 behavior
                if value.is_null() {
                    serializer.serialize_map(Some(0))?.end()
                } else {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("example", &value)?;
                    map.end()
                }
            }
            Self::EnumRoot { groups, .. } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("examples", groups)?;
                map.end()
            }
        }
    }
}

/// Stub `Deserialize` implementation for `PathExample`
///
/// This is required by serde's flatten attribute but never actually used
/// since we only serialize `MutationPathExternal`, never deserialize it.
impl<'de> Deserialize<'de> for PathExample {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "PathExample deserialization not implemented - this type is write-only",
        ))
    }
}
