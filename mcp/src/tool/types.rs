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

use async_trait::async_trait;
use rmcp::model::CallToolResult;

use super::handler_context::HandlerContext;
use super::response_builder::ResponseBuilder;
use crate::error::Result;
use crate::tool::ParamStruct;

/// Framework-level result for tool handler execution.
/// Catches infrastructure errors like parameter extraction failures,
/// system-level errors, or handler setup issues.
///
/// The lifetime parameter `'a` is required because the `Future` captures `&self`
/// when calling trait methods in the async block.
pub type HandlerResult<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Result wrapper that includes parameters so they can be returned in the response.
/// Allows for heterogeneous results and parameters that are optional (unit struct for optional)
#[derive(Debug)]
pub struct ToolResult<T, P = ()> {
    /// The actual result of the tool's business logic
    pub result: Result<T>,
    /// The parameters that were passed to the tool (if any)
    pub params: Option<P>,
}

/// Unified trait for all tool handlers (local and BRP)
///
/// # Implementation Requirements
///
/// Tools must implement either `handle_impl` (most common) or `handle_impl_with_context` (when
/// context is needed):
///
/// ## Most tools (no context needed):
/// ```rust
/// impl ToolFn for MyTool {
///     type Output = MyResult;
///     type Params = MyParams;
///
///     async fn handle_impl(&self, params: MyParams) -> Result<MyResult> {
///         // Your implementation here - no context parameter
///     }
/// }
/// ```
///
/// ## Context-needing tools (e.g., list tools that need workspace roots):
/// ```rust
/// impl ToolFn for ListTool {
///     type Output = ListResult;
///     type Params = NoParams;
///
///     async fn handle_impl_with_context(
///         &self,
///         ctx: HandlerContext,
///         _params: NoParams,
///     ) -> Result<ListResult> {
///         let search_paths = &ctx.roots;
///         // Implementation using context
///     }
/// }
/// ```
#[async_trait]
pub trait ToolFn: Send + Sync {
    /// The concrete type returned by this handler
    type Output: ResultStruct + Send + Sync;
    /// The parameter type for this handler
    type Params: ParamStruct;

    /// Handle the request with just parameters (most common case)
    /// Default implementation panics - tools must implement either this or
    /// `handle_impl_with_context`
    async fn handle_impl(&self, _params: Self::Params) -> Result<Self::Output> {
        unimplemented!("Must implement either handle_impl or handle_impl_with_context")
    }

    /// Handle the request with context (for tools that need `HandlerContext`)
    /// Default implementation ignores context and calls `handle_impl`
    async fn handle_impl_with_context(
        &self,
        _ctx: HandlerContext,
        params: Self::Params,
    ) -> Result<Self::Output> {
        self.handle_impl(params).await
    }

    /// Handle the request and return `ToolResult`
    /// Default implementation extracts parameters and calls `handle_impl_with_context`
    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<'_, ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            let params: Self::Params = ctx.extract_parameter_values()?;
            let result = self.handle_impl_with_context(ctx, params).await;
            Ok(ToolResult {
                result,
                params: None, // Don't include params in response if we can't clone them
            })
        })
    }
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
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send + 'a>>;
}

/// Blanket implementation to convert typed `ToolFn`s to erased ones
impl<T: ToolFn> ErasedToolFn for T {
    fn call_erased<'a>(
        &'a self,
        ctx: HandlerContext,
    ) -> Pin<Box<dyn Future<Output = CallToolResult> + Send + 'a>> {
        Box::pin(async move {
            // we're making a judgement call that we passed a reference to call()

            let result = self.call(ctx.clone()).await;
            match result {
                Ok(tool_result) => ctx.format_result(tool_result),
                Err(e) => ctx.format_framework_error(e),
            }
        })
    }
}

/// Trait for types that can be used as results
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
