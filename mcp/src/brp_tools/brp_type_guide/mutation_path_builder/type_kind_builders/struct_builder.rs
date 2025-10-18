//! `PathBuilder` for `Struct` types
//!
//! Handles the most complex case - struct mutations with one-level recursion.
//! For field contexts, adds both the struct field itself and nested field paths.
//!
//! **Recursion**: YES - Structs recurse into each field to generate mutation paths
//! for nested structures (e.g., `Transform.translation.x`). Each field has a stable
//! name that can be used in paths, allowing deep mutation of nested structures.

use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use super::super::BuilderError;
use super::super::new_types::StructFieldName;
use super::super::path_kind::MutationPathDescriptor;
use super::super::path_kind::PathKind;
use super::super::recursion_context::RecursionContext;
use super::super::support;
use super::type_kind_builder::TypeKindBuilder;
use crate::error::Error;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;

pub struct StructMutationBuilder;

impl TypeKindBuilder for StructMutationBuilder {
    type Item = PathKind;
    type Iter<'a>
        = std::vec::IntoIter<PathKind>
    where
        Self: 'a;

    fn collect_children(&self, ctx: &RecursionContext) -> Result<Self::Iter<'_>> {
        // The new require_registry_schema() returns Result with standard error
        let schema = ctx.require_registry_schema()?;

        // Extract properties from schema - use proper schema methods
        // Note: Missing properties field is valid for empty structs (e.g., Camera2d)
        let Some(properties) = schema.get_properties() else {
            // No properties field means empty struct (marker struct)
            return Ok(vec![].into_iter());
        };

        // Empty properties map is also valid (empty struct/marker struct)
        if properties.is_empty() {
            return Ok(vec![].into_iter()); // Valid marker struct
        }

        // Convert each field into a PathKind
        let mut children = Vec::new();
        for (field_name, field_schema) in properties {
            // Extract field type or return error immediately - no fallback
            // Note: extract_field_type handles complex schemas with $ref
            let Some(type_name) = field_schema.extract_field_type() else {
                return Err(Error::SchemaProcessing {
                    message:   format!(
                        "Failed to extract type for field '{}' in struct '{}'",
                        field_name,
                        ctx.type_name()
                    ),
                    type_name: Some(ctx.type_name().to_string()),
                    operation: Some("extract_field_type".to_string()),
                    details:   Some(format!("Field: {field_name}")),
                }
                .into());
            };

            // Create PathKind for this field
            let path_kind = PathKind::StructField {
                field_name: StructFieldName::from(field_name.clone()),
                type_name,
                parent_type: ctx.type_name().clone(),
            };

            children.push(path_kind);
        }

        Ok(children.into_iter())
    }

    fn assemble_from_children(
        &self,
        _ctx: &RecursionContext,
        children: HashMap<MutationPathDescriptor, Value>,
    ) -> std::result::Result<Value, BuilderError> {
        if children.is_empty() {
            // Valid case: empty struct with no fields (e.g., marker structs)
            return Ok(json!({}));
        }

        // Use shared function to build struct object from child examples
        let struct_obj = support::assemble_struct_from_children(&children);

        Ok(Value::Object(struct_obj))
    }
}
