use rmcp::ErrorData as McpError;
use rmcp::model::CallToolResult;
use serde_json::Value;

use super::ResponseFieldName;
use super::builder::{CallInfo, CallInfoProvider, JsonResponse, ResponseBuilder};
use super::components::ResponseComponents;
use super::field_placement_traits::{FieldAccessor, ResponseData};
use super::large_response::{self, LargeResponseConfig};
use crate::error::{Error, Result};
use crate::tool::{HandlerContext, ParameterName};

/// Specifies where a response field should be placed in the output JSON
#[derive(Clone, Debug)]
pub enum FieldPlacement {
    /// Place field in the metadata object
    Metadata,
    /// Place field in the result object
    Result,
}

/// Response field specification for structured responses.
///
/// Defines how to extract and place fields in the response JSON structure.
#[derive(Clone, Debug)]
pub enum ResponseField {
    /// Reference a field from already-extracted request parameters with explicit placement.
    ///
    /// This variant references data that was already extracted and validated during
    /// the parameter extraction phase, with explicit control over where the field is placed.
    FromRequest {
        /// Name of the field to be output in the response
        response_field_name: ResponseFieldName,
        /// Parameter name from the tool call request parameters
        parameter_name:      ParameterName,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Extract a field from response data with explicit placement.
    ///
    /// This variant specifies extraction of data from the handler or BRP response payload
    /// with explicit control over where the field is placed.
    FromResponse {
        /// Name of the field in the response
        response_field_name: ResponseFieldName,
        /// Source path for extraction
        /// Supports dot notation for nested fields (e.g., "result.entity")
        source_path:         &'static str,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Pass all fields from the BRP response directly to the metadata field.
    ///
    /// This variant takes all top-level fields from the response and places them
    /// in metadata, useful for tools that return many fields that all belong in metadata.
    DirectToMetadata,
    /// Extract a field from response data that may be null - skip if null.
    ///
    /// This variant extracts a field and omits it from the response if the value is null.
    /// Use this for optional fields that should not appear in the response when missing.
    FromResponseNullableWithPlacement {
        /// Name of the field in the response
        response_field_name: ResponseFieldName,
        /// Source path for extraction
        /// Supports dot notation for nested fields (e.g., "result.entity")
        source_path:         &'static str,
        /// Where to place this field in the response
        placement:           FieldPlacement,
    },
    /// Extract the raw BRP response data from the "result" field to the result field
    ///
    /// This is a convenience variant for BRP tools that need to extract the raw BRP response
    /// from the "result" field and place it in the JSON response result field.
    BrpRawResultToResult,
    /// Extract format correction metadata from handler responses
    ///
    /// This variant extracts all format correction fields (`format_corrected`,
    /// `format_corrections`, etc.) from `BrpMethodResult` and places them in metadata. Only
    /// used for V2 tools that support format correction.
    FormatCorrection,
}

impl ResponseField {
    /// Get the field name for this response field specification.
    pub fn name(&self) -> &'static str {
        match self {
            Self::FromRequest {
                response_field_name: name,
                ..
            }
            | Self::FromResponse {
                response_field_name: name,
                ..
            }
            | Self::FromResponseNullableWithPlacement {
                response_field_name: name,
                ..
            } => name.into(),
            Self::DirectToMetadata | Self::FormatCorrection => ResponseFieldName::Metadata.into(),
            Self::BrpRawResultToResult => ResponseFieldName::Result.into(),
        }
    }
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

    /// Type-safe formatter for types that implement FieldAccessor
    /// This allows direct field access without JSON serialization
    pub fn format_result_with_field_access<T, C>(
        self,
        result: Result<T>,
        handler_context: &HandlerContext,
        call_info_data: C,
    ) -> std::result::Result<CallToolResult, McpError>
    where
        T: FieldAccessor + serde::Serialize,
        C: CallInfoProvider,
    {
        let call_info = call_info_data.to_call_info(handler_context.request.name.to_string());

        match result {
            Ok(data) => {
                // Use field accessor to build response
                let components =
                    ResponseComponents::from_field_accessor(&self, handler_context, &data);

                // Build response with clean method chaining
                let builder = ResponseBuilder::success(call_info)
                    .apply_format_corrections(&components)
                    .map_err(|e| {
                        McpError::internal_error(
                            format!("Failed to apply format corrections: {e}"),
                            None,
                        )
                    })?
                    .apply_configured_fields(&components)
                    .map_err(|e| {
                        McpError::internal_error(
                            format!("Failed to apply configured fields: {e}"),
                            None,
                        )
                    })?
                    .message(&components.final_message)
                    .auto_inject_debug_info(components.debug_info.as_ref());

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

    // fn format_success<T>(
    //     &self,
    //     data: T,
    //     handler_context: &HandlerContext,
    //     call_info: CallInfo,
    // ) -> std::result::Result<CallToolResult, McpError>
    // where
    //     T: FieldAccessor,
    // {
    //     // Use field accessor for typed extraction - no JSON serialization
    //     let response = self
    //         .build_success_response(&data, handler_context, call_info)
    //         .map_err(|e| {
    //             McpError::internal_error(format!("Failed to build success response: {e}"), None)
    //         })?;

    //     Ok(Self::handle_large_response(
    //         response,
    //         &handler_context.request.name,
    //     ))
    // }

    fn build_success_response(
        &self,
        data: &dyn FieldAccessor,
        handler_context: &HandlerContext,
        call_info: CallInfo,
    ) -> Result<JsonResponse> {
        // Extract all components using field accessor - no JSON
        let components = ResponseComponents::from_field_accessor(&self, handler_context, data);

        // Build response with clean method chaining
        let builder = ResponseBuilder::success(call_info)
            .apply_format_corrections(&components)?
            .apply_configured_fields(&components)?
            .message(&components.final_message)
            .auto_inject_debug_info(components.debug_info.as_ref());

        Ok(builder.build())
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
                let placeholder_str = format!("{{{}}}", placeholder);
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
