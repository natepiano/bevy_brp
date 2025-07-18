use std::future::Future;
use std::pin::Pin;

use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use super::HandlerFn;
use super::handler_context::{HandlerContext, HasMethod, HasPort, NoMethod, NoPort};

/// Unified tool handler that works with any `HandlerFn` variant
pub struct ToolHandler {
    handler: HandlerFn,
    context: ToolContext,
}

impl ToolHandler {
    pub const fn new(handler: HandlerFn, context: ToolContext) -> Self {
        Self { handler, context }
    }
}

impl ToolHandler {
    pub async fn call_tool(self) -> Result<CallToolResult, McpError> {
        self.handler.call_handler(&self.context).await
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

/// Type alias for BRP handler responses
/// Result type that all local handlers must return
pub trait HandlerResult: Send + Sync {
    /// Serialize this result to a JSON value (required due to dyn compatibility)
    fn to_json(&self) -> serde_json::Value;
}

/// Trait for local handlers using function pointer approach
pub trait LocalToolFn: Send + Sync {
    /// Handle the request and return a typed result
    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<'_>;
}

/// Trait for local handlers with port - no separate port parameter needed
pub trait LocalToolFnWithPort: Send + Sync {
    /// Handle the request and return a typed result - handlers call `ctx.port()` directly
    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<'_>;
}

/// Trait for BRP handlers that return `HandlerResponse` (unified with local handlers)
pub trait BrpToolFn: Send + Sync {
    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<'_>;
}

/// Unified context that wraps Local, `LocalWithPort`, and BRP handler contexts
#[derive(Clone)]
pub enum ToolContext {
    Local(HandlerContext<NoPort, NoMethod>),          // For Local
    LocalWithPort(HandlerContext<HasPort, NoMethod>), // For LocalWithPort
    Brp(HandlerContext<HasPort, HasMethod>),          // For Brp
}

/// BRP method source specification for tool handlers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrpMethodSource {
    /// Static method name known at compile time
    Static(&'static str),
    /// Dynamic method name extracted from request parameters
    Dynamic,
}
