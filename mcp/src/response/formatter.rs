//! # Response Handling Module
//!
//! This module provides different APIs for handling responses in the Bevy BRP MCP server.
//! There are three main approaches, each suited for different use cases:
//!
//! ## `ResponseBuilder` API (`ResponseBuilder`)
//!
//! A flexible builder pattern for constructing responses:
//! - Allows setting message, data, and individual fields
//! - Supports auto-injection of debug information
//! - Provides error handling for serialization issues
//!
//! **Use when:** You need to build responses with multiple fields or complex data structures.
//!
//! ## `ResponseFormatter` API (`ResponseFormatter`)
//!
//! A configurable formatter that handles BRP-specific concerns:
//! - Automatic large response handling with file fallback
//! - Template-based message formatting with parameter substitution
//! - Format correction status handling
//! - Configurable field extraction from response data
//!
//! **Use when:** You need BRP-specific features like large response handling or template
//! formatting.

use std::sync::Arc;

use rmcp::model::CallToolResult;
use serde_json::{Value, json};

use super::builder::{JsonResponse, ResponseBuilder};
use super::field_extractor::FieldExtractor;
use crate::brp_tools::constants::{
    JSON_FIELD_BRP_CALL_INFO, JSON_FIELD_DEBUG_INFO, JSON_FIELD_ERROR_CODE,
    JSON_FIELD_FORMAT_CORRECTED, JSON_FIELD_METHOD, JSON_FIELD_PORT,
};
use crate::brp_tools::support::brp_client::BrpError;
use crate::error::Result;
use crate::support::{LargeResponseConfig, handle_large_response};
use crate::tool::BrpToolCallInfo;

/// Context passed to formatter factory
#[derive(Clone)]
pub struct FormatterContext {
    /// Parameters with defaults applied (e.g., port always present)
    pub params:           Option<Value>,
    pub format_corrected: Option<crate::brp_tools::request_handler::FormatCorrectionStatus>,
}

/// Default error formatter implementation with rich guidance extraction
pub fn format_error_default(error: BrpError, metadata: &BrpToolCallInfo) -> CallToolResult {
    // Extract rich guidance from format_corrections if present
    let rich_guidance = extract_rich_guidance_from_error(&error);

    build_enhanced_error_response(&error, metadata, rich_guidance).map_or_else(
        |_| {
            let fallback = ResponseBuilder::error()
                .message("Failed to build error response")
                .build();
            fallback.to_call_tool_result()
        },
        |response| response.to_call_tool_result(),
    )
}

/// Extract rich guidance fields from `format_corrections` in error data
fn extract_rich_guidance_from_error(error: &BrpError) -> Option<RichGuidance> {
    let error_data = error.data.as_ref()?;
    let format_corrections = error_data.get("format_corrections")?.as_array()?;

    if format_corrections.is_empty() {
        return None;
    }

    // Use the first correction as primary guidance
    let correction = &format_corrections[0];

    // Extract examples from corrected_format
    let examples = correction
        .get("corrected_format")
        .and_then(|cf| cf.get("examples"))
        .and_then(|e| e.as_array())
        .cloned()
        .or_else(|| {
            // Try to extract examples from the correction itself
            correction
                .get("examples")
                .and_then(|e| e.as_array())
                .cloned()
        });

    // Extract hint from format corrections
    let hint = correction
        .get("corrected_format")
        .and_then(|cf| cf.get("hint"))
        .and_then(|h| h.as_str())
        .map(std::string::ToString::to_string)
        .or_else(|| {
            // Try to extract hint from the correction itself
            correction
                .get("hint")
                .and_then(|h| h.as_str())
                .map(std::string::ToString::to_string)
        });

    // Extract valid_values
    let valid_values = correction
        .get("corrected_format")
        .and_then(|cf| cf.get("valid_values"))
        .and_then(|vv| vv.as_array())
        .cloned()
        .or_else(|| {
            // Try to extract valid_values from the correction itself
            correction
                .get("valid_values")
                .and_then(|vv| vv.as_array())
                .cloned()
        });

    // Only return guidance if we have at least one rich field
    if examples.is_some() || hint.is_some() || valid_values.is_some() {
        Some(RichGuidance {
            examples,
            hint,
            valid_values,
        })
    } else {
        None
    }
}

