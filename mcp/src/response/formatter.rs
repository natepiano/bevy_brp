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
//! ## `ResponseFormatter` API (`ResponseFormatter`)
//!
//! A configurable formatter that handles BRP-specific concerns:
//! - Automatic large response handling with file fallback
//! - Template-based message formatting with parameter substitution
//! - Format correction status handling
//! - Configurable field extraction from response data

use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

use super::builder::{JsonResponse, ResponseBuilder};
use super::field_extractor::FieldExtractor;
use super::large_response::{self, LargeResponseConfig};
use super::specification::FieldPlacement;
// Import format discovery types for convenience
use crate::brp_tools::request_handler::{
    FORMAT_DISCOVERY_METHODS, FormatCorrection, FormatCorrectionStatus,
};
use crate::constants::{
    JSON_FIELD_DEBUG_INFO, JSON_FIELD_FORMAT_CORRECTED, JSON_FIELD_FORMAT_CORRECTIONS,
    JSON_FIELD_METADATA,
};
use crate::error::Result;
use crate::service::{HandlerContext, HasCallInfo};

/// A configurable formatter that can handle various BRP response formatting needs
pub struct ResponseFormatter {
    config: FormatterConfig,
}

/// Configuration for the configurable formatter
pub struct FormatterConfig {
    /// Template for success messages - can include placeholders like {entity}, {resource}, etc.
    pub success_template:      Option<String>,
    /// Additional fields to add to success responses
    pub success_fields:        Vec<(String, FieldExtractor, FieldPlacement)>,
    /// Configuration for large response handling
    pub large_response_config: LargeResponseConfig,
}

impl ResponseFormatter {
    pub const fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    pub fn format_success_with_corrections<T>(
        &self,
        data: &Value,
        handler_context: &HandlerContext<T>,
        format_corrections: Option<&[FormatCorrection]>,
        format_corrected: Option<&FormatCorrectionStatus>,
    ) -> CallToolResult
    where
        HandlerContext<T>: HasCallInfo,
    {
        // First build the response
        let response_result = self.build_success_response_with_corrections(
            data,
            handler_context,
            format_corrections,
            format_corrected,
        );
        response_result.map_or_else(
            |_| {
                let fallback = ResponseBuilder::error(handler_context.call_info())
                    .message("Failed to build success response")
                    .build();
                fallback.to_call_tool_result()
            },
            |response| self.handle_large_response(response, &handler_context.request.name),
        )
    }

