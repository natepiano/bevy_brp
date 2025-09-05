//! Core types for mutation path building
//!
//! This module contains the fundamental types used throughout the mutation path building system,
//! including the trait definition, context structures, and all mutation path related types.
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use super::super::constants::RecursionDepth;
use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, MutationKnowledge};
use super::super::response_types::{BrpTypeName, SchemaField};
use super::TypeKind;
use crate::brp_tools::brp_type_schema::constants::SCHEMA_REF_PREFIX;
use crate::brp_tools::brp_type_schema::mutation_knowledge::KnowledgeKey;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

/// Status of whether a mutation path can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Path can be fully mutated
    Mutatable,
    /// Path cannot be mutated (missing traits, unsupported type, etc.)
    NotMutatable,
    /// Path is partially mutatable (some elements mutable, others not)
    PartiallyMutatable,
}

/// Context for a mutation path describing what kind of mutation this is
#[derive(Debug, Clone, Deserialize)]
pub enum MutationPathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField {
        field_name:  String,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in a tuple by index
    /// Applies to tuple elements, enums variants, including generics such as Option<T>
    IndexedElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in an array
    ArrayElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
}

impl MutationPathKind {
    /// Generate a human-readable description for this mutation
    pub fn description(&self) -> String {
        match self {
            Self::RootValue { type_name } => {
                format!("Replace the entire {type_name} value")
            }
            Self::StructField {
                field_name,
                parent_type,
            } => {
                format!("Mutate the {field_name} field of {parent_type}")
            }
            Self::IndexedElement { index, parent_type } => {
                format!("Mutate element {index} of {parent_type}")
            }
            Self::ArrayElement { index, parent_type } => {
                format!("Mutate element [{index}] of {parent_type}")
            }
        }
    }

    /// Get just the variant name for serialization
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::RootValue { .. } => "RootValue",
            Self::StructField { .. } => "StructField",
            Self::IndexedElement { .. } => "TupleElement",
            Self::ArrayElement { .. } => "ArrayElement",
        }
    }
}

impl Display for MutationPathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.variant_name())
    }
}

impl Serialize for MutationPathKind {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path
    pub example:         Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:            String,
    /// For enum types, list of valid variant names
    pub enum_variants:   Option<Vec<String>>,
    /// Type information for this path
    pub type_name:       BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:       MutationPathKind,
    /// Status of whether this path can be mutated
    pub mutation_status: MutationStatus,
    /// Error reason if mutation is not possible
    pub error_reason:    Option<String>,
}

/// Path information combining navigation and type metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind: MutationPathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name: BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind: TypeKind,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPath {
    /// Human-readable description of what this path mutates
    pub description:      String,
    /// Combined path navigation and type metadata
    pub path_info:        PathInfo,
    /// Status of whether this path can be mutated
    pub mutation_status:  MutationStatus,
    /// Error reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason:     Option<String>,
    /// Example value for mutations (for non-Option types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:          Option<Value>,
    /// Example value for setting Some variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_some:     Option<Value>,
    /// Example value for setting None variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_none:     Option<Value>,
    /// List of valid enum variants for this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_variants:    Option<Vec<String>>,
    /// Example values for enum variants (maps variant names to example JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_variants: Option<HashMap<String, Value>>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:             Option<String>,
}

impl MutationPath {
    /// Create a root value mutation with a simplified type name
    pub fn new_root_value(
        type_name: BrpTypeName,
        example_value: Value,
        simplified_type: String,
    ) -> Self {
        Self {
            description:      format!("Replace the entire {type_name} value"),
            path_info:        PathInfo {
                path_kind: MutationPathKind::RootValue { type_name },
                type_name: BrpTypeName::from(simplified_type),
                type_kind: TypeKind::Value, // Root values are treated as Value types
            },
            example:          Some(example_value),
            example_some:     None,
            example_none:     None,
            enum_variants:    None,
            example_variants: None,
            note:             None,
            mutation_status:  MutationStatus::Mutatable,
            error_reason:     None,
        }
    }

