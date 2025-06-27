//! Shared context for format discovery phases

use serde_json::Value;

/// Shared context that flows through all format discovery phases
#[derive(Debug, Clone)]
pub struct DiscoveryContext {
    /// The BRP method being executed
    pub method: String,

    /// The original parameters passed to the method
    pub original_params: Option<Value>,

    /// The port to connect to (optional)
    pub port: Option<u16>,

    /// Accumulated debug information
    pub debug_info: Vec<String>,

    /// The initial error that triggered discovery (if any)
    pub initial_error: Option<crate::brp_tools::support::brp_client::BrpError>,
}

impl DiscoveryContext {
    /// Create a new discovery context
    pub fn new(
        method: impl Into<String>,
        params: Option<Value>,
        port: Option<u16>,
        initial_debug_info: Vec<String>,
    ) -> Self {
        Self {
            method: method.into(),
            original_params: params,
            port,
            debug_info: initial_debug_info,
            initial_error: None,
        }
    }

    /// Add a debug message
    pub fn add_debug(&mut self, message: impl Into<String>) {
        self.debug_info.push(message.into());
    }

    /// Set the initial error
    pub fn set_error(&mut self, error: crate::brp_tools::support::brp_client::BrpError) {
        self.initial_error = Some(error);
    }
}