/// Rich guidance extracted from format corrections
#[derive(Debug, Clone)]
struct RichGuidance {
    examples:     Option<Vec<Value>>,
    hint:         Option<String>,
    valid_values: Option<Vec<Value>>,
}

fn build_enhanced_error_response(
    error: &BrpError,
    metadata: &BrpToolCallInfo,
    rich_guidance: Option<RichGuidance>,
) -> Result<JsonResponse> {
    let mut builder = ResponseBuilder::error()
        .message(&error.message)
        .add_field(JSON_FIELD_ERROR_CODE, error.code)?
        .add_field(
            JSON_FIELD_BRP_CALL_INFO,
            json!({
                JSON_FIELD_METHOD: metadata.tool_name,
                JSON_FIELD_PORT: metadata.port
            }),
        )?;

    // Add rich guidance fields if available (flat structure, not nested)
    if let Some(guidance) = rich_guidance {
        if let Some(examples) = guidance.examples {
            builder = builder.add_field("examples", &examples)?;
        }
        if let Some(hint) = guidance.hint {
            builder = builder.add_field("hint", &hint)?;
        }
        if let Some(valid_values) = guidance.valid_values {
            builder = builder.add_field("valid_values", &valid_values)?;
        }
    }

    // Include remaining error data (excluding format_corrections to avoid duplication)
    if let Some(data) = &error.data {
        if let Some(data_obj) = data.as_object() {
            for (key, value) in data_obj {
                // Skip format_corrections since we've extracted guidance from it
                // Also skip metadata to avoid duplication
                // Also skip status to avoid redundancy with top-level status
                if key != "format_corrections" && key != "metadata" && key != "status" {
                    builder = builder.add_field(key, value)?;
                }
            }
        }
    }

    Ok(builder.build())
}

/// A configurable formatter that can handle various BRP response formatting needs
pub struct ResponseFormatter {
    config:  Arc<FormatterConfig>,
    context: FormatterContext,
}

/// Configuration for the configurable formatter
pub struct FormatterConfig {
    /// Template for success messages - can include placeholders like {entity}, {resource}, etc.
    pub success_template:      Option<String>,
    /// Additional fields to add to success responses
    pub success_fields:        Vec<(String, FieldExtractor)>,
    /// Configuration for large response handling
    pub large_response_config: Option<LargeResponseConfig>,
}

impl ResponseFormatter {
    #[allow(clippy::missing_const_for_fn)] // False positive - Arc doesn't support const construction
    pub fn new(config: Arc<FormatterConfig>, context: FormatterContext) -> Self {
        Self { config, context }
    }

    pub fn format_success(&self, data: &Value, metadata: BrpToolCallInfo) -> CallToolResult {
        // Check if this is a passthrough formatter - if so, convert data directly to CallToolResult
        if self.is_passthrough_formatter() {
            return Self::format_passthrough_response(self, data);
        }

        // Check if this is a raw result formatter - if so, put BRP data in result field
        if self.is_raw_result_formatter() {
            return self.format_raw_result_response(data, metadata);
        }

        // First build the response
        let response_result = self.build_success_response(data);

        if let Ok(response) = response_result {
            self.handle_large_response_if_needed(response, &metadata.tool_name)
        } else {
            let fallback = ResponseBuilder::error()
                .message("Failed to build success response")
                .build();
            fallback.to_call_tool_result()
        }
    }

    /// Check if this formatter is configured as a passthrough formatter
    fn is_passthrough_formatter(&self) -> bool {
        self.config.success_template.is_none()
            && self.config.success_fields.is_empty()
            && self.config.large_response_config.is_none()
    }

    /// Check if this formatter is configured as a raw result formatter
    fn is_raw_result_formatter(&self) -> bool {
        // Raw result formatters have no success fields and have large response config
        // They may have a template for the message
        self.config.success_fields.is_empty()
            && self.config.large_response_config.is_some()
            && self
                .config
                .large_response_config
                .as_ref()
                .is_some_and(|config| config.file_prefix == "brp_response_")
    }

