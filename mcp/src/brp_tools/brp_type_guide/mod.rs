//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide accurate format discovery for BRP operations.

mod all_types_tool;
mod brp_type_name;
mod constants;
mod mutation_path_builder;
mod response_types;
mod tool;
mod type_guide;
mod type_kind;

// Re-export public API
// Internal use for format discovery
pub use all_types_tool::{AllTypeGuidesParams, BrpAllTypeGuides};
pub use brp_type_name::BrpTypeName;
pub use mutation_path_builder::NotMutableReason;
pub use tool::{BrpTypeGuide, TypeGuideEngine, TypeGuideParams};
