//! Generic `DiscoveryEngine` struct and constructor
//!
//! This module defines the generic `DiscoveryEngine<State>` struct that supports
//! the type state pattern for compile-time validation of discovery phases.

use serde_json::Value;

use super::types::{DiscoveryEngine, TypeDiscovery};
use crate::brp_tools::{BrpClientError, Port};
use crate::error::{Error, Result};
use crate::tool::BrpMethod;

impl DiscoveryEngine<TypeDiscovery> {
    /// Create a new format discovery engine for a specific method and port
    ///
    /// Returns an error if the parameters are invalid for format discovery
    /// (e.g., None when format discovery requires parameters, or error is not a format error)
    pub fn new(
        method: BrpMethod,
        port: Port,
        params: Option<Value>,
        original_error: BrpClientError,
    ) -> Result<Self> {
        // Check if we can recover from this error type
        // Format discovery can only work with specific error types
        if !original_error.is_format_error() {
            return Err(Error::InvalidArgument(
                "Format discovery can only be used with format errors".to_string(),
            )
            .into());
        }

        // Validate that parameters exist for format discovery
        // Format discovery requires parameters to extract type information
        let params = params.ok_or_else(|| {
            Error::InvalidArgument(
                "Format discovery requires parameters to extract type information".to_string(),
            )
        })?;

        Ok(Self {
            method,
            port,
            params,
            original_error,
            state: TypeDiscovery,
        })
    }
}