    /// Handle large response processing if configured
    fn handle_large_response(&self, response: JsonResponse, method: &str) -> CallToolResult {
        // Check if response is too large and handle result field extraction
        match large_response::handle_large_response(
            response,
            method,
            self.config.large_response_config.clone(),
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

    fn build_success_response_with_corrections<T>(
        &self,
        data: &Value,
        handler_context: &HandlerContext<T>,
        format_corrections: Option<&[FormatCorrection]>,
        format_corrected: Option<&FormatCorrectionStatus>,
    ) -> Result<JsonResponse>
    where
        HandlerContext<T>: HasCallInfo,
    {
        let type_name = std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        tracing::debug!(
            "build_success_response<{}>: response_fields count = {}",
            type_name,
            self.config.success_fields.len()
        );

        let call_info = handler_context.call_info();
        let mut builder = ResponseBuilder::success(call_info);
        let template_values = Self::initialize_template_values(handler_context);
        let (clean_data, brp_extras_debug_info) = Self::extract_debug_and_clean_data(data);

        Self::add_format_corrections(
            &mut builder,
            handler_context,
            format_corrections,
            format_corrected,
            None, // BRP method name not available in generic context
        )?;
        let template_values = self.add_configured_fields(
            &mut builder,
            &clean_data,
            template_values,
            handler_context,
        )?;
        self.apply_template_if_provided(&mut builder, &clean_data, &template_values);
        Self::override_message_for_format_correction(&mut builder, format_corrected);
        builder = builder.auto_inject_debug_info(brp_extras_debug_info.as_ref());

        let response = builder.build();
        tracing::trace!(
            "build_success_response<{}>: final response = {:?}",
            type_name,
            response
        );
        Ok(response)
    }

    /// Initialize template values with original parameters
    fn initialize_template_values<T>(
        handler_context: &HandlerContext<T>,
    ) -> serde_json::Map<String, Value> {
        let mut template_values = serde_json::Map::new();
        if let Some(params) = &handler_context.request.arguments {
            template_values.extend(params.clone());
        }
        template_values
    }

    /// Extract debug info and clean data from incoming response
    fn extract_debug_and_clean_data(data: &Value) -> (Value, Option<Value>) {
        let mut clean_data = data.clone();
        let mut brp_extras_debug_info = None;

        if let Value::Object(data_map) = data {
            if let Some(debug_info) = data_map.get(JSON_FIELD_DEBUG_INFO) {
                if !debug_info.is_null() && (debug_info.is_array() || debug_info.is_string()) {
                    brp_extras_debug_info = Some(debug_info.clone());
                }
            }

            if let Value::Object(clean_map) = &mut clean_data {
                clean_map.remove(JSON_FIELD_DEBUG_INFO);
            }
        }

        (clean_data, brp_extras_debug_info)
    }

    /// Add configured fields and collect their values for template substitution
    fn add_configured_fields<T>(
        &self,
        builder: &mut ResponseBuilder,
        clean_data: &Value,
        mut template_values: serde_json::Map<String, Value>,
        handler_context: &HandlerContext<T>,
    ) -> Result<serde_json::Map<String, Value>> {
        for (field_name, extractor, placement) in &self.config.success_fields {
            let value = extractor(clean_data, handler_context);

            if field_name == JSON_FIELD_METADATA && matches!(placement, FieldPlacement::Metadata) {
                if let Value::Object(data_map) = &value {
                    for (key, val) in data_map {
                        *builder = builder.clone().add_field(key, val)?;
                    }
                }
            } else {
                *builder = builder
                    .clone()
                    .add_field_to(field_name, &value, placement.clone())?;
            }

            template_values.insert(field_name.to_string(), value);
        }
        Ok(template_values)
    }

    /// Apply template substitution if template is provided
    fn apply_template_if_provided(
        &self,
        builder: &mut ResponseBuilder,
        clean_data: &Value,
        template_values: &serde_json::Map<String, Value>,
    ) {
        if let Some(template) = &self.config.success_template {
            let final_template_values =
                Self::resolve_template_placeholders(template, template_values, clean_data);
            let template_params = Value::Object(final_template_values);
            let message = substitute_template(template, Some(&template_params));
            *builder = builder.clone().message(message);
        }
    }

    /// Resolve template placeholders by checking both `template_values` and response data
    fn resolve_template_placeholders(
        template: &str,
        template_values: &serde_json::Map<String, Value>,
        clean_data: &Value,
    ) -> serde_json::Map<String, Value> {
        let mut final_template_values = template_values.clone();

        let mut remaining = template;
        while let Some(start) = remaining.find('{') {
            if let Some(end) = remaining[start + 1..].find('}') {
                let placeholder = &remaining[start + 1..start + 1 + end];

                if !final_template_values.contains_key(placeholder) {
                    if let Value::Object(data_map) = clean_data {
                        if let Some(value) = data_map.get(placeholder) {
                            final_template_values.insert(placeholder.to_string(), value.clone());
                        }
                    }
                }

                remaining = &remaining[start + 1 + end + 1..];
            } else {
                break;
            }
        }

        final_template_values
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

impl ResponseFormatter {
    /// Add format corrections to the response builder - with internal method check
    #[allow(clippy::too_many_lines)]
    fn add_format_corrections<T>(
        builder: &mut ResponseBuilder,
        handler_context: &HandlerContext<T>,
        format_corrections: Option<&[FormatCorrection]>,
        format_corrected: Option<&FormatCorrectionStatus>,
        brp_method_name: Option<&str>,
    ) -> Result<()> {
        tracing::debug!(
            "add_format_corrections called for method: {}",
            handler_context.request.name
        );
        tracing::debug!(
            "format_corrections: {:?}",
            format_corrections.map(<[_]>::len)
        );
        tracing::debug!("format_corrected: {:?}", format_corrected);
        tracing::debug!("brp_method_name: {:?}", brp_method_name);

        // Early return if method doesn't support format discovery - check BRP method name, not MCP
        // tool name
        if let Some(brp_method) = brp_method_name {
            if !FORMAT_DISCOVERY_METHODS.contains(&brp_method) {
                tracing::debug!(
                    "BRP method {} doesn't support format discovery, returning early",
                    brp_method
                );
                return Ok(());
            }
        } else {
            tracing::debug!("No BRP method name provided, returning early");
            return Ok(());
        }

        // Add format_corrected status if provided
        if let Some(status) = format_corrected {
            let format_corrected_value = serde_json::to_value(status).map_err(|e| {
                error_stack::Report::new(crate::error::Error::General(format!(
                    "Failed to serialize format_corrected: {e}"
                )))
            })?;
            *builder = builder
                .clone()
                .add_field(JSON_FIELD_FORMAT_CORRECTED, &format_corrected_value)?;
        }

        // Add format corrections array if provided and not empty
        if let Some(corrections) = format_corrections {
            if !corrections.is_empty() {
                let corrections_value = json!(
                    corrections
                        .iter()
                        .map(|correction| {
                            let mut correction_json = json!({
                                "component": correction.component,
                                "original_format": correction.original_format,
                                "corrected_format": correction.corrected_format,
                                "hint": correction.hint
                            });

                            // Add rich metadata fields if available
                            if let Some(obj) = correction_json.as_object_mut() {
                                if let Some(ops) = &correction.supported_operations {
                                    obj.insert("supported_operations".to_string(), json!(ops));
                                }
                                if let Some(paths) = &correction.mutation_paths {
                                    obj.insert("mutation_paths".to_string(), json!(paths));
                                }
                                if let Some(cat) = &correction.type_category {
                                    obj.insert("type_category".to_string(), json!(cat));
                                }
                            }

                            correction_json
                        })
                        .collect::<Vec<_>>()
                );

                *builder = builder
                    .clone()
                    .add_field(JSON_FIELD_FORMAT_CORRECTIONS, &corrections_value)?;

                // Add rich metadata from first correction to metadata field when format correction
                // succeeds
                if let Some(first_correction) = corrections.first() {
                    tracing::debug!("Found first correction: {:?}", first_correction.component);
                    if let Some(status) = format_corrected {
                        tracing::debug!("Format corrected status: {:?}", status);
                        if status == &FormatCorrectionStatus::Succeeded {
                            tracing::debug!(
                                "Format correction succeeded, adding rich metadata to response"
                            );
                            if let Some(ops) = &first_correction.supported_operations {
                                tracing::debug!("Adding supported_operations: {:?}", ops);
                                *builder = builder.clone().add_field_to(
                                    "supported_operations",
                                    json!(ops),
                                    crate::response::FieldPlacement::Metadata,
                                )?;
                            }
                            if let Some(paths) = &first_correction.mutation_paths {
                                tracing::debug!("Adding mutation_paths: {:?}", paths);
                                *builder = builder.clone().add_field_to(
                                    "mutation_paths",
                                    json!(paths),
                                    crate::response::FieldPlacement::Metadata,
                                )?;
                            }
                            if let Some(cat) = &first_correction.type_category {
                                tracing::debug!("Adding type_category: {:?}", cat);
                                *builder = builder.clone().add_field_to(
                                    "type_category",
                                    json!(cat),
                                    crate::response::FieldPlacement::Metadata,
                                )?;
                            }
                        } else {
                            tracing::debug!(
                                "Format correction status is not Succeeded: {:?}",
                                status
                            );
                        }
                    } else {
                        tracing::debug!("No format corrected status provided");
                    }
                } else {
                    tracing::debug!("No corrections found in array");
                }
            }
        }

        Ok(())
    }

    /// Override message if format correction occurred
    fn override_message_for_format_correction(
        builder: &mut ResponseBuilder,
        format_corrected: Option<&FormatCorrectionStatus>,
    ) {
        if format_corrected == Some(&FormatCorrectionStatus::Succeeded) {
            *builder = builder
                .clone()
                .message("Request succeeded with format correction applied");
        }
    }
}

/// Universal formatter that handles both local and BRP results uniformly
///
/// This function serves as the primary formatting entry point for all handler types.
/// It automatically detects error vs success responses and applies appropriate formatting,
/// including format correction handling for BRP results.
pub fn format_tool_call_result<T>(
    result: std::result::Result<serde_json::Value, McpError>,
    handler_context: &HandlerContext<T>,
    formatter_config: FormatterConfig,
) -> std::result::Result<CallToolResult, McpError>
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

                // Check if this is a BRP result with format correction information
                let (format_corrections, format_corrected) = extract_format_correction_info(&value);

                // For V2, the entire value contains the structured result
                // Use format_success_with_corrections to handle format correction messaging
                Ok(formatter.format_success_with_corrections(
                    &value,
                    handler_context,
                    format_corrections.as_deref(),
                    format_corrected.as_ref(),
                ))
            }
        }
        Err(e) => Err(e),
    }
}

