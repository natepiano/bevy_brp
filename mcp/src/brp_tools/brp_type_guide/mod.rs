//! Local registry-based type schema discovery
//!
//! This module provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide accurate format discovery for BRP operations.

mod brp_type_name;
mod constants;
mod guide;
mod mutation_path_builder;
mod response_types;
mod struct_field_name;
mod tool_all_types;
mod tool_type_guide;
mod type_kind;
mod type_knowledge;
mod variant_signature;

// Re-export public API
// Internal use for format discovery
pub use brp_type_name::BrpTypeName;
pub use tool_all_types::AllTypeGuidesParams;
pub use tool_all_types::BrpAllTypeGuides;
pub use tool_type_guide::BrpTypeGuide;
pub use tool_type_guide::TypeGuideParams;
use crate::error::Result;

/// Visibility facade for type-guide generation across `brp_tools` submodules.
///
/// Sibling modules should depend on this parent-level entry point instead of the
/// `TypeGuideEngine` implementation type in `tool_type_guide.rs`.
pub(super) async fn generate_type_guide_response(
    port: super::Port,
    requested_types: &[String],
) -> Result<response_types::TypeGuideResponse> {
    tool_type_guide::generate_type_guide_response(port, requested_types).await
}
