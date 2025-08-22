//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide format discovery equivalent to `brp_extras_discover_format`.

mod engine;
mod hardcoded_formats;
mod registry_cache;
mod result_types;
mod schema_processor;
mod tool;
mod type_discovery;
mod types;
mod wrapper_types;

// Re-export public API
pub use tool::{BrpTypeSchema, TypeSchemaParams};
pub use types::BrpTypeName;
