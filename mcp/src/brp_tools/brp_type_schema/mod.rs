//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide format discovery equivalent to `brp_extras_discover_format`.

pub mod hardcoded_formats;
pub mod registry_cache;
mod result_types;
mod tool;
pub mod type_discovery;
pub mod types;
mod wrapper_types;

// Re-export public API
pub use tool::{BrpTypeSchema, TypeSchemaParams, TypeSchemaResult};
pub use types::{BrpTypeName, TypeKind};
