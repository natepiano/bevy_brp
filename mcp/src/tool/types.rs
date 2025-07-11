use std::pin::Pin;

use rmcp::Error as McpError;

use crate::service::HandlerContext;

/// Metadata about a BRP request for response formatting
#[derive(Debug, Clone)]
pub struct BrpToolCallInfo {
    pub tool_name: String,
    pub port:      u16,
}

impl BrpToolCallInfo {
    pub fn new(tool_name: &str, port: u16) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            port,
        }
    }
}

/// Type alias for the response from local handlers
///
/// Breaking down the type:
/// - `Pin<Box<...>>`: Heap-allocated Future that won't move in memory
/// - `dyn Future`: Async function that can be awaited
/// - `Output = Result<...>`: Can fail with `McpError`
/// - `Box<dyn HandlerResult>`: Type-erased result implementing `HandlerResult` trait
/// - `+ Send + 'a`: Can be sent between threads, lifetime tied to handler
pub type HandlerResponse<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn HandlerResult>, McpError>> + Send + 'a>>;

/// Result type that all local handlers must return
pub trait HandlerResult: Send + Sync {
    /// Serialize this result to a JSON value (required due to dyn compatibility)
    fn to_json(&self) -> serde_json::Value;
}

/// Trait for local handlers using function pointer approach
pub trait LocalHandler: Send + Sync {
    /// Handle the request and return a typed result
    fn handle(&self, ctx: &HandlerContext) -> HandlerResponse<'_>;
}

/// Type of handler for the tool
#[derive(Clone)]
pub enum HandlerType {
    /// BRP handler - calls a BRP method
    Brp {
        /// BRP method to call (e.g., "bevy/destroy")
        method: &'static str,
    },

    /// Local handler using function pointer approach
    Local {
        /// Handler trait object
        handler: Box<dyn LocalHandler>,
    },
}
