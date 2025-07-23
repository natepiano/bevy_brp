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

use super::handler_context::HandlerContext;
use crate::error::Result;
use crate::response::{CallInfoProvider, FieldAccessor, LocalCallInfo, ResponseData, ResponseDef};

/// Type alias for the response from local handlers
///
/// Breaking down the type:
/// - `Pin<Box<...>>`: Heap-allocated Future that won't move in memory
/// - `dyn Future`: Async function that can be awaited
/// - `Output = crate::error::Result<T>`: Can fail with internal `Error` type
/// - `+ Send + 'static`: Can be sent between threads, static lifetime
pub type HandlerResponse<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

/// Unified trait for all tool handlers (local and BRP)
pub trait ToolFn: Send + Sync {
    /// The concrete type returned by this handler
    type Output: ResponseData + Send + Sync;
    /// The type that provides `CallInfo` data for this tool
    type CallInfoData: CallInfoProvider;

    /// Handle the request and return a typed result with `CallInfo` data
    fn call(&self, ctx: &HandlerContext) -> HandlerResponse<(Self::CallInfoData, Self::Output)>;
}

/// Extension trait for tools whose output implements FieldAccessor
pub trait ToolFnWithFieldAccess: ToolFn
where
    Self::Output: FieldAccessor,
{
    // Marker trait
}

/// Type-erased version for heterogeneous storage
/// Provides consistent formatting the Result for all tool calls - reducing potential bugs
/// Also allows us to pass the typed Result to the formatter although
/// the formatter does serialize it right away so this may be of dubious value
///
/// Without some kind of type erasure, we can't use the associated types on `ToolFn`
/// If retaining the type info is deemed unnecessary, we could serialize result, get rid of
/// the type erasure and and simplify the call flow a bit.
pub trait ErasedUnifiedToolFn: Send + Sync {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext,
        response_def: ResponseDef,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>;
}

/// Blanket implementation to convert typed `ToolFn`s to erased ones
impl<T: ToolFn> ErasedUnifiedToolFn for T {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext,
        response_def: ResponseDef,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<CallToolResult, McpError>> + Send + 'a>>
    {
        Box::pin(async move {
            let result = self.call(ctx).await;
            match result {
                Ok((call_info_data, output)) => {
                    // Use standard format_result which will internally check for FieldAccessor
                    response_def.format_result(Ok(output), ctx, call_info_data)
                }
                Err(e) => {
                    // For errors, we don't have CallInfoData, so use a default LocalCallInfo
                    response_def.format_result::<T::Output, _>(Err(e), ctx, LocalCallInfo)
                }
            }
        })
    }
}

/// Unified tool handler that works with any tool
pub struct ToolHandler {
    handler: std::sync::Arc<dyn ErasedUnifiedToolFn>,
    context: HandlerContext,
}

impl ToolHandler {
    pub const fn new(
        handler: std::sync::Arc<dyn ErasedUnifiedToolFn>,
        context: HandlerContext,
    ) -> Self {
        Self { handler, context }
    }
}

impl ToolHandler {
    pub async fn call_tool(self) -> std::result::Result<CallToolResult, McpError> {
        // Generate formatter config from tool definition
        let response_spec = self.context.tool_def().response_def();

        self.handler
            .call_erased(&self.context, response_spec.clone())
            .await
    }
}
