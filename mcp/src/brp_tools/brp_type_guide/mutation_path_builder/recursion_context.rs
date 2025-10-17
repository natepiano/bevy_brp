//! Recursion context for mutation path building
//!
//! This module implements an **immutable context propagation pattern** where each level of
//! type recursion gets a fresh `RecursionContext` built from its parent's state.
//!
//! ## Why Create New Contexts at Each Level?
//!
//! The mutation path building process is a **depth-first tree traversal** where:
//! - **Descending**: State accumulates (paths grow longer, variant chains extend)
//! - **Ascending**: Each level needs its exact context to build its mutation paths
//!
//! Creating new contexts at each level enables:
//! 1. **Proper Path Accumulation**: `""` → `".translation"` → `".translation.x"`
//! 2. **Variant Chain Building**: Enum variant requirements accumulate through nesting
//! 3. **Clean Ascent Processing**: Each level has its correct context when building paths
//! 4. **Path Action Propagation**: Skip decisions flow down the entire subtree
//!
//! ## Context Creation Flow
//!
//! ```text
//! Root: RecursionContext::new(root_path_kind, registry)
//!   ↓
//! Child 1: ctx.create_recursion_context(field_path_kind, Create)
//!   ↓
//! Child 2: ctx.create_recursion_context(nested_field_path_kind, Create)
//! ```
//!
//! Each child context inherits from parent:
//! - `registry`: Shared (cheap Arc clone)
//! - `mutation_path`: Parent path + new segment
//! - `variant_chain`: Parent chain (cloned, may be extended for enum children)
//! - `path_action`: Controls mutation path exposure (not recursion)
//!
//! ## Path Action: Exposure vs Recursion
//!
//! `PathAction` controls whether mutation paths are **exposed in the final result**,
//! NOT whether recursion happens:
//!
//! - **`PathAction::Create`**: Include this path and all descendant paths in results
//!   - Example: `Transform.translation.x` → exposes `""`, `".translation"`, `".translation.x"`
//!
//! - **`PathAction::Skip`**: Recurse for examples but DON'T expose descendant paths
//!   - Example: `HashMap<String, Transform>` → exposes only `""` with map example
//!   - Still recurses into `Transform` to get example values for map assembly
//!   - Does NOT expose Transform's `.rotation`, `.scale` paths (invalid for maps)
//!
//! **Skip Propagation**: Once a parent sets `Skip`, the entire subtree stays `Skip`.
//! This prevents deeper nested paths from leaking into results when their container
//! type (Map, Set) doesn't support nested mutations.
//!
//! Used by: `MapMutationBuilder`, `SetMutationBuilder` (collections that only support
//! root-level replacement, not element-level mutations).
//!
//! ## State Mutation
//!
//! While context creation is immutable, `variant_chain` CAN be mutated after creation
//! in `enum_path_builder.rs` (line 493) when processing enum children. This is the
//! only post-creation mutation and enables variant selection information to flow
//! through the type hierarchy.
//!
//! ## Example: Transform Struct
//!
//! ```text
//! Transform (root: "")
//!   ├─ translation: Vec3 (path: ".translation")
//!   │   ├─ x: f32 (path: ".translation.x")
//!   │   ├─ y: f32 (path: ".translation.y")
//!   │   └─ z: f32 (path: ".translation.z")
//!   ├─ rotation: Quat (path: ".rotation")
//!   └─ scale: Vec3 (path: ".scale")
//! ```
//!
//! Each node gets a `RecursionContext` with its exact position, enabling correct
//! mutation path generation during the ascent phase.
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use serde_json::Value;

use super::super::brp_type_name::BrpTypeName;
use super::super::constants::MAX_TYPE_RECURSION_DEPTH;
use super::super::type_knowledge::{BRP_TYPE_KNOWLEDGE, KnowledgeKey};
use super::new_types::{MutationPath, VariantName};
use super::path_kind::PathKind;
use super::types::PathAction;
use super::variant_signature::VariantSignature;
use super::{BuilderError, NotMutableReason};
use crate::error::Error;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Type-safe wrapper for recursion depth tracking
///
/// The `increment()` and `exceeds_limit()` methods are intentionally private to this module,
/// ensuring they can only be called from `RecursionContext::create_recursion_context()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecursionDepth(usize);

impl RecursionDepth {
    pub const ZERO: Self = Self(0);

    /// Increment depth - private to this module
    const fn increment(self) -> Self {
        Self(self.0 + 1)
    }

