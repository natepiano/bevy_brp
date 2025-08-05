//! Generic `DiscoveryEngine` struct and constructor
//!
//! This module defines the generic `DiscoveryEngine<State>` struct that supports
//! the type state pattern for compile-time validation of discovery phases.

use serde_json::Value;

use super::types::TypeDiscovery;
use crate::brp_tools::{BrpClientError, Port};
use crate::error::Result;
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

impl DiscoveryEngine<TypeDiscovery> {
    /// Create a new format discovery engine for a specific method and port
    ///
    /// Returns an error if the parameters are invalid for format discovery
    /// (e.g., None when format discovery requires parameters, or error is not a format error)
    #[allow(clippy::unnecessary_wraps)] // Keeping Result for future validation
    pub fn new(
        method: BrpMethod,
        port: Port,
        params: Option<Value>,
        original_error: BrpClientError,
    ) -> Result<Self> {
        // Extract parameters, handling the None case
        let params = params.unwrap_or_default();

        // TODO: In Phase 3, we'll add validation logic here
        // For now, just create the engine in TypeDiscovery state
        Ok(Self {
            method,
            port,
            params,
            original_error,
            state: TypeDiscovery,
        })
    }
}