    /// Create from internal `MutationPath` with proper formatting logic
    pub fn from_mutation_path(
        path: &MutationPathInternal,
        description: String,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        // Regular non-Option path
        let example_variants = if path.enum_variants.is_some() {
            // This is an enum type - generate example variants using the new system
            let enum_type = Some(&path.type_name); // Extract enum type from path
            let examples = super::build_all_enum_examples(type_schema, registry, 0, enum_type); // Pass both
            if examples.is_empty() {
                None
            } else {
                Some(examples)
            }
        } else {
            None
        };

        // Compute enum_variants from example_variants keys (alphabetically sorted)
        let enum_variants = example_variants.as_ref().map(|variants| {
            let mut keys: Vec<String> = variants.keys().cloned().collect();
            keys.sort(); // Alphabetical sorting for consistency
            keys
        });

        // Get TypeKind for the field type
        let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema, &path.type_name);

        Self {
            description,
            path_info: PathInfo {
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
            },
            example: if path.example.is_null() {
                None
            } else {
                Some(path.example.clone())
            },
            example_some: None,
            example_none: None,
            enum_variants,
            example_variants,
            note: None,
            mutation_status: path.mutation_status,
            error_reason: path.error_reason.clone(),
        }
    }
}

/// Trait for building mutation paths for different type kinds
///
/// This trait provides type-directed dispatch for mutation path building,
/// replacing the large conditional match statement with clean separation of concerns.
/// Each type kind gets its own implementation that handles the specific logic needed.
pub trait MutationPathBuilder {
    /// Build mutation paths with depth tracking for recursion safety
    ///
    /// This method takes a `MutationPathContext` which provides all necessary information
    /// including the registry, wrapper info, and enum variants, plus a `RecursionDepth`
    /// parameter to track recursion depth and prevent infinite loops.
    ///
    /// Returns a `Result` containing a vector of `MutationPathInternal` representing
    /// all possible mutation paths, or an error if path building failed.
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>>;
}

/// Context for building mutation paths - handles root vs field scenarios
/// necessary because Struct, specifically, allows us to recurse down a level
/// for complex types that have Struct fields
#[derive(Debug, Clone)]
pub enum RootOrField {
    /// Building paths for a root type (used in root mutations)
    Root { type_name: BrpTypeName },
    /// Building paths for a field within a parent type
    Field {
        field_name:  String,
        field_type:  BrpTypeName,
        parent_type: BrpTypeName,
    },
}

impl RootOrField {
    /// Create a field context
    pub fn field(field_name: &str, field_type: &BrpTypeName, parent_type: &BrpTypeName) -> Self {
        Self::Field {
            field_name:  field_name.to_string(),
            field_type:  field_type.clone(),
            parent_type: parent_type.clone(),
        }
    }

    /// Create a root context
    pub fn root(type_name: &BrpTypeName) -> Self {
        Self::Root {
            type_name: type_name.clone(),
        }
    }

    /// Get the type being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        match self {
            Self::Root { type_name } => type_name,
            Self::Field { field_type, .. } => field_type,
        }
    }
}

/// Context for mutation path building operations
///
/// This struct provides all the necessary context for building mutation paths,
/// including access to the registry, wrapper type information, and enum variants.
#[derive(Debug)]
pub struct MutationPathContext {
    /// The building context (root or field)
    pub location:         RootOrField,
    /// Reference to the type registry
    pub registry:         Arc<HashMap<BrpTypeName, Value>>,
    /// Path prefix for nested structures (e.g., ".translation" when building Vec3 fields)
    pub path_prefix:      String,
    /// Parent's mutation knowledge for extracting component examples
    pub parent_knowledge: Option<&'static MutationKnowledge>,
}