    /// Check if depth exceeds limit - private to this module
    const fn exceeds_limit(self) -> bool {
        self.0 > MAX_TYPE_RECURSION_DEPTH
    }
}

// Allow direct comparison with integers
impl Deref for RecursionDepth {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Context for mutation path building operations
///
/// This struct provides all the necessary context for building mutation paths,
/// including access to the registry, and enum variants.
#[derive(Debug)]
pub struct RecursionContext {
    /// The building context (root or field)
    pub path_kind:                PathKind,
    /// Reference to the type registry
    pub registry:                 Arc<HashMap<BrpTypeName, Value>>,
    /// the accumulated mutation path as we recurse through the type
    pub mutation_path:            MutationPath,
    /// Action to take regarding path creation (set by `MutationPathBuilder`)
    /// Design Review: Using enum instead of boolean for clarity and type safety
    pub path_action:              PathAction,
    /// Chain of variant constraints from root to current position
    /// Independent of `enum_context` - tracks ancestry for `PathRequirement` construction
    pub variant_chain:            Vec<VariantName>,
    /// Recursion depth tracking to prevent infinite loops
    pub depth:                    RecursionDepth,
    /// Parent enum variant signature (only set when processing enum variant children)
    /// The enum type is available via `path_kind.parent_type` - no need to store it redundantly
    pub parent_variant_signature: Option<VariantSignature>,
}

impl RecursionContext {
    /// Create a new mutation path context
    pub fn new(path_kind: PathKind, registry: Arc<HashMap<BrpTypeName, Value>>) -> Self {
        Self {
            path_kind,
            registry,
            mutation_path: MutationPath::from(""),
            path_action: PathAction::Create, // Default to creating paths
            variant_chain: Vec::new(),       // Start with empty variant chain
            depth: RecursionDepth::ZERO,     // Start at depth 0
            parent_variant_signature: None,  // NEW
        }
    }

    /// Get the type name being processed
    pub const fn type_name(&self) -> &BrpTypeName {
        self.path_kind.type_name()
    }

    /// Generate the path segment string for a `PathKind` (private to this module)
    fn path_kind_to_segment(path_kind: &PathKind) -> String {
        match path_kind {
            PathKind::RootValue { .. } => String::new(),
            PathKind::StructField { field_name, .. } => format!(".{field_name}"),
            PathKind::IndexedElement { index, .. } => format!(".{index}"),
            PathKind::ArrayElement { index, .. } => format!("[{index}]"),
        }
    }

    /// Require the schema to be present, returning an error if missing
    pub fn require_registry_schema(&self) -> crate::error::Result<&Value> {
        self.registry.get(self.type_name()).ok_or_else(|| {
            Error::General(format!(
                "Type {} not found in registry",
                self.type_name().display_name()
            ))
            .into()
        })
    }

    /// Create a new context for recursion
    ///
    /// Increments depth and automatically checks depth limit, returning an error if exceeded.
    /// This ensures depth checking cannot be accidentally skipped.
    ///
    /// The `increment()` and `exceeds_limit()` methods are private to this module, ensuring
    /// they can only be called here.
    pub fn create_recursion_context(
        &self,
        path_kind: PathKind,
        child_path_action: PathAction,
    ) -> std::result::Result<Self, BuilderError> {
        // Increment depth for child context
        let new_depth = self.depth.increment();

        // Check depth limit immediately after increment
        if new_depth.exceeds_limit() {
            tracing::debug!(
                "RECURSION LIMIT EXCEEDED: type={}, depth={}, path={}",
                path_kind.type_name(),
                *new_depth,
                self.mutation_path
            );
            return Err(BuilderError::NotMutable(
                NotMutableReason::RecursionLimitExceeded(path_kind.type_name().clone()),
            ));
        }

        let new_path_prefix = MutationPath::from(format!(
            "{}{}",
            self.mutation_path,
            Self::path_kind_to_segment(&path_kind)
        ));

        // Set path_action with proper propagation logic:
        // If parent is already Skip, stay Skip (regardless of what child wants)
        // Otherwise, use the child's preference
        let path_action = if matches!(self.path_action, PathAction::Skip) {
            PathAction::Skip // Once skipping, keep skipping for entire subtree
        } else {
            child_path_action
        };

        Ok(Self {
            path_kind,
            registry: Arc::clone(&self.registry),
            mutation_path: new_path_prefix,
            path_action,
            variant_chain: self.variant_chain.clone(), // Inherit parent's variant chain
            depth: new_depth,
            parent_variant_signature: self.parent_variant_signature.clone(), /* NEW: inherit from
                                                                              * parent */
        })
    }

