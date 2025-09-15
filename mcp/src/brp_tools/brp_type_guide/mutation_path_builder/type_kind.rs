//! Category of type for quick identification and processing
//!
//! This enum represents the actual type kinds returned by Bevy's type registry.
//! These correspond to the "kind" field in registry schema responses.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use strum::{AsRefStr, Display, EnumString};

use super::MutationPathBuilder;
use super::builders::{
    ArrayMutationBuilder, EnumMutationBuilder, ListMutationBuilder, MapMutationBuilder,
    SetMutationBuilder, StructMutationBuilder, TupleMutationBuilder, ValueMutationBuilder,
};
use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeGuidance, KnowledgeKey};
use super::not_mutable_reason::NotMutableReason;
use super::protocol_enforcer::ProtocolEnforcer;
use super::recursion_context::RecursionContext;
use super::types::{MutationPathInternal, MutationStatus};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::brp_tools::brp_type_guide::response_types::BrpTypeName;
use crate::error::Result;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

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
    /// Set type (`HashSet`, `BTreeSet`, etc.)
    Set,
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

    /// Get the appropriate builder instance for this type kind
    pub fn builder(&self) -> Box<dyn MutationPathBuilder> {
        let base_builder: Box<dyn MutationPathBuilder> = match self {
            Self::Struct => Box::new(StructMutationBuilder),
            Self::Tuple | Self::TupleStruct => Box::new(TupleMutationBuilder),
            Self::Array => Box::new(ArrayMutationBuilder),
            Self::List => Box::new(ListMutationBuilder),
            Self::Map => Box::new(MapMutationBuilder),
            Self::Set => Box::new(SetMutationBuilder),
            Self::Enum => Box::new(EnumMutationBuilder),
            Self::Value => Box::new(ValueMutationBuilder),
        };

        // Wrap with protocol enforcer if migrated
        if base_builder.is_migrated() {
            Box::new(ProtocolEnforcer::new(base_builder))
        } else {
            base_builder
        }
    }

    /// Build a mutation path for types with `TreatAsValue` knowledge
    /// that come from our hard coded knowledge
    fn build_treat_as_value_path(ctx: &RecursionContext) -> Option<MutationPathInternal> {
        if let Some(knowledge) =
            BRP_MUTATION_KNOWLEDGE.get(&KnowledgeKey::exact(ctx.type_name().to_string()))
            && let KnowledgeGuidance::TreatAsRootValue { simplified_type } = knowledge.guidance()
        {
            // Build a single root mutation path for types that should be treated as values
            let example = knowledge.example().clone();

            let path = MutationPathInternal {
                path: ctx.mutation_path.clone(),
                example,
                type_name: BrpTypeName::from(simplified_type),
                path_kind: ctx.path_kind.clone(),
                mutation_status: MutationStatus::Mutable,
                mutation_status_reason: None,
            };

            return Some(path);
        }

        None
    }

    /// Build `NotMutable` path from `MutationSupport` error details
    fn build_not_mutable_path_from_support(
        ctx: &RecursionContext,
        support: &NotMutableReason,
        directive_suffix: &str,
    ) -> MutationPathInternal {
        MutationPathInternal {
            path: ctx.mutation_path.clone(),
            example: json!({
                "NotMutable": format!("{support}"),
                "agent_directive": format!("This type cannot be mutated{directive_suffix} - see error message for details")
            }),
            type_name: ctx.type_name().clone(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::NotMutable,
            mutation_status_reason: Option::<Value>::from(support),
        }
    }
}

// Implementation of MutationPathBuilder for TypeKind

impl MutationPathBuilder for TypeKind {
    fn build_paths(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        // Check recursion limit first
        if depth.exceeds_limit() {
            let recursion_limit_path = Self::build_not_mutable_path_from_support(
                ctx,
                &NotMutableReason::RecursionLimitExceeded(ctx.type_name().clone()),
                "",
            );
            return Ok(vec![recursion_limit_path]);
        }

        // Check if this type has TreatAsValue knowledge
        // which bypasses any further recursion to provide a simplified Value example
        if let Some(mutation_path_internal) = Self::build_treat_as_value_path(ctx) {
            return Ok(vec![mutation_path_internal]);
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
            | Self::Set
            | Self::Enum => depth.increment(),
            // Leaf types and wrappers - preserve current depth
            Self::Value => depth,
        };

        match self {
            Self::Struct => self.builder().build_paths(ctx, builder_depth),
            Self::Tuple | Self::TupleStruct => {
                tracing::debug!(
                    "TypeKind: Dispatching to unmigrated TupleMutationBuilder for type '{}'",
                    ctx.type_name()
                );
                let result = TupleMutationBuilder.build_paths(ctx, builder_depth);
                tracing::debug!(
                    "TypeKind: TupleMutationBuilder for '{}' returned {} paths",
                    ctx.type_name(),
                    result.as_ref().map(|paths| paths.len()).unwrap_or(0)
                );
                result
            }
            Self::Array => self.builder().build_paths(ctx, builder_depth),
            Self::List => self.builder().build_paths(ctx, builder_depth),
            Self::Map | Self::Set => self.builder().build_paths(ctx, builder_depth),
            Self::Enum => EnumMutationBuilder.build_paths(ctx, builder_depth),
            Self::Value => {
                // Check serialization inline, no recursion needed
                if ctx.value_type_has_serialization(ctx.type_name()) {
                    // Use self.builder() for migrated ValueMutationBuilder to get
                    // ProtocolEnforcer wrapper
                    self.builder().build_paths(ctx, builder_depth)
                } else {
                    let not_mutable_path = Self::build_not_mutable_path_from_support(
                        ctx,
                        &NotMutableReason::MissingSerializationTraits(ctx.type_name().clone()),
                        "",
                    );
                    Ok(vec![not_mutable_path])
                }
            }
        }
    }
}
