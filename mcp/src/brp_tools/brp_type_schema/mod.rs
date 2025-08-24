//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide format discovery equivalent to `brp_extras_discover_format`.

mod format_knowledge;
mod mutation_path_builders;
mod response_types;
mod tool;
mod type_info;
mod wrapper_types;

// Re-export public API
pub use format_knowledge::BRP_FORMAT_KNOWLEDGE;
pub use response_types::{BrpTypeName, EnumFieldInfo, EnumVariantKind, MutationPath, TypeKind};
pub use tool::{TypeSchema, TypeSchemaEngine, TypeSchemaParams};
pub use type_info::TypeInfo;
