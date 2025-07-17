use std::sync::Arc;

use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use super::format_v2;
use super::types::{BrpMethodSource, BrpToolFnV2};
use crate::response::{FormatterConfig, ResponseBuilder, ResponseFormatter};
use crate::service::{HandlerContext, LocalContext};
use crate::tool::types::ToolContext;
use crate::tool::{LocalToolFn, LocalToolFnWithPort};

/// Enum to hold either basic handler or handler with port
#[derive(Clone)]
pub enum HandlerFn {
    Local(Arc<dyn LocalToolFn>),
    LocalWithPort(Arc<dyn LocalToolFnWithPort>),
    BrpV2 {
        handler:       Arc<dyn super::types::BrpToolFnV2>,
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
        match (self, ctx) {
            (Self::Local(handler), ToolContext::Local(local_ctx)) => {
                let formatter_config = create_formatter_from_def(local_ctx);

                Box::pin(async move {
                    let result = handler
                        .call(local_ctx)
                        .await
                        .map(|typed_result| typed_result.to_json());
                    format_v2::format_tool_call_result_v2(result, local_ctx, formatter_config)
                })
            }
            (Self::LocalWithPort(handler), ToolContext::Local(local_ctx)) => {
                let formatter_config = create_formatter_from_def(local_ctx);

                let Some(port) = local_ctx.port() else {
                    return Box::pin(async move {
                        Err(McpError::invalid_params(
                            "WithPort handler called without port parameter",
                            None,
                        ))
                    });
                };

                Box::pin(async move {
                    let result = handler
                        .call(local_ctx, port)
                        .await
                        .map(|typed_result| typed_result.to_json());
                    format_tool_call_result(result, local_ctx, formatter_config)
                })
            }
            (Self::BrpV2 { handler, .. }, ToolContext::Brp(brp_ctx)) => {
                let formatter_config = brp_ctx.tool_def().formatter().build_formatter_config();

                Box::pin(async move {
                    // Call V2 handler, get HandlerResult
                    let result = handler.call(brp_ctx).await;

                    // Convert to JSON and format like local handlers
                    let json_result = result.map(|typed_result| typed_result.to_json());

                    // Use enhanced format_tool_call_result from separate file
                    format_v2::format_tool_call_result_v2(json_result, brp_ctx, formatter_config)
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
    pub fn brp_v2_static<T: BrpToolFnV2 + 'static>(handler: T, method: &'static str) -> Self {
        Self::BrpV2 {
            handler:       Arc::new(handler),
            method_source: BrpMethodSource::Static(method),
        }
    }

    /// Create a BRP V2 handler with dynamic method (from parameter)
    pub fn brp_v2_dynamic<T: BrpToolFnV2 + 'static>(handler: T) -> Self {
        Self::BrpV2 {
            handler:       Arc::new(handler),
            method_source: BrpMethodSource::Dynamic,
        }
    }
}

/// Create formatter config from tool definition
fn create_formatter_from_def(handler_context: &HandlerContext<LocalContext>) -> FormatterConfig {
    // Build the formatter config from the response specification
    handler_context
        .tool_def()
        .formatter()
        .build_formatter_config()
}

/// Format the result of a local tool handler that returns `Result<Value, McpError>` using
/// `HandlerContext`
///
/// todo: address the cognitive load of all of these conditionals - can we refactor
/// to use the type system?
fn format_tool_call_result(
    result: Result<serde_json::Value, McpError>,
    handler_context: &HandlerContext<LocalContext>,
    formatter_config: FormatterConfig,
) -> Result<CallToolResult, McpError> {
    match result {
        Ok(value) => {
            // Check if the value contains a status field indicating an error
            let is_error = value
                .get("status")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "error");

            let call_info = handler_context.call_info();

            if is_error {
                // For error responses, build the error response directly
                let message = value
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Operation failed");

                // Build error response with all the data fields
                let error_response = ResponseBuilder::error(call_info.clone()).message(message);

                // For disambiguation errors, only include specific fields
                let error_response = if let serde_json::Value::Object(map) = &value {
                    // Check if this is a disambiguation error by looking for duplicate_paths
                    let is_disambiguation = map
                        .get("duplicate_paths")
                        .and_then(|v| v.as_array())
                        .is_some_and(|arr| !arr.is_empty());

                    if is_disambiguation {
                        // For disambiguation errors, only include the name field and
                        // duplicate_paths
                        map.iter()
                            .filter(|(key, val)| {
                                let k = key.as_str();
                                k != "status"
                                    && k != "message"
                                    && (k == "duplicate_paths"
                                        || k == "app_name"
                                        || k == "example_name")
                                    && !val.is_null()
                            })
                            .try_fold(error_response, |builder, (key, val)| {
                                builder.add_field(key, val)
                            })
                            .unwrap_or_else(|_| {
                                // If adding fields failed, just return the basic error response
                                ResponseBuilder::error(call_info.clone()).message(message)
                            })
                    } else {
                        // For other errors, include all non-null fields
                        map.iter()
                            .filter(|(key, val)| {
                                key.as_str() != "status"
                                    && key.as_str() != "message"
                                    && !val.is_null()
                            })
                            .try_fold(error_response, |builder, (key, val)| {
                                builder.add_field(key, val)
                            })
                            .unwrap_or_else(|_| {
                                // If adding fields failed, just return the basic error response
                                ResponseBuilder::error(call_info.clone()).message(message)
                            })
                    }
                } else {
                    error_response
                };

                Ok(error_response.build().to_call_tool_result())
            } else {
                // Use the new format_success_with_context to include call_info
                Ok(
                    ResponseFormatter::new(formatter_config)
                        .format_success(&value, handler_context),
                )
            }
        }
        Err(e) => Err(e),
    }
}
