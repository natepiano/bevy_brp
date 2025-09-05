//! Category of type for quick identification and processing
//!
//! This enum represents the actual type kinds returned by Bevy's type registry.
//! These correspond to the "kind" field in registry schema responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

use super::builders::{
    ArrayMutationBuilder, DefaultMutationBuilder, EnumMutationBuilder, ListMutationBuilder,
    MapMutationBuilder, StructMutationBuilder, TupleMutationBuilder,
};
use super::types::{MutationPathBuilder, MutationPathContext};
use crate::brp_tools::brp_type_schema::constants::RecursionDepth;
use super::types::{MutationPathInternal, MutationPathKind, MutationStatus};
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, SchemaField};
use crate::brp_tools::brp_type_schema::type_info::MutationSupport;
use crate::error::Result;
use crate::string_traits::JsonFieldAccess;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRefStr, EnumString)]
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
pub enum TypeKind {
    /// Array type
    Array,
    /// Enum type
    Enum,
    /// List type
    List,
    /// Map type (`HashMap`, `BTreeMap`, etc.)
    Map,
    /// Regular struct type
    Struct,
    /// Tuple type
    Tuple,
    /// Tuple struct type
    TupleStruct,
    /// Value type (primitive types like i32, f32, bool, String)
    Value,
}

impl TypeKind {
    /// Extract `TypeKind` from a registry schema with fallback to `Value`
    pub fn from_schema(schema: &Value, type_name: &BrpTypeName) -> Self {
        schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Type '{}' has missing or invalid 'kind' field in registry schema, defaulting to TypeKind::Value",
                    type_name
                );
                Self::Value
            })
    }

    /// Build `NotMutatable` path from `MutationSupport` error details
    fn build_not_mutatable_path_from_support(
        ctx: &super::types::MutationPathContext,
        support: &crate::brp_tools::brp_type_schema::type_info::MutationSupport,
        directive_suffix: &str,
    ) -> MutationPathInternal {
        use serde_json::json;

        use super::types::RootOrField;
        // MutationPathInternal, MutationPathKind, MutationStatus already imported above

        match &ctx.location {
            RootOrField::Root { type_name } => MutationPathInternal {
                path:            String::new(),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This type cannot be mutated{directive_suffix} - see error message for details")
                }),
                enum_variants:   None,
                type_name:       type_name.clone(),
                path_kind:       MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(support),
            },
            RootOrField::Field {
                field_name,
                field_type,
                parent_type,
            } => MutationPathInternal {
                path:            format!(".{field_name}"),
                example:         json!({
                    "NotMutatable": format!("{support}"),
                    "agent_directive": format!("This field cannot be mutated{directive_suffix} - see error message for details")
                }),
                enum_variants:   None,
                type_name:       field_type.clone(),
                path_kind:       MutationPathKind::StructField {
                    field_name:  field_name.clone(),
                    parent_type: parent_type.clone(),
                },
                mutation_status: MutationStatus::NotMutatable,
                error_reason:    Option::<String>::from(support),
            },
        }
    }
}

// Implementation of MutationPathBuilder for TypeKind

impl MutationPathBuilder for TypeKind {
    fn build_paths(
        &self,
        ctx: &MutationPathContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Check recursion limit first
        if depth.exceeds_limit() {
            let recursion_limit_path = Self::build_not_mutatable_path_from_support(
                ctx,
                &MutationSupport::RecursionLimitExceeded(ctx.type_name().clone()),
                "",
            );
            return Ok(vec![recursion_limit_path]);
        }

        // Only increment depth for container types that recurse into nested structures
        let builder_depth = match self {
            // Container types that recurse - increment depth
            Self::Struct
            | Self::Tuple
            | Self::TupleStruct
            | Self::Array
            | Self::List
            | Self::Map
            | Self::Enum => depth.increment(),
            // Leaf types and wrappers - preserve current depth
            Self::Value => depth,
        };

        match self {
            Self::Struct => StructMutationBuilder.build_paths(ctx, builder_depth),
            Self::Tuple | Self::TupleStruct => TupleMutationBuilder.build_paths(ctx, builder_depth),
            Self::Array => ArrayMutationBuilder.build_paths(ctx, builder_depth),
            Self::List => ListMutationBuilder.build_paths(ctx, builder_depth),
            Self::Map => MapMutationBuilder.build_paths(ctx, builder_depth),
            Self::Enum => EnumMutationBuilder.build_paths(ctx, builder_depth),
            Self::Value => {
                // Check serialization inline, no recursion needed
                if ctx.value_type_has_serialization(ctx.type_name()) {
                    DefaultMutationBuilder.build_paths(ctx, builder_depth)
                } else {
                    let not_mutatable_path = Self::build_not_mutatable_path_from_support(
                        ctx,
                        &MutationSupport::MissingSerializationTraits(ctx.type_name().clone()),
                        "",
                    );
                    Ok(vec![not_mutatable_path])
                }
            }
        }
    }
}
