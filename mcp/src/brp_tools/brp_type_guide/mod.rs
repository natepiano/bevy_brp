//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide accurate format discovery for BRP operations.

mod all_types_tool;
mod constants;
mod example_builder;
mod mutation_path_builder;
mod response_types;
mod tool;
mod type_guide;

// Re-export public API
// Internal use for format discovery
pub use all_types_tool::{AllTypeGuidesParams, BrpAllTypeGuides};
pub use response_types::BrpTypeName;
pub use tool::{BrpTypeGuide, TypeGuideEngine, TypeGuideParams};
