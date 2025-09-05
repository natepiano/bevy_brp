//! Context types for mutation path building
//!
//! This module contains the context structures and related types used for building mutation paths,
//! including the main context struct and location enums.
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use super::super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, MutationKnowledge};
use super::super::response_types::{BrpTypeName, ReflectTrait, SchemaField};
use crate::brp_tools::brp_type_schema::constants::SCHEMA_REF_PREFIX;
use crate::brp_tools::brp_type_schema::mutation_knowledge::KnowledgeKey;
use crate::string_traits::JsonFieldAccess;

/// Context for building mutation paths - handles root vs field scenarios
/// necessary because Struct, specifically, allows us to recurse down a level
/// for complex types that have Struct fields
#[derive(Debug, Clone)]
pub enum PathLocation {
    /// Building paths for a root type (used in root mutations)
    Root { type_name: BrpTypeName },
    /// Building paths for a elements within a parent type
    Element {
        mutation_path: String,
        element_type:  BrpTypeName,
        parent_type:   BrpTypeName,
    },
}

impl PathLocation {
    /// Create a mutation path with its associated type and parent type
    pub fn mutation_path(
        mutation_path: &str,
        element_type: &BrpTypeName,
        parent_type: &BrpTypeName,
    ) -> Self {
        Self::Element {
            mutation_path: mutation_path.to_string(),
            element_type:  element_type.clone(),
            parent_type:   parent_type.clone(),
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
            Self::Element { element_type, .. } => element_type,
        }
    }
}

/// Context for mutation path building operations
///
/// This struct provides all the necessary context for building mutation paths,
/// including access to the registry, and enum variants.
#[derive(Debug)]
pub struct RecursionContext {
    /// The building context (root or field)
    pub location:         PathLocation,
    /// Reference to the type registry
    pub registry:         Arc<HashMap<BrpTypeName, Value>>,
    /// Path prefix for nested structures (e.g., ".translation" when building Vec3 fields)
    pub path_prefix:      String,
    /// Parent's mutation knowledge for extracting component examples
    pub parent_knowledge: Option<&'static MutationKnowledge>,
}

impl RecursionContext {
    /// Create a new mutation path context
    pub const fn new(location: PathLocation, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
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
        let mutation_path = accessor
            .trim_start_matches('.')
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();

        // Check if field has hardcoded knowledge to pass to children
        // First check struct_field, then exact type
        let field_knowledge = BRP_MUTATION_KNOWLEDGE
            .get(&KnowledgeKey::struct_field(parent_type, &mutation_path))
            .or_else(|| BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(field_type)));

        Self {
            location:         PathLocation::mutation_path(&mutation_path, field_type, parent_type),
            registry:         Arc::clone(&self.registry),
            path_prefix:      new_path_prefix,
            parent_knowledge: field_knowledge,
        }
    }

    /// Check if a value type has serialization support
    /// Used to determine if opaque Value types like String can be mutated
    pub fn value_type_has_serialization(&self, type_name: &BrpTypeName) -> bool {
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
