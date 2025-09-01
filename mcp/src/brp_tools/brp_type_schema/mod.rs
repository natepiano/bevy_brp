//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide accurate format discovery for BRP operations.

mod constants;
mod format_knowledge;
mod mutation_path_builders;
mod response_types;
mod tool;
mod type_info;
mod wrapper_types;

// Re-export public API
// Internal use for format discovery
pub use tool::{TypeSchema, TypeSchemaEngine, TypeSchemaParams};
