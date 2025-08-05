//! Type state markers for the discovery engine
//!
//! These marker types ensure compile-time state validation for the discovery process.

use serde_json::Value;

use super::super::discovery_context::DiscoveryContext;
use crate::brp_tools::{BrpClientError, Port};
use crate::tool::BrpMethod;

/// Generic discovery engine with type state validation
///
/// The `State` parameter ensures that only valid operations can be called
/// for the current discovery phase.
pub struct DiscoveryEngine<State> {
    pub method:         BrpMethod,
    pub port:           Port,
    pub params:         Value,
    pub original_error: BrpClientError,
    #[allow(dead_code)] // Used in future phases when State contains data
    pub state: State,
}

/// Marker type for the `TypeDiscovery` state.
/// This state is responsible for creating the discovery context by calling
/// the registry and optional extras plugin.
pub struct TypeDiscovery;

/// State type for the `SerializationCheck` state.
/// This state holds a discovery context and is responsible for checking
/// if types have required serialization traits (Bevy 0.16 workaround).
pub struct SerializationCheck(pub DiscoveryContext);

/// State type for the `ExtrasDiscovery` state.
/// This state holds a discovery context and is responsible for building
/// corrections from extras data when no serialization issues are found.
#[allow(dead_code)]
pub struct ExtrasDiscovery(pub DiscoveryContext);

/// State type for the `PatternCorrection` state.
/// This state holds a discovery context and is responsible for applying
/// pattern-based corrections when extras discovery is unavailable or fails.
#[allow(dead_code)]
pub struct PatternCorrection(pub DiscoveryContext);
