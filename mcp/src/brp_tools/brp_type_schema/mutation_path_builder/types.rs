//! Mutation path builders for different type kinds
//!
//! This module implements the TYPE-SYSTEM-002 refactor: Replace conditional chains
//! in mutation path building with type-directed dispatch using the `MutationPathBuilder` trait.
//!
//! The key insight is that different `TypeKind` variants need different logic for building
//! mutation paths, but this should be cleanly separated from the field-level logic.
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use super::super::constants::RecursionDepth;
use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, MutationKnowledge};
use super::super::response_types::{self, BrpTypeName, MutationPathInternal, SchemaField};
use crate::brp_tools::brp_type_schema::constants::SCHEMA_REF_PREFIX;
use crate::brp_tools::brp_type_schema::mutation_knowledge::KnowledgeKey;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

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
        use response_types::ReflectTrait;

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