    /// Extract all element types from Tuple/TupleStruct schema
    pub fn extract_tuple_element_types(schema: &Value) -> Option<Vec<BrpTypeName>> {
        Self::get_schema_field_as_array(schema, SchemaField::PrefixItems)
            .map(|items| items.iter().filter_map(Value::extract_field_type).collect())
    }

    /// Helper to get a schema field as an array
    fn get_schema_field_as_array(schema: &Value, field: SchemaField) -> Option<&Vec<Value>> {
        schema.get_field(field).and_then(Value::as_array)
    }

    /// Find mutation knowledge for this context
    ///
    /// This unified lookup method replaces the fragmented approach of separate lookup methods.
    /// It checks context-specific matches first, then falls back to exact type matches.
    ///
    /// Lookup order:
    /// 1. Struct field match (for field-specific values like `Camera3d.depth_texture_usages`) -
    ///    highest priority
    /// 2. Enum signature match (for variant element values like `AlphaMode2d::Mask(f32).0`)
    /// 3. Exact type match (handles most primitive and simple types) - fallback
    pub fn find_knowledge(
        &self,
    ) -> std::result::Result<
        Option<&'static super::super::type_knowledge::TypeKnowledge>,
        BuilderError,
    > {
        // Try context-specific matches based on PathKind FIRST - these have higher priority
        match &self.path_kind {
            PathKind::StructField {
                field_name,
                parent_type,
                ..
            } => {
                // Try struct field-specific knowledge first - this overrides generic type knowledge
                // Example: Camera3d.depth_texture_usages needs value 20, not generic u32 value
                let key = KnowledgeKey::struct_field(parent_type, field_name.as_str());
                tracing::debug!("Trying struct field match with key: {:?}", key);
                if let Some(knowledge) = BRP_TYPE_KNOWLEDGE.get(&key) {
                    tracing::debug!(
                        "Found struct field match for {}.{}: {:?}",
                        parent_type,
                        field_name,
                        knowledge.example()
                    );
                    return Ok(Some(knowledge));
                }
                tracing::debug!(
                    "No struct field match found for {}.{}, falling back to exact type match",
                    parent_type,
                    field_name
                );

                // Fall through to exact type match for struct fields without specific knowledge
            }
            PathKind::IndexedElement {
                index, parent_type, ..
            } => {
                // Check if we're a child of an enum variant signature
                if let Some(signature) = &self.parent_variant_signature {
                    match signature {
                        VariantSignature::Tuple(_types) => {
                            // Architectural guarantee: The index was created by enumerating
                            // this signature's types, so bounds are guaranteed valid

                            let key = KnowledgeKey::enum_variant_signature(
                                parent_type.clone(), // enum type from PathKind
                                signature.clone(),
                                *index,
                            );

                            if let Some(knowledge) = BRP_TYPE_KNOWLEDGE.get(&key) {
                                tracing::debug!(
                                    "Found enum signature knowledge for {parent_type}[{index}]: {:?}",
                                    knowledge.example()
                                );
                                return Ok(Some(knowledge));
                            }
                        }
                        VariantSignature::Struct(_) | VariantSignature::Unit => {
                            // ARCHITECTURAL INVARIANT VIOLATION
                            // IndexedElement should only occur with Tuple signatures
                            // create_paths_for_signature() creates StructField for Struct, nothing
                            // for Unit
                            use error_stack::Report;
                            return Err(BuilderError::SystemError(Report::new(
                                Error::InvalidState(format!(
                                    "IndexedElement path kind with {signature:?} variant signature for type {}. This indicates a bug in path generation logic.",
                                    parent_type.display_name()
                                )),
                            )));
                        }
                    }
                }
                // Fall through to exact type match
            }
            PathKind::RootValue { .. } | PathKind::ArrayElement { .. } => {
                // For these path kinds, only exact type matching applies
            }
        }

        // Try exact type match as fallback - this handles most cases
        let exact_key = KnowledgeKey::exact(self.type_name());
        Ok(BRP_TYPE_KNOWLEDGE.get(&exact_key))
    }

    /// Creates a `NoMutableChildren` error with this context's type name
    pub fn create_no_mutable_children_error(&self) -> NotMutableReason {
        NotMutableReason::NoMutableChildren {
            parent_type: self.type_name().clone(),
        }
    }
}
