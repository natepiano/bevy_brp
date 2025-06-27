//! Format discovery engine for BRP component introspection
//!
//! This module provides the core functionality for discovering component formats
//! through Bevy's reflection system and generating format information for BRP operations.
//!
//! The module is organized into focused sub-modules:
//! - `registry`: Type registry access utilities
//! - `error`: Unified error handling and debug context
//! - `examples`: Primitive and type example generation
//! - `types`: `TypeInfo` processing and analysis
//! - `spawn`: Spawn format generation logic
//! - `mutation`: Mutation info generation logic
//! - `core`: Main discovery orchestration
//! - `handler`: Public API and request handling

// Internal modules
mod core;
mod error;
mod examples;
mod handler;
mod mutation;
mod registry;
mod spawn;
mod types;

// Re-export public API to maintain compatibility
pub use handler::{
    discover_component_format_simple as discover_component_format,
    discover_multiple_formats_public as discover_multiple_formats,
    get_common_component_types_public as get_common_component_types, handler,
};

// Re-export core discovery functions for advanced users
// (None currently needed publicly)

// Internal-only re-exports for the modules to use each other

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;
    use crate::discovery::error::{DebugContext, DiscoveryError};

    #[test]
    fn test_common_component_types() {
        let types = get_common_component_types();
        assert!(!types.is_empty());
        assert!(types.contains(&"bevy_transform::components::transform::Transform".to_string()));
    }

    #[test]
    fn test_debug_context() {
        let mut ctx = DebugContext::new();
        ctx.push("test message");
        assert_eq!(ctx.messages.len(), 1);
        assert_eq!(ctx.messages[0], "test message");
    }

    #[test]
    fn test_discovery_error() {
        let error = DiscoveryError::unsupported_type("test type");
        let json_error = error.to_json_error();
        assert!(json_error.contains_key("reason"));
        assert!(json_error.contains_key("details"));
    }
}
