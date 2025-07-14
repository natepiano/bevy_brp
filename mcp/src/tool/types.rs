use std::pin::Pin;

use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::service::{HandlerContext, LocalContext};

/// Trait for individual tool handler implementations
/// `#[async_trait]` allows us to use `ToolHandler` in `Box<dyn ToolHandler>` situations
#[async_trait]
pub trait ToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError>;
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
