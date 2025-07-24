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
use crate::tool::{CallInfoProvider, ResponseData};

/// Framework-level result for tool handler execution.
/// Catches infrastructure errors like parameter extraction failures,
/// system-level errors, or handler setup issues.
pub type HandlerResult<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

/// Business logic result wrapper that always includes call info data.
/// Generic over C: `CallInfoProvider` to preserve the tool's specific call info type.
#[derive(Debug)]
pub struct ToolResult<T, C: CallInfoProvider> {
    pub call_info_data: C,
    /// The actual result of the tool's business logic
    pub result:         Result<T>,
}

impl<T, C: CallInfoProvider> ToolResult<T, C> {
    /// Create a result from call info data and result
    pub const fn from_result(result: Result<T>, call_info_data: C) -> Self {
        Self {
            call_info_data,
            result,
        }
    }
}

/// Unified trait for all tool handlers (local and BRP)
pub trait ToolFn: Send + Sync {
    /// The concrete type returned by this handler
    type Output: ResponseData + Send + Sync;
    /// The type that provides `CallInfo` data for this tool
    type CallInfoData: CallInfoProvider;

    /// Handle the request and return `ToolResult` with `CallInfoData`
    fn call(
        &self,
        ctx: HandlerContext,
    ) -> HandlerResult<ToolResult<Self::Output, Self::CallInfoData>>;
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
                    // Process the ToolResult - pass both call_info_data and result to format_result
                    tool_name.format_result(tool_result.call_info_data, tool_result.result, &ctx)
                }
                Err(e) => {
                    // Framework error - can't extract parameters or other infrastructure issue
                    // Use simple LocalCallInfo default and call format_framework_error
                    Ok(tool_name.format_framework_error(e, &ctx))
                }
            }
        })
    }
}
