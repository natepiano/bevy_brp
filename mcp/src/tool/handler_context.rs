use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use error_stack::ResultExt;
use rmcp::model::{CallToolRequestParam, CallToolResult};
use serde_json::{Value, json};

use super::json_response::ToolCallJsonResponse;
use crate::error::{Error, Result};
use crate::tool::large_response::{CHARS_PER_TOKEN, LargeResponseConfig};
use crate::tool::response_builder::Response;
use crate::tool::{ParamStruct, ResultStruct, ToolDef, ToolResult};

/// Context passed to all handlers containing service, request, and MCP context
#[derive(Clone)]
pub struct HandlerContext {
    pub(super) tool_def: ToolDef,
    pub request:         CallToolRequestParam,
    pub roots:           Vec<PathBuf>,
}

impl HandlerContext {
    /// Create a new `HandlerContext`
    pub(crate) const fn new(
        tool_def: ToolDef,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            tool_def,
            request,
            roots,
        }
    }

    /// Get tool definition by looking up the request name in the service's tool registry
    ///
    /// # Errors
    /// Returns an error if the tool definition is not found.
    pub const fn tool_def(&self) -> &ToolDef {
        &self.tool_def
    }

    /// Common parameter extraction methods (used by both BRP and local handlers)
    pub fn extract_parameter_values<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Get request arguments as JSON Value
        // Special case: if T is unit type, use null instead of empty object
        let args_value = if std::any::type_name::<T>() == "()" {
            serde_json::Value::Null
        } else {
            self.request.arguments.as_ref().map_or_else(
                || serde_json::Value::Object(serde_json::Map::new()),
                |args| serde_json::Value::Object(args.clone()),
            )
        };

        serde_json::from_value(args_value).map_err(|e| {
            tracing::debug!("Serde deserialization error: {}", e);
            error_stack::Report::new(Error::ParameterExtraction(format!(
                "Failed to extract parameters for type: {}",
                std::any::type_name::<T>()
            )))
            .attach("Parameter validation failed")
            .attach(format!("Expected type: {}", std::any::type_name::<T>()))
            .attach(format!("Serde error: {e}"))
        })
    }

    /// Get a field value from the request arguments
    pub fn extract_optional_named_field(&self, field_name: &str) -> Option<&Value> {
        self.request.arguments.as_ref()?.get(field_name)
    }

    /// Format a tool result into a `CallToolResult`
    pub fn format_result<T, P>(&self, tool_result: ToolResult<T, P>) -> CallToolResult
    where
        T: ResultStruct,
        P: ParamStruct,
    {
        let tool_name = self.tool_def.tool_name;
        let call_info = tool_name.get_call_info();

        match tool_result.result {
            Ok(data) => {
                let response =
                    match Response::success(&data, tool_result.params, call_info.clone(), self) {
                        Ok(response) => response,
                        Err(report) => {
                            return Response::error_message(
                                format!("Internal error: {}", report.current_context()),
                                call_info,
                            )
                            .to_call_tool_result();
                        }
                    };

                // Handle large response here with access to tool_name
                match self.handle_large_response_if_needed(response) {
                    Ok(processed) => processed.to_call_tool_result(),
                    Err(e) => Response::error_message(
                        format!("Failed to process response: {}", e.current_context()),
                        call_info,
                    )
                    .to_call_tool_result(),
                }
            }
            Err(report) => match report.current_context() {
                Error::Structured { result } => {
                    // Create error response from structured result
                    match Response::error(
                        result.as_ref(),
                        tool_result.params,
                        call_info.clone(),
                        self,
                    ) {
                        Ok(response) => response.to_call_tool_result(),
                        Err(e) => Response::error_message(
                            format!("Failed to create error response: {}", e.current_context()),
                            call_info,
                        )
                        .to_call_tool_result(),
                    }
                }
                Error::ToolCall { message, details } => {
                    // Create error response with the error message and details
                    Response::error_with_details(message, details.as_ref(), call_info)
                        .to_call_tool_result()
                }
                _ => Response::error_message(
                    format!("Internal error: {}", report.current_context()),
                    call_info,
                )
                .to_call_tool_result(),
            },
        }
    }

    /// Format framework errors
    pub fn format_framework_error(
        &self,
        error: error_stack::Report<crate::error::Error>,
    ) -> CallToolResult {
        let tool_name = self.tool_def.tool_name;
        let call_info = tool_name.get_call_info();

        Response::error_message(
            format!("Framework error: {}", error.current_context()),
            call_info,
        )
        .to_call_tool_result()
    }

    /// Handle large responses if needed
    fn handle_large_response_if_needed(
        &self,
        response: ToolCallJsonResponse,
    ) -> Result<ToolCallJsonResponse> {
        let config = LargeResponseConfig::default();

        // Check size and handle
        let response_json = serde_json::to_string(&response)
            .change_context(Error::General("Failed to serialize response".to_string()))?;
        let estimated_tokens = response_json.len() / CHARS_PER_TOKEN;

        if estimated_tokens > config.max_tokens
            && let Some(result_field) = &response.result
        {
            // Generate filename using self.tool_def.tool_name
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .change_context(Error::General("Failed to get timestamp".to_string()))?
                .as_secs();

            let sanitized_identifier = self.tool_def.tool_name.to_string().replace(['/', ' '], "_");
            let filename = format!(
                "{}{}{}.json",
                config.file_prefix, sanitized_identifier, timestamp
            );

            let filepath = config.temp_dir.join(&filename);

            let result_json = serde_json::to_string_pretty(result_field).change_context(
                Error::General("Failed to serialize result field".to_string()),
            )?;

            fs::write(&filepath, &result_json).change_context(Error::FileOperation(format!(
                "Failed to write result to {}",
                filepath.display()
            )))?;

            let mut modified_response = response;
            modified_response.result = Some(json!({
                "saved_to_file": true,
                "filepath": filepath.to_string_lossy(),
                "instructions": "Use Read tool to examine, Grep to search, or jq commands to filter the data.",
                "original_size_tokens": estimated_tokens
            }));

            return Ok(modified_response);
        }

        Ok(response)
    }
}
