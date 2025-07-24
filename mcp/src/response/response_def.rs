use rmcp::ErrorData as McpError;
use rmcp::model::CallToolResult;
use serde_json::Value;

use super::builder::{CallInfoProvider, JsonResponse, ResponseBuilder};
use super::field_placement::ResponseData;
use super::large_response::{self, LargeResponseConfig};
use crate::error::{Error, Result};
use crate::tool::HandlerContext;

/// Specifies where a response field should be placed in the output JSON
#[derive(Clone, Debug)]
pub enum FieldPlacement {
    /// Place field in the metadata object
    Metadata,
    /// Place field in the result object
    Result,
}

/// Defines how to format the response for a tool.
///
/// Specifies the message template for responses.
#[derive(Clone)]
pub struct ResponseDef {
    /// Template for success messages
    pub message_template: &'static str,
}

impl ResponseDef {
    /// Type-safe formatter that accepts our internal Result directly
    pub fn format_result<T, C>(
        self,
        result: Result<T>,
        handler_context: &HandlerContext,
        call_info_data: C,
    ) -> std::result::Result<CallToolResult, McpError>
    where
        T: ResponseData,
        C: CallInfoProvider,
    {
        let call_info = call_info_data.to_call_info(handler_context.request.name.to_string());

        match result {
            Ok(data) => {
                // Build response using ResponseData trait
                let builder = ResponseBuilder::success(call_info);
                let builder = data.add_response_fields(builder).map_err(|e| {
                    McpError::internal_error(format!("Failed to add response fields: {e}"), None)
                })?;

                // Perform template substitution
                let message = self.substitute_template(&builder, handler_context);
                let builder = builder.message(message);

                let response = builder.build();
                Ok(Self::handle_large_response(
                    response,
                    &handler_context.request.name,
                ))
            }
            Err(report) => match report.current_context() {
                Error::ToolCall { message, details } => Ok(ResponseBuilder::error(call_info)
                    .message(message)
                    .add_optional_details(details.as_ref())
                    .build()
                    .to_call_tool_result()),
                _ => Ok(ResponseBuilder::error(call_info)
                    .message(format!("Internal error: {}", report.current_context()))
                    .build()
                    .to_call_tool_result()),
            },
        }
    }

    /// Substitute template placeholders with values from the builder
    fn substitute_template(
        &self,
        builder: &ResponseBuilder,
        handler_context: &HandlerContext,
    ) -> String {
        let mut result = self.message_template.to_string();

        // Extract placeholders from template
        let placeholders = self.parse_template_placeholders(&result);

        for placeholder in placeholders {
            if let Some(replacement) =
                self.find_placeholder_value(&placeholder, builder, handler_context)
            {
                let placeholder_str = format!("{{{placeholder}}}");
                result = result.replace(&placeholder_str, &replacement);
            }
        }

        result
    }

    /// Parse template to find placeholder names
    fn parse_template_placeholders(&self, template: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut remaining = template;

        while let Some(start) = remaining.find('{') {
            if let Some(end) = remaining[start + 1..].find('}') {
                let placeholder = &remaining[start + 1..start + 1 + end];
                if !placeholder.is_empty() {
                    placeholders.push(placeholder.to_string());
                }
                remaining = &remaining[start + 1 + end + 1..];
            } else {
                break;
            }
        }

        placeholders
    }

    /// Find value for a placeholder
    fn find_placeholder_value(
        &self,
        placeholder: &str,
        builder: &ResponseBuilder,
        handler_context: &HandlerContext,
    ) -> Option<String> {
        // First check metadata
        if let Some(Value::Object(metadata)) = builder.metadata() {
            if let Some(value) = metadata.get(placeholder) {
                return Some(self.value_to_string(value));
            }
        }

        // Then check result if placeholder is "result"
        if placeholder == "result" {
            if let Some(result_value) = builder.result() {
                return Some(self.value_to_string(result_value));
            }
        }

        // Finally check request parameters
        if let Some(value) = handler_context.extract_optional_named_field(placeholder) {
            return Some(self.value_to_string(value));
        }

        None
    }

    /// Convert value to string for template substitution
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => format!("{} items", arr.len()),
            _ => value.to_string(),
        }
    }

    /// Handle large response processing if configured
    fn handle_large_response(response: JsonResponse, method: &str) -> CallToolResult {
        // Check if response is too large and handle result field extraction
        match large_response::handle_large_response(
            response,
            method,
            LargeResponseConfig::default(),
        ) {
            Ok(processed_response) => {
                // Return the processed response (either original or with result field saved to
                // file)
                processed_response.to_call_tool_result()
            }
            Err(e) => {
                tracing::warn!("Error handling large response: {:?}", e);
                // Error handling the large response, return error response
                ResponseBuilder::error(crate::response::CallInfo::local(
                    "large_response_error".to_string(),
                ))
                .message("Error processing large response")
                .build()
                .to_call_tool_result()
            }
        }
    }
}
