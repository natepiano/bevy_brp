use std::sync::Arc;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolResult;

use super::handler_context::HandlerContext;
use super::types::{BrpToolFn, ErasedBrpToolFn, ErasedLocalToolFn, LocalToolFn, ToolContext};
use crate::response::FormatterConfig;

/// Trait for extracting formatter config from any tool context
trait HasFormatterConfig {
    fn formatter_config(&self) -> FormatterConfig;
}

impl HasFormatterConfig for ToolContext {
    fn formatter_config(&self) -> FormatterConfig {
        match self {
            Self::Local(local_ctx) => create_formatter_from_def(local_ctx),
            Self::Brp(brp_ctx) => create_formatter_from_def(brp_ctx),
        }
    }
}

/// Enum to hold either local handler or BRP handler
#[derive(Clone)]
pub enum HandlerFn {
    Local(Arc<dyn ErasedLocalToolFn>),
    Brp(Arc<dyn ErasedBrpToolFn>),
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
            (Self::Local(handler), ToolContext::Local(local_ctx)) => {
                handler.call_erased(local_ctx, formatter_config)
            }
            (Self::Brp(handler), ToolContext::Brp(brp_ctx)) => {
                handler.call_erased(brp_ctx, formatter_config)
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
    /// Create a local handler with automatic Arc wrapping
    pub fn local<T: LocalToolFn + 'static>(handler: T) -> Self {
        Self::Local(Arc::new(handler))
    }

    /// Create a BRP handler
    pub fn brp<T: BrpToolFn + 'static>(handler: T) -> Self {
        Self::Brp(Arc::new(handler))
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
