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

use rmcp::model::CallToolResult;

use super::handler_context::HandlerContext;
use super::tool_name::ToolName;
use crate::brp_tools::Port;
use crate::error::Result;
use crate::tool::{CallInfo, ResponseData};

/// Framework-level result for tool handler execution.
/// Catches infrastructure errors like parameter extraction failures,
/// system-level errors, or handler setup issues.
pub type HandlerResult<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

/// Business logic result wrapper that includes optional port.
#[derive(Debug)]
pub struct ToolResult<T> {
    /// Optional port for tools that use ports
    pub port:   Option<Port>,
    /// The actual result of the tool's business logic
    pub result: Result<T>,
}

impl<T> ToolResult<T> {
    /// Create a result with port
    pub const fn with_port(result: Result<T>, port: Port) -> Self {
        Self {
            port: Some(port),
            result,
        }
    }

    /// Create a result without port
    pub const fn without_port(result: Result<T>) -> Self {
        Self { port: None, result }
    }
}

/// Unified trait for all tool handlers (local and BRP)
pub trait ToolFn: Send + Sync {
    /// The concrete type returned by this handler
    type Output: ResponseData + MessageTemplateProvider + Send + Sync;

    /// Handle the request and return `ToolResult` with optional port
    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output>>;
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
        ctx: HandlerContext,
        tool_name: ToolName,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult>> + Send + 'a>>;
}

/// Blanket implementation to convert typed `ToolFn`s to erased ones
impl<T: ToolFn> ErasedUnifiedToolFn for T {
    fn call_erased<'a>(
        &'a self,
        ctx: HandlerContext,
        tool_name: ToolName,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult>> + Send + 'a>> {
        Box::pin(async move {
            // we're making a judgement call that we passed a reference to call()

            let result = self.call(ctx.clone()).await;
            match result {
                Ok(tool_result) => {
                    // Construct CallInfo from tool name and optional port
                    let call_info =
                        CallInfo::from_tool_and_port(tool_name.to_string(), tool_result.port);
                    tool_name.format_result(call_info, tool_result.result, &ctx)
                }
                Err(e) => {
                    // Framework error - can't extract parameters or other infrastructure issue
                    Ok(tool_name.format_framework_error(e, &ctx))
                }
            }
        })
    }
}

/// Trait for types that can provide dynamic message templates
///
/// This trait is automatically implemented by the `ResultStruct` derive macro
/// for structs that have a field with `#[to_message(message_template = "...")]`.
///
/// **Important**: When this trait is implemented via the macro:
/// - All struct fields become private
/// - A `::new()` constructor is generated
/// - The struct can ONLY be constructed via `::new()` to ensure the message template is set
///
/// # Example
/// ```ignore
/// #[derive(ResultStruct)]
/// struct MyResult {
///     #[to_metadata]
///     count: usize,  // This becomes private!
///
///     #[to_message(message_template = "Processed {count} items")]
///     message_template: String,  // This becomes private!
/// }
///
/// // Can only construct via:
/// let result = MyResult::new(42);
/// ```
pub trait MessageTemplateProvider {
    /// Get the message template for this response
    fn get_message_template(&self) -> Result<&str>;
}
