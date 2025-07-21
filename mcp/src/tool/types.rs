//! Tool handler type system with type erasure for heterogeneous storage.
//!
//! This module provides a two-layer trait system:
//!
//! 1. **Typed traits** (`LocalToolFn`, etc.) - Preserve concrete return types
//!    - Each handler specifies its own `Output` type
//!    - Provides type safety at implementation site
//!
//! 2. **Erased traits** (`ErasedLocalToolFn`, etc.) - Hide type information
//!    - Return a uniform `CallToolResult` type
//!    - Allow different handlers to be stored in the same collection
//!
//! The blanket implementations automatically convert typed handlers to erased ones,
//! calling the typed handler internally and formatting the result. This allows
//! `HandlerFn` enum to store `Arc<dyn ErasedLocalToolFn>` while handlers only
//! implement the simpler typed interface.

use std::future::Future;
use std::pin::Pin;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolResult;

use super::HandlerFn;
use super::handler_context::{HandlerContext, HasMethod, HasPort, NoMethod, NoPort};
use crate::error::Result;
use crate::response::FormatterConfig;

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
    pub async fn call_tool(self) -> std::result::Result<CallToolResult, McpError> {
        self.handler.call_handler(&self.context).await
    }
}

/// Type alias for the response from local handlers
///
/// Breaking down the type:
/// - `Pin<Box<...>>`: Heap-allocated Future that won't move in memory
/// - `dyn Future`: Async function that can be awaited
/// - `Output = crate::error::Result<T>`: Can fail with internal `Error` type
/// - `+ Send + 'static`: Can be sent between threads, static lifetime
pub type HandlerResponse<T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'static>>;

/// Trait for local handlers using function pointer approach
pub trait LocalToolFn: Send + Sync {
    /// The concrete type returned by this handler
    type Output: serde::Serialize + Send + Sync;

    /// Handle the request and return a typed result
    fn call(&self, ctx: &HandlerContext<NoPort, NoMethod>) -> HandlerResponse<Self::Output>;
}

/// Trait for local handlers with port - no separate port parameter needed
pub trait LocalToolFnWithPort: Send + Sync {
    /// The concrete type returned by this handler
    type Output: serde::Serialize + Send + Sync;

    /// Handle the request and return a typed result - handlers call `ctx.port()` directly
    fn call(&self, ctx: &HandlerContext<HasPort, NoMethod>) -> HandlerResponse<Self::Output>;
}

/// Trait for BRP handlers that return `HandlerResponse` (unified with local handlers)
pub trait BrpToolFn: Send + Sync {
    /// The concrete type returned by this handler
    type Output: serde::Serialize + Send + Sync;

    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<Self::Output>;
}

/// Type-erased version for use in `HandlerFn` enum
/// These traits return `CallToolResult` directly, avoiding double serialization
pub trait ErasedLocalToolFn: Send + Sync {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext<NoPort, NoMethod>,
        formatter_config: FormatterConfig,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>;
}

pub trait ErasedLocalToolFnWithPort: Send + Sync {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext<HasPort, NoMethod>,
        formatter_config: FormatterConfig,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>;
}

pub trait ErasedBrpToolFn: Send + Sync {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext<HasPort, HasMethod>,
        formatter_config: FormatterConfig,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>;
}

/// Blanket implementations to convert typed handlers to erased ones
impl<T: LocalToolFn> ErasedLocalToolFn for T {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext<NoPort, NoMethod>,
        formatter_config: FormatterConfig,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>
    {
        Box::pin(async move {
            let result = self.call(ctx).await;
            crate::response::format_tool_result(result, ctx, formatter_config)
        })
    }
}

impl<T: LocalToolFnWithPort> ErasedLocalToolFnWithPort for T {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext<HasPort, NoMethod>,
        formatter_config: FormatterConfig,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>
    {
        Box::pin(async move {
            let result = self.call(ctx).await;
            crate::response::format_tool_result(result, ctx, formatter_config)
        })
    }
}

impl<T: BrpToolFn> ErasedBrpToolFn for T {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext<HasPort, HasMethod>,
        formatter_config: FormatterConfig,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>
    {
        Box::pin(async move {
            let result = self.call(ctx).await;
            crate::response::format_tool_result(result, ctx, formatter_config)
        })
    }
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
