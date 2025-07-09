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

use super::brp_client::BrpError;
use crate::brp_tools::constants::{
    JSON_FIELD_DEBUG_INFO, JSON_FIELD_ERROR_CODE, JSON_FIELD_FORMAT_CORRECTED, JSON_FIELD_METADATA,
    JSON_FIELD_METHOD, JSON_FIELD_PORT,
};
use crate::brp_tools::request_handler::FormatterContext;
use crate::error::Result;
use crate::support::response::ResponseBuilder;
use crate::support::{LargeResponseConfig, handle_large_response};

/// Metadata about a BRP request for response formatting
#[derive(Debug, Clone)]
pub struct BrpMetadata {
    pub method: String,
    pub port:   u16,
}

impl BrpMetadata {
    pub fn new(method: &str, port: u16) -> Self {
        Self {
            method: method.to_string(),
            port,
        }
    }
}

/// Default error formatter implementation with rich guidance extraction
pub fn format_error_default(error: BrpError, metadata: &BrpMetadata) -> CallToolResult {
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
    metadata: &BrpMetadata,
    rich_guidance: Option<RichGuidance>,
) -> Result<crate::support::response::JsonResponse> {
    let mut builder = ResponseBuilder::error()
        .message(&error.message)
        .add_field(JSON_FIELD_ERROR_CODE, error.code)?
        .add_field(
            JSON_FIELD_METADATA,
            json!({
                JSON_FIELD_METHOD: metadata.method,
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

/// Function type for extracting field values from context
pub type FieldExtractor = Box<dyn Fn(&Value, &FormatterContext) -> Value + Send + Sync>;

impl ResponseFormatter {
    #[allow(clippy::missing_const_for_fn)] // False positive - Arc doesn't support const construction
    pub fn new(config: Arc<FormatterConfig>, context: FormatterContext) -> Self {
        Self { config, context }
    }

    pub fn format_success(&self, data: &Value, metadata: BrpMetadata) -> CallToolResult {
        // Check if this is a passthrough formatter - if so, convert data directly to CallToolResult
        if self.is_passthrough_formatter() {
            return Self::format_passthrough_response(self, data);
        }

        // First build the response
        let response_result = self.build_success_response(data);

        if let Ok(response) = response_result {
            self.handle_large_response_if_needed(response, &metadata.method)
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

    /// Format a passthrough response by converting the data directly to `CallToolResult`
    fn format_passthrough_response(_self: &Self, data: &Value) -> CallToolResult {
        // For passthrough, the data should already be a structured response
        // Convert it directly to CallToolResult as JSON content
        let json_string = serde_json::to_string_pretty(data).unwrap_or_else(|_| {
            r#"{"status":"error","message":"Failed to serialize passthrough response"}"#.to_string()
        });

        CallToolResult::success(vec![rmcp::model::Content::text(json_string)])
    }

    /// Handle large response processing if configured
    fn handle_large_response_if_needed(
        &self,
        response: crate::support::response::JsonResponse,
        method: &str,
    ) -> CallToolResult {
        // Check if we need to handle large response
        self.config.large_response_config.as_ref().map_or_else(
            || response.to_call_tool_result(),
            |large_config| {
                // Convert response to Value for size checking
                let response_value = serde_json::to_value(&response).unwrap_or(Value::Null);

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
                    Err(_) => {
                        // Error handling large response, return original
                        response.to_call_tool_result()
                    }
                }
            },
        )
    }

    fn build_success_response(
        &self,
        data: &Value,
    ) -> Result<crate::support::response::JsonResponse> {
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
            let template_params = Value::Object(template_values);
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

    /// Create a formatter for simple entity operations (destroy, etc.)
    pub fn entity_operation(_entity_field: &str) -> ResponseFormatterBuilder {
        Self::standard()
    }

    /// Create a formatter for resource operations
    pub fn resource_operation(_resource_field: &str) -> ResponseFormatterBuilder {
        Self::standard()
    }

    /// Create a formatter that passes through the response data
    #[cfg(test)]
    pub fn pass_through() -> ResponseFormatterBuilder {
        use crate::brp_tools::constants::JSON_FIELD_DATA;

        ResponseFormatterBuilder {
            config: FormatterConfig {
                success_template:      Some("Operation completed successfully".to_string()),
                success_fields:        vec![(
                    JSON_FIELD_DATA.to_string(),
                    Box::new(extractors::pass_through_data),
                )],
                large_response_config: Some(LargeResponseConfig {
                    file_prefix: "brp_response_".to_string(),
                    ..Default::default()
                }),
            },
        }
    }

    /// Create a formatter for list operations
    pub fn list_operation() -> ResponseFormatterBuilder {
        Self::standard()
    }

    /// Create a formatter for local standard operations
    pub fn local_standard() -> ResponseFormatterBuilder {
        Self::standard()
    }

    /// Create a formatter for local collection operations
    pub fn local_collection() -> ResponseFormatterBuilder {
        Self::standard()
    }

    /// Create a formatter for local operations that return pre-structured responses
    pub fn local_passthrough() -> ResponseFormatterBuilder {
        ResponseFormatterBuilder {
            config: FormatterConfig {
                success_template:      None,
                success_fields:        vec![],
                large_response_config: None, // No large response handling for passthrough
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
pub mod extractors {
    use super::{FormatterContext, Value};
    use crate::brp_tools::constants::{JSON_FIELD_ENTITY, JSON_FIELD_RESOURCE};

    /// Extract entity ID from context params
    pub fn entity_from_params(_data: &Value, context: &FormatterContext) -> Value {
        context
            .params
            .as_ref()
            .and_then(|p| p.get(JSON_FIELD_ENTITY))
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Extract resource name from context params
    pub fn resource_from_params(_data: &Value, context: &FormatterContext) -> Value {
        context
            .params
            .as_ref()
            .and_then(|p| p.get(JSON_FIELD_RESOURCE))
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Pass through the BRP response data
    pub fn pass_through_data(data: &Value, _context: &FormatterContext) -> Value {
        data.clone()
    }

    /// Count elements in an array from the response data
    pub fn array_count(data: &Value, _context: &FormatterContext) -> Value {
        // Check if data is wrapped in a structure with a "data" field
        data.as_object()
            .and_then(|obj| obj.get("data"))
            .map_or_else(
                || data.as_array().map_or(0, std::vec::Vec::len).into(),
                |inner_data| inner_data.as_array().map_or(0, std::vec::Vec::len).into(),
            )
    }

    /// Create a field extractor that gets components from params
    #[cfg(test)]
    pub fn components_from_params(_data: &Value, context: &FormatterContext) -> Value {
        context
            .params
            .as_ref()
            .and_then(|p| p.get("components"))
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Extract count from data for local operations
    pub fn count_from_data(data: &Value, _context: &FormatterContext) -> Value {
        // Check if data is wrapped in a structure with a "count" field
        data.as_object()
            .and_then(|obj| obj.get("count"))
            .map_or_else(
                || data.as_array().map_or(0, std::vec::Vec::len).into(),
                std::clone::Clone::clone,
            )
    }

    /// Extract message from params for local operations
    pub fn message_from_params(_data: &Value, context: &FormatterContext) -> Value {
        context
            .params
            .as_ref()
            .and_then(|p| p.get("message"))
            .cloned()
            .unwrap_or_else(|| Value::String("Operation completed".to_string()))
    }
}

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
                Box::new(extractors::entity_from_params),
            )],
            large_response_config: None,
        };

        let context = FormatterContext {
            params:           Some(json!({ "entity": 123 })),
            format_corrected: None,
        };

        let formatter = ResponseFormatter::new(Arc::new(config), context);
        let metadata = BrpMetadata::new("bevy/destroy", DEFAULT_BRP_PORT);
        let result = formatter.format_success(&Value::Null, metadata);

        // Verify result has content
        assert_eq!(result.content.len(), 1);
        // For now, we'll just verify that formatting doesn't panic
        // TODO: Once we understand Content type better, add proper content validation
    }

    #[test]
    fn test_enhanced_error_formatting_direct() {
        let metadata = BrpMetadata::new("bevy/destroy", DEFAULT_BRP_PORT);
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

    #[test]
    fn test_entity_operation_builder() {
        use crate::brp_tools::constants::JSON_FIELD_DESTROYED_ENTITY;

        let factory = ResponseFormatterFactory::entity_operation(JSON_FIELD_DESTROYED_ENTITY)
            .with_template("Successfully destroyed entity {entity}")
            .with_response_field(
                JSON_FIELD_DESTROYED_ENTITY,
                Box::new(extractors::entity_from_params),
            )
            .build();

        let context = FormatterContext {
            params:           Some(json!({ "entity": 789 })),
            format_corrected: None,
        };

        let formatter = factory.create(context);
        let metadata = BrpMetadata::new("bevy/destroy", DEFAULT_BRP_PORT);
        let result = formatter.format_success(&Value::Null, metadata);

        // Verify result has content
        assert_eq!(result.content.len(), 1);
        // TODO: Add proper content validation once Content type is understood
    }

    #[test]
    fn test_pass_through_builder() {
        let factory = ResponseFormatterFactory::pass_through().build();

        let context = FormatterContext {
            params:           None,
            format_corrected: None,
        };

        let formatter = factory.create(context);
        let metadata = BrpMetadata::new("bevy/query", DEFAULT_BRP_PORT);
        let data = json!([{"entity": 1}, {"entity": 2}]);
        let result = formatter.format_success(&data, metadata);

        // Verify result has content
        assert_eq!(result.content.len(), 1);
        // TODO: Add proper content validation once Content type is understood
    }

    #[test]
    fn test_extractors() {
        let context = FormatterContext {
            params:           Some(json!({
                "entity": 100,
                "resource": "TestResource"
            })),
            format_corrected: None,
        };

        let data = json!({"result": "success"});

        assert_eq!(extractors::entity_from_params(&data, &context), 100);
        assert_eq!(
            extractors::resource_from_params(&data, &context),
            "TestResource"
        );
        assert_eq!(extractors::pass_through_data(&data, &context), data);

        // Test array_count extractor
        let array_data = json!([1, 2, 3, 4, 5]);
        assert_eq!(extractors::array_count(&array_data, &context), 5);

        let empty_array = json!([]);
        assert_eq!(extractors::array_count(&empty_array, &context), 0);

        let non_array = json!({"not": "array"});
        assert_eq!(extractors::array_count(&non_array, &context), 0);

        // Test components_from_params extractor
        let components_context = FormatterContext {
            params:           Some(json!({
                "components": ["Transform", "Sprite"]
            })),
            format_corrected: None,
        };
        assert_eq!(
            extractors::components_from_params(&data, &components_context),
            json!(["Transform", "Sprite"])
        );

        // Test with missing components field
        assert_eq!(
            extractors::components_from_params(&data, &context),
            Value::Null
        );
    }
}
