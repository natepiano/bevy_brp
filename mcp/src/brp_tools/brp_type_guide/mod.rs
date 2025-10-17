//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide accurate format discovery for BRP operations.

mod brp_type_name;
mod builder;
mod constants;
mod mutation_path_builder;
mod response_types;
mod tool_all_types;
mod tool_type_guide;
mod type_kind;
mod type_knowledge;

// Re-export public API
// Internal use for format discovery
pub use brp_type_name::BrpTypeName;
pub use tool_all_types::{AllTypeGuidesParams, BrpAllTypeGuides};
pub use tool_type_guide::{BrpTypeGuide, TypeGuideEngine, TypeGuideParams};
