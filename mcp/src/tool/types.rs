use std::pin::Pin;
use std::sync::Arc;

use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::service::{HandlerContext, LocalContext};

/// Trait for individual tool handler implementations
pub trait ToolHandlerImpl {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError>;
}

/// Enum for tool handlers in the flat structure
pub enum ToolHandler {
    Local(crate::tool::LocalToolHandler),
    Brp(crate::tool::BrpToolHandler),
}

impl ToolHandler {
    pub async fn call_tool(self) -> Result<CallToolResult, McpError> {
        match self {
            Self::Local(handler) => Box::new(handler).call_tool().await,
            Self::Brp(handler) => Box::new(handler).call_tool().await,
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
pub trait LocalToolFunction: Send + Sync {
    /// Handle the request and return a typed result
    fn call(&self, ctx: &HandlerContext<LocalContext>) -> HandlerResponse<'_>;
}

/// Type of handler for the tool
#[derive(Clone)]
pub enum HandlerType {
    /// BRP handler - calls a BRP method
    Brp {
        /// BRP method to call (e.g., "bevy/destroy")
        method: &'static str,
    },

    /// BRP execute handler - calls a dynamic BRP method determined at runtime
    BrpExecute,

    /// Local handler using function pointer approach
    Local {
        /// Handler trait object
        handler: Arc<dyn LocalToolFunction>,
    },
}