impl MutationPathContext {
    /// Create a new mutation path context
    pub const fn new(location: RootOrField, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        Self {
            location,
            registry,
            path_prefix: String::new(),
            parent_knowledge: None,
        }
    }

    /// Get the type name being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        self.location.type_name()
    }

    /// Require the schema to be present, logging a warning if missing
    /// Looks up the schema from the registry based on the current type
    pub fn require_schema(&self) -> Option<&Value> {
        self.registry.get(self.type_name()).or_else(|| {
            warn!(
                type_name = %self.type_name(),
                "Schema missing for type - mutation paths may be incomplete"
            );
            None
        })
    }

    /// Look up a type in the registry
    pub fn get_type_schema(&self, type_name: &BrpTypeName) -> Option<&Value> {
        self.registry.get(type_name)
    }

    /// Create a new context for a child element (field, array element, tuple element)
    /// The accessor should include the appropriate punctuation (e.g., ".field", "[0]", ".0")
    pub fn create_field_context(&self, accessor: &str, field_type: &BrpTypeName) -> Self {
        let parent_type = self.type_name();
        // Build the new path prefix by appending the accessor to the current prefix
        let new_path_prefix = format!("{}{}", self.path_prefix, accessor);

        // Extract just the field name from accessor for the location
        // Remove leading "." or "[" and trailing "]" to get the name/index
        let field_name = accessor
            .trim_start_matches('.')
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();

        // Check if field type has hardcoded knowledge to pass to children
        let field_knowledge = BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(field_type));

        Self {
            location:         RootOrField::field(&field_name, field_type, parent_type),
            registry:         Arc::clone(&self.registry),
            path_prefix:      new_path_prefix,
            parent_knowledge: field_knowledge,
        }
    }

    /// Return an example value unchanged (wrapper functionality removed)
    pub const fn wrap_example(inner_value: Value) -> Value {
        inner_value
    }

    /// Check if a value type has serialization support
    /// Used to determine if opaque Value types like String can be mutated
    pub fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
        use crate::brp_tools::brp_type_schema::response_types::ReflectTrait;

        self.get_type_schema(type_name).is_some_and(|schema| {
            let reflect_types: Vec<ReflectTrait> =
                Self::get_schema_field_as_array(schema, SchemaField::ReflectTypes)
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| s.parse().ok())
                    .collect();

            reflect_types.contains(&ReflectTrait::Serialize)
                && reflect_types.contains(&ReflectTrait::Deserialize)
        })
    }

    /// Extract element type from List or Array schema
    pub fn extract_list_element_type(schema: &Value) -> Option<BrpTypeName> {
        schema
            .get("items")
            .and_then(|items| items.get_field(SchemaField::Type))
            .and_then(Self::extract_type_ref_with_schema_field)
    }

    /// Extract value type from Map schema
    pub fn extract_map_value_type(schema: &Value) -> Option<BrpTypeName> {
        schema
            .get("additionalProperties")
            .and_then(|props| props.get_field(SchemaField::Type))
            .and_then(Self::extract_type_ref_with_schema_field)
    }

    /// Extract all element types from Tuple/TupleStruct schema
    pub fn extract_tuple_element_types(schema: &Value) -> Option<Vec<BrpTypeName>> {
        Self::get_schema_field_as_array(schema, SchemaField::PrefixItems).map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get_field(SchemaField::Type)
                        .and_then(Self::extract_type_ref_with_schema_field)
                })
                .collect()
        })
    }

    fn extract_type_ref_with_schema_field(type_value: &Value) -> Option<BrpTypeName> {
        type_value
            .get_field(SchemaField::Ref)
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix(SCHEMA_REF_PREFIX))
            .map(BrpTypeName::from)
    }

    /// Helper to get a schema field as an array
    fn get_schema_field_as_array(schema: &Value, field: SchemaField) -> Option<&Vec<Value>> {
        schema.get_field(field).and_then(Value::as_array)
    }
}