/// Extract format correction information from V2 BRP result JSON
fn extract_format_correction_info(
    value: &serde_json::Value,
) -> (
    Option<Vec<FormatCorrection>>,
    Option<FormatCorrectionStatus>,
) {
    let format_corrected = value
        .get("format_corrected")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let format_corrections = value
        .get("format_corrections")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|correction_json| {
                    // Convert JSON back to FormatCorrection struct
                    let component = correction_json
                        .get("component")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let original_format = correction_json
                        .get("original_format")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let corrected_format = correction_json
                        .get("corrected_format")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let hint = correction_json
                        .get("hint")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let supported_operations = correction_json
                        .get("supported_operations")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        });

                    let mutation_paths = correction_json
                        .get("mutation_paths")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        });

                    let type_category = correction_json
                        .get("type_category")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    FormatCorrection {
                        component,
                        original_format,
                        corrected_format,
                        hint,
                        supported_operations,
                        mutation_paths,
                        type_category,
                    }
                })
                .collect()
        });

    (format_corrections, format_corrected)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::response::CallInfo;

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
    fn test_result_placement_direct_value() {
        // Test that FieldPlacement::Result puts data directly in result field
        use crate::response::builder::ResponseBuilder;

        let test_data = json!({
            "value": {
                "Srgba": {
                    "alpha": 1.0,
                    "blue": 0.1843,
                    "green": 0.1725,
                    "red": 0.1686
                }
            }
        });

        let call_info = CallInfo::local("test_tool".to_string());
        let response = ResponseBuilder::success(call_info)
            .message("Retrieved resource")
            .add_field_to("ignored_field_name", &test_data, FieldPlacement::Result)
            .expect("Failed to add field")
            .build();

        // The result field should directly contain our test_data
        assert_eq!(response.result, Some(test_data));

        // Convert to JSON to verify structure
        let json_str = response.to_json().expect("Failed to convert to JSON");
        let parsed: Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");

        // Verify the structure matches expected format
        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["message"], "Retrieved resource");
        assert_eq!(parsed["result"]["value"]["Srgba"]["alpha"], 1.0);

        // Ensure no wrapping field name was added
        assert!(parsed["result"].get("ignored_field_name").is_none());
    }
}
