//! Type state markers for the discovery engine
//!
//! These marker types ensure compile-time state validation for the discovery process.

/// Marker type for the `TypeDiscovery` state.
/// This state is responsible for creating the discovery context by calling
/// the registry and optional extras plugin.
pub struct TypeDiscovery;
