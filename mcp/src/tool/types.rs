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
use crate::error::Result;
use crate::tool::{CallInfoProvider, LocalCallInfo, ResponseData};

/// Helper trait to convert a `Result<T>` into the tuple format required by `ToolFn`
pub trait WithCallInfo<C: CallInfoProvider> {
    /// Convert a `Result<T>` to `(CallInfoData, Result<T>)` format
    fn with_call_info(self, call_info: C) -> (C, Self);
}

impl<T, C: CallInfoProvider> WithCallInfo<C> for Result<T> {
    fn with_call_info(self, call_info: C) -> (C, Self) {
        (call_info, self)
    }
}

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

    /// Handle the request and return `CallInfo` data with a typed result
    /// `CallInfo` is always returned, even in error cases, ensuring proper context is preserved
    fn call(
        &self,
        ctx: &HandlerContext,
    ) -> HandlerResponse<(Self::CallInfoData, Result<Self::Output>)>;
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
        tool_name: ToolName,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult>> + Send + 'a>>;
}

/// Blanket implementation to convert typed `ToolFn`s to erased ones
impl<T: ToolFn> ErasedUnifiedToolFn for T {
    fn call_erased<'a>(
        &'a self,
        ctx: &'a HandlerContext,
        tool_name: ToolName,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult>> + Send + 'a>> {
        Box::pin(async move {
            let result = self.call(ctx).await;
            match result {
                Ok((call_info_data, inner_result)) => {
                    // Now we always have call_info_data, regardless of success or error
                    // prior to this we returned the an Err that lost the call_info_data and we had
                    // to default now we can return an Ok with call info always
                    // and the result itself will be Ok/Err depending on the inner result
                    tool_name.format_result(call_info_data, inner_result, ctx)
                }
                Err(e) => {
                    // This should be rare - only if the handler itself fails before returning
                    // CallInfo In this case, we still need to use a default whicdh is just the
                    // tool name
                    tool_name.format_result::<T::Output, _>(LocalCallInfo, Err(e), ctx)
                }
            }
        })
    }
}
