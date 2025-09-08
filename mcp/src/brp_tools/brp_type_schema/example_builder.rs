//! Temporary example builder to break circular dependencies
//!
//! This is a temporary structure used during the migration to unify example generation
//! through path builders. It serves as an intermediate step to break the circular
//! dependency between `TypeInfo::build_type_example` and builder methods.
//!
//! **This file will be deleted** after the migration is complete.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::constants::RecursionDepth;
use super::mutation_path_builder::{
    ArrayMutationBuilder, EnumMutationBuilder, KnowledgeKey, ListMutationBuilder,
    MapMutationBuilder, SetMutationBuilder, StructMutationBuilder, TupleMutationBuilder, TypeKind,
};
use super::response_types::BrpTypeName;

/// Temporary builder to break circular dependencies during migration
///
/// This struct provides a single method that builders can call instead of
/// `TypeInfo::build_type_example`, which allows us to break the circular
/// dependency during the migration process.
///
/// **Migration Status**: This is temporary scaffolding that will be removed
/// once all example generation is moved into path builders.
pub struct ExampleBuilder;

impl ExampleBuilder {
    /// Builders call this instead of `TypeInfo::build_type_example`
    ///
    /// This method contains the dispatch logic moved from `TypeInfo` to break
    /// the circular dependency between `TypeInfo` and builders.
    ///
    /// # Arguments
    /// * `type_name` - The fully-qualified type name to build an example for
    /// * `registry` - The type registry containing schema information
    /// * `depth` - Current recursion depth for stack overflow prevention
    ///
    /// # Returns
    /// A JSON `Value` representing an example of the specified type
    pub fn build_example(
        type_name: &BrpTypeName,
        registry: &HashMap<BrpTypeName, Value>,
        depth: RecursionDepth,
    ) -> Value {
        // Prevent stack overflow from deep recursion
        if depth.exceeds_limit() {
            return json!(null);
        }

        // Use enum dispatch for format knowledge lookup
        if let Some(example) = KnowledgeKey::find_example_for_type(type_name) {
            return example;
        }

        // Check if we have the type in the registry
        let Some(field_schema) = registry.get(type_name) else {
            return json!(null);
        };

        let field_kind = TypeKind::from_schema(field_schema, type_name);
        match field_kind {
            TypeKind::Enum => EnumMutationBuilder::build_enum_spawn_example(
                field_schema,
                registry,
                Some(type_name),
                depth.increment(),
            ),
            TypeKind::Array => ArrayMutationBuilder::build_array_example_static(
                type_name,
                field_schema,
                registry,
                depth,
            ),
            TypeKind::Tuple | TypeKind::TupleStruct => {
                TupleMutationBuilder::build_tuple_example_static(field_schema, registry, depth)
            }
            TypeKind::Struct => {
                StructMutationBuilder::build_struct_example_static(field_schema, registry, depth)
            }
            TypeKind::List => {
                ListMutationBuilder::build_list_example_static(field_schema, registry, depth)
            }
            TypeKind::Set => {
                SetMutationBuilder::build_set_example_static(field_schema, registry, depth)
            }
            TypeKind::Map => {
                MapMutationBuilder::build_map_example_static(field_schema, registry, depth)
            }
            TypeKind::Value => json!(null),
        }
    }
}
