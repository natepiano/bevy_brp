use std::sync::Arc;

use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::response::{FormatterConfig, ResponseBuilder, ResponseFormatter};
use crate::service::{HandlerContext, LocalContext};
use crate::tool::{HandlerResponse, LocalToolFunction, LocalToolFunctionWithPort, ToolHandler};

/// Enum to hold either basic handler or handler with port
#[derive(Clone)]
pub enum LocalHandler {
    Basic(Arc<dyn LocalToolFunction>),
    WithPort(Arc<dyn LocalToolFunctionWithPort>),
}

impl LocalHandler {
    /// Dispatch method that calls the appropriate handler based on type
    pub fn call_handler<'a>(
        &'a self,
        ctx: &'a HandlerContext<LocalContext>,
    ) -> HandlerResponse<'a> {
        match self {
            Self::Basic(handler) => handler.call(ctx),
            Self::WithPort(handler) => {
                let port = ctx.port().ok_or_else(|| {
                    McpError::invalid_params("WithPort handler called without port parameter", None)
                });

                match port {
                    Ok(p) => handler.call(ctx, p),
                    Err(e) => Box::pin(async move { Err(e) }),
                }
            }
        }
    }
}

impl LocalHandler {
    /// Create a Basic handler with automatic Arc wrapping
    pub fn basic<T: LocalToolFunction + 'static>(handler: T) -> Self {
        Self::Basic(Arc::new(handler))
    }

    /// Create a `WithPort` handler with automatic Arc wrapping
    pub fn with_port<T: LocalToolFunctionWithPort + 'static>(handler: T) -> Self {
        Self::WithPort(Arc::new(handler))
    }
}

pub struct LocalToolHandler {
    context: HandlerContext<LocalContext>,
    handler: LocalHandler,
}

impl LocalToolHandler {
    pub const fn new(context: HandlerContext<LocalContext>, handler: LocalHandler) -> Self {
        Self { context, handler }
    }
}

#[async_trait]
impl ToolHandler for LocalToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError> {
        local_tool_call(&self.context, &self.handler).await
    }
}

/// Generate a `LocalFn` handler using function pointer approach
async fn local_tool_call(
    handler_context: &HandlerContext<LocalContext>,
    handler: &LocalHandler,
) -> Result<CallToolResult, McpError> {
    let formatter_config = create_formatter_from_def(handler_context)?;

    let result = handler
        .call_handler(handler_context)
        .await
        .map(|typed_result| typed_result.to_json());

    format_tool_call_result(result, handler_context, formatter_config)
}

/// Create formatter config from tool definition
fn create_formatter_from_def(
    handler_context: &HandlerContext<LocalContext>,
) -> Result<FormatterConfig, McpError> {
    let tool_def = handler_context.tool_def()?;

    // Build the formatter config from the response specification
    let formatter_config = tool_def.formatter().build_formatter_config();

    Ok(formatter_config)
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
