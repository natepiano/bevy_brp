//! Shared context for format discovery phases

use serde_json::Value;
use tracing::{debug, trace};

/// Shared context that flows through all format discovery phases
#[derive(Debug, Clone)]
pub struct DiscoveryContext {
    /// The BRP method being executed
    pub method: String,

    /// The original parameters passed to the method
    pub original_params: Option<Value>,

    /// The port to connect to (optional)
    pub port: Option<u16>,

    /// The initial error that triggered discovery (if any)
    pub initial_error: Option<crate::brp_tools::support::brp_client::BrpError>,
}

impl DiscoveryContext {
    /// Create a new discovery context
    pub fn new(method: impl Into<String>, params: Option<Value>, port: Option<u16>) -> Self {
        let method_name = method.into();
        debug!("Creating discovery context for method: {}", method_name);

        Self {
            method: method_name,
            original_params: params,
            port,
            initial_error: None,
        }
    }

    /// Add a debug message using tracing
    pub fn add_debug(message: impl Into<String>) {
        trace!("Discovery: {}", message.into());
    }

    /// Set the initial error
    pub fn set_error(&mut self, error: crate::brp_tools::support::brp_client::BrpError) {
        self.initial_error = Some(error);
    }
}
