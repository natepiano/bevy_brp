use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::service::{BrpContext, HandlerContext, LocalContext};

/// Trait for individual tool handler implementations
/// `#[async_trait]` allows us to use `ToolHandler` in `Box<dyn ToolHandler>` situations
#[async_trait]
pub trait ToolHandlerTrait {
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

/// Type alias for BRP handler responses
pub type BrpHandlerResponse<'a> =
    Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + 'a>>;

/// Result type that all local handlers must return
pub trait HandlerResult: Send + Sync {
    /// Serialize this result to a JSON value (required due to dyn compatibility)
    fn to_json(&self) -> serde_json::Value;
}

/// Trait for local handlers using function pointer approach
pub trait LocalToolFn: Send + Sync {
    /// Handle the request and return a typed result
    fn call(&self, ctx: &HandlerContext<LocalContext>) -> HandlerResponse<'_>;
}

/// Trait for local handlers using function pointer approach
pub trait LocalToolFnWithPort: Send + Sync {
    /// Handle the request and return a typed result
    fn call(&self, ctx: &HandlerContext<LocalContext>, port: u16) -> HandlerResponse<'_>;
}

/// Trait for BRP handlers using function pointer approach
pub trait BrpToolFn: Send + Sync {
    /// Handle the BRP request and return a result
    fn call(&self, ctx: &HandlerContext<BrpContext>) -> BrpHandlerResponse<'_>;
}

/// Unified context that wraps both Local and BRP handler contexts
#[derive(Clone)]
pub enum ToolContext {
    Local(HandlerContext<LocalContext>),
    Brp(HandlerContext<BrpContext>),
}

/// BRP method source specification for tool handlers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrpMethodSource {
    /// Static method name known at compile time
    Static(&'static str),
    /// Dynamic method name extracted from request parameters
    Dynamic,
}