    /// Format a passthrough response by converting the data directly to `CallToolResult`
    fn format_passthrough_response(_self: &Self, data: &Value) -> CallToolResult {
        // For passthrough, the data should already be a structured response
        // Convert it directly to CallToolResult as JSON content
        let json_string = serde_json::to_string_pretty(data).unwrap_or_else(|_| {
            r#"{"status":"error","message":"Failed to serialize passthrough response"}"#.to_string()
        });

        CallToolResult::success(vec![rmcp::model::Content::text(json_string)])
    }

    /// Format a raw result response by putting BRP data in the result field
    fn format_raw_result_response(
        &self,
        data: &Value,
        metadata: BrpToolCallInfo,
    ) -> CallToolResult {
        // Build response with BRP data in result field
        let mut builder = ResponseBuilder::success();

        // Set template message if provided
        if let Some(template) = &self.config.success_template {
            let template_params = self.context.params.as_ref();
            let message = substitute_template(template, template_params);
            builder = builder.message(message);
        }

        // Clean the data to remove any format_corrections before putting in result
        let mut clean_data = data.clone();
        let format_corrections_to_add = if let Value::Object(data_map) = data {
            // Check if format corrections exist and are non-empty
            data_map
                .get("format_corrections")
                .and_then(|format_corrections| {
                    if !format_corrections.is_null()
                        && format_corrections.is_array()
                        && format_corrections
                            .as_array()
                            .is_some_and(|arr| !arr.is_empty())
                    {
                        Some(format_corrections.clone())
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        // Remove format_corrections from the clean data
        if let Value::Object(data_map) = &mut clean_data {
            data_map.remove("format_corrections");
        }

        // For raw responses, put the entire BRP response in the result field
        builder = match builder.with_result(&clean_data) {
            Ok(b) => b,
            Err(_) => {
                return ResponseBuilder::error()
                    .message("Failed to set result field")
                    .build()
                    .to_call_tool_result();
            }
        };

        // Only add metadata if there are format corrections
        if let Some(format_corrections) = format_corrections_to_add {
            // Try to add both fields, but if it fails, continue without metadata
            if let Ok(b) = builder
                .add_metadata_field("format_corrected", "attempted")
                .and_then(|b| b.add_metadata_field("format_corrections", &format_corrections))
            {
                builder = b;
            } else {
                // If adding metadata failed, recreate the builder with just the result
                builder = ResponseBuilder::success();
                if let Some(template) = &self.config.success_template {
                    let template_params = self.context.params.as_ref();
                    let message = substitute_template(template, template_params);
                    builder = builder.message(message);
                }
                builder = match builder.with_result(&clean_data) {
                    Ok(b) => b,
                    Err(_) => {
                        return ResponseBuilder::error()
                            .message("Failed to set result field")
                            .build()
                            .to_call_tool_result();
                    }
                };
            }
        }

        let response = builder.build();
        self.handle_large_response_if_needed(response, &metadata.tool_name)
    }

    /// Handle large response processing if configured
    fn handle_large_response_if_needed(
        &self,
        response: JsonResponse,
        method: &str,
    ) -> CallToolResult {
        // Check if we need to handle large response
        self.config.large_response_config.as_ref().map_or_else(
            || response.to_call_tool_result(),
            |large_config| {
                // We need to check the size of the actual JSON that will be sent to MCP
                // This is what to_call_tool_result() will serialize
                let final_json = response.to_json_fallback();
                let response_value =
                    serde_json::from_str::<Value>(&final_json).unwrap_or(Value::Null);

                // Check if response is too large
                match handle_large_response(&response_value, method, large_config.clone()) {
                    Ok(Some(fallback_response)) => {
                        // Response was too large and saved to file
                        ResponseBuilder::success()
                            .message(
                                fallback_response["message"]
                                    .as_str()
                                    .unwrap_or("Response saved to file"),
                            )
                            .add_field("filepath", &fallback_response["filepath"])
                            .unwrap_or_else(|_| ResponseBuilder::success())
                            .add_field("instructions", &fallback_response["instructions"])
                            .unwrap_or_else(|_| ResponseBuilder::success())
                            .build()
                            .to_call_tool_result()
                    }
                    Ok(None) => {
                        // Response is small enough, return as-is
                        response.to_call_tool_result()
                    }
                    Err(e) => {
                        tracing::warn!("Error handling large response: {:?}", e);
                        // Error handling large response, return original
                        response.to_call_tool_result()
                    }
                }
            },
        )
    }

    fn build_success_response(&self, data: &Value) -> Result<JsonResponse> {
        let mut builder = ResponseBuilder::success();

        // Collect extracted field values for template substitution
        let mut template_values = serde_json::Map::new();

        // Add original params to template values
        if let Some(Value::Object(params)) = &self.context.params {
            template_values.extend(params.clone());
        }

        // Extract debug info and format corrections from data first
        let mut clean_data = data.clone();
        let mut brp_extras_debug_info = None;

        if let Value::Object(data_map) = data {
            // Extract brp_extras_debug_info from data (if exists)
            if let Some(debug_info) = data_map.get(JSON_FIELD_DEBUG_INFO) {
                if !debug_info.is_null() && (debug_info.is_array() || debug_info.is_string()) {
                    brp_extras_debug_info = Some(debug_info.clone());
                }
            }

            // Always preserve format_corrections from the input data
            if let Some(format_corrections) = data_map.get("format_corrections") {
                if !format_corrections.is_null() && format_corrections.is_array() {
                    builder = builder.add_field("format_corrections", format_corrections)?;
                }
            }

            // Add format_corrected from context if present
            if let Some(ref format_corrected) = self.context.format_corrected {
                let format_corrected_value =
                    serde_json::to_value(format_corrected).map_err(|e| {
                        error_stack::Report::new(crate::error::Error::General(format!(
                            "Failed to serialize format_corrected: {e}"
                        )))
                    })?;
                builder =
                    builder.add_field(JSON_FIELD_FORMAT_CORRECTED, &format_corrected_value)?;
            }

            // Clean debug_info from data to prevent duplication
            if let Value::Object(clean_map) = &mut clean_data {
                clean_map.remove(JSON_FIELD_DEBUG_INFO);
            }
        }

        // Add configured fields and collect their values for template substitution (using clean
        // data)
        for (field_name, extractor) in &self.config.success_fields {
            let value = extractor(&clean_data, &self.context);
            builder = builder.add_field(field_name, &value)?;

            // Add extracted value to template substitution map
            template_values.insert(field_name.to_string(), value);
        }

        // Apply success template if provided (after collecting all field values)
        if let Some(template) = &self.config.success_template {
            // Check if template contains placeholders that aren't in template_values
            // If so, try to find them in the response data
            let mut final_template_values = template_values.clone();

            // Simple regex-free placeholder detection
            let mut remaining = template.as_str();
            while let Some(start) = remaining.find('{') {
                if let Some(end) = remaining[start + 1..].find('}') {
                    let placeholder = &remaining[start + 1..start + 1 + end];

                    // If this placeholder isn't in our values, check response data
                    if !final_template_values.contains_key(placeholder) {
                        if let Value::Object(data_map) = &clean_data {
                            if let Some(value) = data_map.get(placeholder) {
                                final_template_values
                                    .insert(placeholder.to_string(), value.clone());
                            }
                        }
                    }

                    remaining = &remaining[start + 1 + end + 1..];
                } else {
                    break;
                }
            }

            let template_params = Value::Object(final_template_values);
            let message = substitute_template(template, Some(&template_params));
            builder = builder.message(message);
        }

        // Override message if format correction occurred
        if self.context.format_corrected
            == Some(crate::brp_tools::request_handler::FormatCorrectionStatus::Succeeded)
        {
            builder = builder.message("Request succeeded with format correction applied");
        }

        // Auto-inject debug info at response level if debug mode is enabled
        builder = builder.auto_inject_debug_info(brp_extras_debug_info.as_ref());

        Ok(builder.build())
    }
}

/// Factory for creating configurable formatters
pub struct ResponseFormatterFactory {
    config: Arc<FormatterConfig>,
}

impl ResponseFormatterFactory {
    /// Create a standard formatter with common configuration
    pub fn standard() -> ResponseFormatterBuilder {
        ResponseFormatterBuilder {
            config: FormatterConfig {
                success_template:      None,
                success_fields:        vec![],
                large_response_config: Some(LargeResponseConfig {
                    file_prefix: "brp_response_".to_string(),
                    ..Default::default()
                }),
            },
        }
    }

    /// Create a formatter for Raw BRP responses that go directly to result field
    pub fn raw_result() -> ResponseFormatterBuilder {
        ResponseFormatterBuilder {
            config: FormatterConfig {
                success_template:      None,
                success_fields:        vec![],
                large_response_config: Some(LargeResponseConfig {
                    file_prefix: "brp_response_".to_string(),
                    ..Default::default()
                }),
            },
        }
    }
}

impl ResponseFormatterFactory {
    pub fn create(&self, context: FormatterContext) -> ResponseFormatter {
        ResponseFormatter::new(Arc::clone(&self.config), context)
    }
}

/// Builder for configuring formatters
pub struct ResponseFormatterBuilder {
    config: FormatterConfig,
}

impl ResponseFormatterBuilder {
    /// Set the success message template
    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.config.success_template = Some(template.into());
        self
    }

    /// Add a field to the success response
    pub fn with_response_field(
        mut self,
        name: impl Into<String>,
        extractor: FieldExtractor,
    ) -> Self {
        self.config.success_fields.push((name.into(), extractor));
        self
    }

    /// Build the formatter factory
    pub fn build(self) -> ResponseFormatterFactory {
        ResponseFormatterFactory {
            config: Arc::new(self.config),
        }
    }
}

/// Substitute placeholders in a template string with values from params
fn substitute_template(template: &str, params: Option<&Value>) -> String {
    let mut result = template.to_string();

    if let Some(Value::Object(map)) = params {
        for (key, value) in map {
            let placeholder = format!("{{{key}}}");
            let replacement = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => value.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
    }

    result
}

// Response size estimation functions

// Common field extractors

// Helper functions for common field extractors

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::brp_tools::constants::DEFAULT_BRP_PORT;

    #[test]
    fn test_substitute_template() {
        let params = Some(json!({
            "entity": 123,
            "name": "test_resource"
        }));

        let result = substitute_template("Entity {entity} with name {name}", params.as_ref());
        assert_eq!(result, "Entity 123 with name test_resource");

        let result = substitute_template("No substitutions", params.as_ref());
        assert_eq!(result, "No substitutions");

        let result = substitute_template("Missing {missing} placeholder", params.as_ref());
        assert_eq!(result, "Missing {missing} placeholder");
    }

    #[test]
    fn test_configurable_formatter_success() {
        use crate::brp_tools::constants::JSON_FIELD_DESTROYED_ENTITY;

        let config = FormatterConfig {
            success_template:      Some("Successfully destroyed entity {entity}".to_string()),
            success_fields:        vec![(
                JSON_FIELD_DESTROYED_ENTITY.to_string(),
                Box::new(|_data, context| {
                    use crate::extractors::McpCallExtractor;
                    context
                        .params
                        .as_ref()
                        .and_then(|params| {
                            // Create a temporary request to use the extractor
                            if let Value::Object(args) = params {
                                let request = rmcp::model::CallToolRequestParam {
                                    arguments: Some(args.clone()),
                                    name:      String::new().into(),
                                };
                                let extractor = McpCallExtractor::from_request(&request);
                                extractor.entity_id().map(|id| Value::Number(id.into()))
                            } else {
                                None
                            }
                        })
                        .unwrap_or(Value::Null)
                }),
            )],
            large_response_config: None,
        };

        let context = FormatterContext {
            params:           Some(json!({ "entity": 123 })),
            format_corrected: None,
        };

        let formatter = ResponseFormatter::new(Arc::new(config), context);
        let metadata = BrpToolCallInfo::new("bevy/destroy", DEFAULT_BRP_PORT);
        let result = formatter.format_success(&Value::Null, metadata);

        // Verify result has content
        assert_eq!(result.content.len(), 1);
        // For now, we'll just verify that formatting doesn't panic
        // TODO: Once we understand Content type better, add proper content validation
    }

    #[test]
    fn test_enhanced_error_formatting_direct() {
        let metadata = BrpToolCallInfo::new("bevy/destroy", DEFAULT_BRP_PORT);
        let error = BrpError {
            code:    -32603,
            message: "Entity not found".to_string(),
            data:    None,
        };

        let result = format_error_default(error, &metadata);

        // Verify result has content
        assert_eq!(result.content.len(), 1);
        // All errors now use the enhanced format_error_default function
    }
}
