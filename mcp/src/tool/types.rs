use std::future::Future;
use std::pin::Pin;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolResult;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::HandlerFn;
use super::handler_context::{HandlerContext, HasMethod, HasPort, NoMethod, NoPort};
use crate::field_extraction::ResponseFieldName;
use crate::response::ResponseStatus;

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

/// Standard error type for tool responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub message: String,
    #[serde(flatten)]
    pub details: Option<serde_json::Value>,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: None,
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

/// Type alias for BRP handler responses
/// Result type that all local handlers must return
pub trait HandlerResult: Send + Sync {
    /// Serialize this result to a JSON value (required due to dyn compatibility)
    fn to_json(&self) -> serde_json::Value;
}

/// Wrapper that converts Result<T, `ToolError`> to JSON with status field
pub struct ToolResult<T>(pub Result<T, ToolError>);

impl<T: Serialize + Send + Sync> HandlerResult for ToolResult<T> {
    fn to_json(&self) -> serde_json::Value {
        match &self.0 {
            Ok(data) => {
                let mut json = serde_json::to_value(data).unwrap_or_else(|_| json!({}));
                if let Value::Object(map) = &mut json {
                    map.insert(
                        ResponseFieldName::Status.to_string(),
                        json!(ResponseStatus::Success),
                    );
                }
                json
            }
            Err(error) => {
                json!({
                    ResponseFieldName::Status.to_string(): ResponseStatus::Error,
                    ResponseFieldName::Message.to_string(): error.message,
                    "error_details": error.details
                })
            }
        }
    }
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
