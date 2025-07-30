//! Tool handler type system with type erasure for heterogeneous storage.
//!
//! This module provides a two-layer trait system:
//!
//! 1. **Typed traits** (`ToolFn`) - Preserve concrete return types
//!    - Each handler specifies its own `Output` and `Params` types
//!    - Provides type safety at implementation site
//!
//! 2. **Erased traits** (`ErasedToolFn`) - Hide type information
//!    - Return a uniform `CallToolResult` type
//!    - Allow different handlers to be stored in the same collection
//!
//! The blanket implementation automatically converts typed handlers to erased ones,
//! calling the typed handler internally and formatting the result. This allows
//! collections to store `Arc<dyn ErasedToolFn>` while handlers only need to
//! implement the simpler typed `ToolFn` interface.

use std::future::Future;
use std::pin::Pin;

use rmcp::model::CallToolResult;

use super::handler_context::HandlerContext;
use super::tool_name::ToolName;
use crate::error::Result;
use crate::tool::{ParamStruct, ResponseBuilder};

/// Framework-level result for tool handler execution.
/// Catches infrastructure errors like parameter extraction failures,
/// system-level errors, or handler setup issues.
pub type HandlerResult<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

/// Business logic result wrapper that includes parameters.
/// Allows for heterogeneous results and parameters that are optional (unit struct for optional)
#[derive(Debug)]
pub struct ToolResult<T, P = ()> {
    /// The actual result of the tool's business logic
    pub result: Result<T>,
    /// The parameters that were passed to the tool (if any)
    pub params: Option<P>,
}

/// Unified trait for all tool handlers (local and BRP)
pub trait ToolFn: Send + Sync {
    /// The concrete type returned by this handler
    type Output: ResultStruct + Send + Sync;
    /// The parameter type for this handler
    type Params: ParamStruct;

    /// Handle the request and return `ToolResult`
    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>>;
}

/// Type-erased version for heterogeneous storage
/// Provides consistent formatting the Result for all tool calls - reducing potential bugs
/// Also allows us to pass the typed Result to the formatter although
/// the formatter does serialize it right away so this may be of dubious value
///
/// Without some kind of type erasure, we can't use the associated types on `ToolFn`
/// If retaining the type info is deemed unnecessary, we could serialize result, get rid of
/// the type erasure and and simplify the call flow a bit.
pub trait ErasedToolFn: Send + Sync {
    fn call_erased<'a>(
        &'a self,
        ctx: HandlerContext,
        tool_name: ToolName,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send + 'a>>;
}

/// Blanket implementation to convert typed `ToolFn`s to erased ones
impl<T: ToolFn> ErasedToolFn for T {
    fn call_erased<'a>(
        &'a self,
        ctx: HandlerContext,
        tool_name: ToolName,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send + 'a>> {
        Box::pin(async move {
            // we're making a judgement call that we passed a reference to call()

            let result = self.call(ctx.clone()).await;
            match result {
                Ok(tool_result) => {
                    // Pass tool_result to format_result, which will create CallInfo internally
                    // This now returns CallToolResult directly, not Result<CallToolResult>
                    tool_name.format_result(tool_result, &ctx)
                }
                Err(e) => {
                    // Framework error - can't extract parameters or other infrastructure issue
                    // This also returns CallToolResult directly
                    tool_name.format_framework_error(e, &ctx)
                }
            }
        })
    }
}

/// Trait for types that can be used as structured results
///
/// This trait is automatically implemented by the `ResultStruct` derive macro.
///
/// **Important**: When this trait is implemented via the macro:
/// - All struct fields become private
/// - A `::new()` constructor is generated
/// - The struct can ONLY be constructed via `::new()` to ensure proper initialization
/// - Fields with `#[to_message(message_template = "...")]` provide the message template
/// - Fields with `#[to_metadata]` or `#[to_result]` are added to the response
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
pub trait ResultStruct: Send + Sync {
    /// Add all response fields to the builder
    fn add_response_fields(&self, builder: ResponseBuilder) -> Result<ResponseBuilder>;

    /// Get the message template for this response
    fn get_message_template(&self) -> Result<&str>;
}
