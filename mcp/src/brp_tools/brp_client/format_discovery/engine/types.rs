//! Type state markers for the discovery engine
//!
//! These marker types ensure compile-time state validation for the discovery process.

use std::ops::Deref;

use serde_json::Value;

use super::discovery_context::DiscoveryContext;
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

// Implement Deref for states that wrap DiscoveryContext
impl Deref for SerializationCheck {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for ExtrasDiscovery {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for PatternCorrection {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Provide methods to extract the inner DiscoveryContext
impl SerializationCheck {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

impl ExtrasDiscovery {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

/// A newtype wrapper for BRP type names used as `HashMap` keys
///
/// This type provides documentation and type safety for strings that represent
/// fully-qualified Rust type names (e.g., "`bevy_transform::components::transform::Transform`")
/// when used as keys in type information maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BrpTypeName(String);

impl BrpTypeName {
    /// Get the underlying string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrpTypeName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrpTypeName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BrpTypeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
