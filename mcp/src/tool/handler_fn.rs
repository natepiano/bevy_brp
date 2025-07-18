use std::sync::Arc;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolResult;

use super::handler_context::HandlerContext;
use super::types::{BrpMethodSource, BrpToolFn};
use crate::response::{FormatterConfig, format_tool_call_result};
use crate::tool::types::ToolContext;
use crate::tool::{LocalToolFn, LocalToolFnWithPort};

/// Trait for extracting formatter config from any tool context
trait HasFormatterConfig {
    fn formatter_config(&self) -> FormatterConfig;
}

impl HasFormatterConfig for ToolContext {
    fn formatter_config(&self) -> FormatterConfig {
        match self {
            Self::Local(local_ctx) => create_formatter_from_def(local_ctx),
            Self::LocalWithPort(local_with_port_ctx) => {
                create_formatter_from_def(local_with_port_ctx)
            }
            Self::Brp(brp_ctx) => create_formatter_from_def(brp_ctx),
        }
    }
}

/// Enum to hold either basic handler or handler with port
#[derive(Clone)]
pub enum HandlerFn {
    Local(Arc<dyn LocalToolFn>),
    LocalWithPort(Arc<dyn LocalToolFnWithPort>),
    Brp {
        handler:       Arc<dyn super::types::BrpToolFn>,
        method_source: BrpMethodSource,
    },
}

impl HandlerFn {
    /// Dispatch method that calls the appropriate handler based on type
    pub fn call_handler<'a>(
        &'a self,
        ctx: &'a ToolContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<CallToolResult, McpError>> + Send + 'a>,
    > {
        // Generate formatter config once using the trait method
        let formatter_config = ctx.formatter_config();

        // Now dispatch to the appropriate handler
        match (self, ctx) {
            (Self::Local(handler), ToolContext::Local(local_ctx)) => Box::pin(async move {
                let result = handler
                    .call(local_ctx)
                    .await
                    .map(|typed_result| typed_result.to_json());
                format_tool_call_result(result, local_ctx, formatter_config)
            }),
            (Self::LocalWithPort(handler), ToolContext::LocalWithPort(local_with_port_ctx)) => {
                Box::pin(async move {
                    let result = handler
                        .call(local_with_port_ctx)
                        .await
                        .map(|typed_result| typed_result.to_json());
                    format_tool_call_result(result, local_with_port_ctx, formatter_config)
                })
            }
            (Self::Brp { handler, .. }, ToolContext::Brp(brp_ctx)) => {
                Box::pin(async move {
                    // Call V2 handler, get HandlerResult
                    let result = handler.call(brp_ctx).await;

                    // Convert to JSON and format like local handlers
                    let json_result = result.map(|typed_result| typed_result.to_json());

                    // Use enhanced format_tool_call_result
                    format_tool_call_result(json_result, brp_ctx, formatter_config)
                })
            }
            _ => Box::pin(async move {
                Err(McpError::invalid_params(
                    "Invalid handler/context combination",
                    None,
                ))
            }),
        }
    }
}

impl HandlerFn {
    /// Create a Basic handler with automatic Arc wrapping
    pub fn local<T: LocalToolFn + 'static>(handler: T) -> Self {
        Self::Local(Arc::new(handler))
    }

    /// Create a `WithPort` handler with automatic Arc wrapping
    pub fn local_with_port<T: LocalToolFnWithPort + 'static>(handler: T) -> Self {
        Self::LocalWithPort(Arc::new(handler))
    }

    /// Create a BRP V2 handler with static method
    pub fn brp_v2_static<T: BrpToolFn + 'static>(handler: T, method: &'static str) -> Self {
        Self::Brp {
            handler:       Arc::new(handler),
            method_source: BrpMethodSource::Static(method),
        }
    }

    /// Create a BRP V2 handler with dynamic method (from parameter)
    pub fn brp_v2_dynamic<T: BrpToolFn + 'static>(handler: T) -> Self {
        Self::Brp {
            handler:       Arc::new(handler),
            method_source: BrpMethodSource::Dynamic,
        }
    }
}

/// Create formatter config from tool definition (generic for all context types)
fn create_formatter_from_def<Port, Method>(
    handler_context: &HandlerContext<Port, Method>,
) -> FormatterConfig {
    // Build the formatter config from the response specification
    handler_context
        .tool_def()
        .formatter()
        .build_formatter_config()
}
