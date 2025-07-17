use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::response::{FormatterConfig, ResponseBuilder, ResponseFormatter};
use crate::service::{HandlerContext, HasCallInfo};

/// V2 formatter that handles both local and BRP results uniformly
pub fn format_tool_call_result_v2<T>(
    result: Result<serde_json::Value, McpError>,
    handler_context: &HandlerContext<T>,
    formatter_config: FormatterConfig,
) -> Result<CallToolResult, McpError>
where
    HandlerContext<T>: HasCallInfo,
{
    match result {
        Ok(value) => {
            // Check if this is an error response
            let is_error = value
                .get("status")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "error");

            if is_error {
                // Handle error response
                let message = value
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");

                let error_response =
                    ResponseBuilder::error(handler_context.call_info()).message(message);

                // Add error fields as metadata
                let error_response = if let serde_json::Value::Object(map) = &value {
                    map.iter()
                        .filter(|(key, val)| {
                            let k = key.as_str();
                            k != "status" && k != "message" && !val.is_null()
                        })
                        .try_fold(error_response, |builder, (key, val)| {
                            builder.add_field(key, val)
                        })
                        .unwrap_or_else(|_| {
                            // If adding fields failed, just return the basic error response
                            ResponseBuilder::error(handler_context.call_info()).message(message)
                        })
                } else {
                    error_response
                };

                Ok(error_response.build().to_call_tool_result())
            } else {
                // Handle success response
                let formatter = ResponseFormatter::new(formatter_config);

                // For V2, the entire value contains the structured result with data field
                // The formatter will extract fields based on ResponseSpecification
                Ok(formatter.format_success(&value, handler_context))
            }
        }
        Err(e) => Err(e),
    }
}
