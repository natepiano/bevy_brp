use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::{Value, json};

use super::config::{BrpHandlerConfig, FormatterContext};
use super::format_discovery::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
use super::traits::ExtractedParams;
use crate::BrpMcpService;
use crate::brp_tools::brp_set_debug_mode;
use crate::brp_tools::constants::{
    JSON_FIELD_DATA, JSON_FIELD_DEBUG_INFO, JSON_FIELD_FORMAT_CORRECTIONS,
    JSON_FIELD_ORIGINAL_ERROR, JSON_FIELD_PORT,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult};
use crate::brp_tools::support::response_formatter::{BrpMetadata, ResponseFormatter};
use crate::error::{Error, report_to_mcp_error};
use crate::support::large_response::handle_brp_large_response;

/// Result of parameter extraction from a request
pub struct RequestParams {
    /// Extracted parameters from the configured extractor
    pub extracted: ExtractedParams,
}

/// Extract and validate all parameters from a BRP request
fn extract_request_params(
    request: &rmcp::model::CallToolRequestParam,
    config: &BrpHandlerConfig,
    debug_info: &mut Vec<String>,
) -> Result<RequestParams, McpError> {
    // Log raw request arguments before extraction
    if let Some(ref args) = request.arguments {
        let sanitized_args = serde_json::to_string(args)
            .unwrap_or_else(|_| "<serialization error>".to_string())
            .replace("\"value\":{", "\"value\":\"Hidden\",\"_original\":{")
            .replace("\"value\":[", "\"value\":\"Hidden\",\"_original\":[");

        debug_info.push(format!("Raw request arguments: {sanitized_args}"));
    } else {
        debug_info.push("Raw request arguments: None".to_string());
    }

    debug_info.push("Starting parameter extraction".to_string());

    // Extract parameters using the configured extractor
    let extracted = config.param_extractor.extract(request)?;

    // Log extracted parameters with sanitization
    if let Some(ref method) = extracted.method {
        debug_info.push(format!("Extracted method: {method}"));
    }

    debug_info.push(format!("Extracted port: {}", extracted.port));

    if let Some(ref params) = extracted.params {
        // Log specific extracted parameters based on common BRP patterns
        if let Some(entity) = params.get("entity").and_then(serde_json::Value::as_u64) {
            debug_info.push(format!("Extracted entity: {entity}"));
        }

        if let Some(component) = params.get("component").and_then(serde_json::Value::as_str) {
            debug_info.push(format!("Extracted component: {component}"));
        }

        if let Some(resource) = params.get("resource").and_then(serde_json::Value::as_str) {
            debug_info.push(format!("Extracted resource: {resource}"));
        }

        if let Some(path) = params.get("path").and_then(serde_json::Value::as_str) {
            debug_info.push(format!("Extracted path: {path}"));
        }

        if params.get("value").is_some() {
            debug_info.push("Extracted value: [Hidden for security]".to_string());
        }

        if let Some(components) = params.get("components") {
            if let Some(obj) = components.as_object() {
                debug_info.push(format!("Extracted components: {} types", obj.len()));
                for key in obj.keys() {
                    debug_info.push(format!("  - Component type: {key}"));
                }
            }
        }
    } else {
        debug_info.push("Extracted params: None".to_string());
    }

    Ok(RequestParams { extracted })
}

/// Resolve the actual BRP method name to call
fn resolve_brp_method(
    extracted: &ExtractedParams,
    config: &BrpHandlerConfig,
    debug_info: &mut Vec<String>,
) -> Result<String, McpError> {
    debug_info.push("Starting method resolution".to_string());

    // Log the method resolution sources
    if let Some(ref method) = extracted.method {
        debug_info.push(format!("Method from request: {method}"));
    } else {
        debug_info.push("Method from request: None".to_string());
    }

    if let Some(config_method) = config.method {
        debug_info.push(format!("Method from config: {config_method}"));
    } else {
        debug_info.push("Method from config: None".to_string());
    }

    // Perform the actual resolution
    let resolved_method = extracted
        .method
        .as_deref()
        .or(config.method)
        .map(String::from)
        .ok_or_else(|| -> McpError {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(
                    "Missing BRP method specification".to_string(),
                ))
                .attach_printable("Either method from request or config must be specified"),
            )
        })?;

    debug_info.push(format!("Method resolution: {resolved_method}"));

    Ok(resolved_method)
}

/// Add only format corrections to response data (not debug info)
fn add_format_corrections_only(response_data: &mut Value, format_corrections: &[FormatCorrection]) {
    if format_corrections.is_empty() {
        return;
    }

    let corrections_value = json!(
        format_corrections
            .iter()
            .map(|correction| {
                json!({
                    "component": correction.component,
                    "original_format": correction.original_format,
                    "corrected_format": correction.corrected_format,
                    "hint": correction.hint
                })
            })
            .collect::<Vec<_>>()
    );

    // If response_data is an object, add fields
    if let Value::Object(map) = response_data {
        map.insert(JSON_FIELD_FORMAT_CORRECTIONS.to_string(), corrections_value);
    } else {
        // If not an object, wrap it
        let wrapped = json!({
            JSON_FIELD_DATA: response_data.clone(),
            JSON_FIELD_FORMAT_CORRECTIONS: corrections_value
        });
        *response_data = wrapped;
    }
}

/// Context for processing responses
struct ResponseContext<'a> {
    metadata:          BrpMetadata,
    formatter_factory: &'a crate::brp_tools::support::response_formatter::ResponseFormatterFactory,
    formatter_context: FormatterContext,
}

/// Process a successful BRP response
fn process_success_response(
    data: Option<Value>,
    enhanced_result: &EnhancedBrpResult,
    method_name: &str,
    context: ResponseContext<'_>,
) -> Result<CallToolResult, McpError> {
    let mut response_data = data.unwrap_or(Value::Null);

    // Extract debug info for BRP MCP debug info
    let brp_mcp_debug_info =
        if !enhanced_result.debug_info.is_empty() && brp_set_debug_mode::is_debug_enabled() {
            Some(json!(enhanced_result.debug_info))
        } else {
            None
        };

    // Add format corrections only (not debug info, as it will be handled separately)
    add_format_corrections_only(&mut response_data, &enhanced_result.format_corrections);

    // Create new FormatterContext with BRP MCP debug info
    let new_formatter_context = FormatterContext {
        params: context.formatter_context.params.clone(),
        brp_mcp_debug_info,
    };

    // Create new formatter with updated context
    let updated_formatter = context.formatter_factory.create(new_formatter_context);

    // Check if response is too large and use file fallback if needed
    let final_data = handle_brp_large_response(&response_data, method_name)
        .map_err(|e| report_to_mcp_error(&e))?
        .map_or(response_data, |fallback_response| fallback_response);

    Ok(updated_formatter.format_success(&final_data, context.metadata))
}

/// Process an error BRP response
fn process_error_response(
    mut error_info: BrpError,
    enhanced_result: &EnhancedBrpResult,
    formatter: &ResponseFormatter,
    metadata: &BrpMetadata,
) -> CallToolResult {
    let original_error_message = error_info.message.clone();

    // Check if we have an enhanced diagnostic message from format discovery
    let enhanced_message = enhanced_result
        .format_corrections
        .iter()
        .find(|correction| correction.hint.contains("cannot be used with BRP"))
        .map(|correction| correction.hint.clone());

    // Use enhanced message if available, otherwise keep original
    let has_enhanced = enhanced_message.is_some();
    if let Some(enhanced_msg) = enhanced_message {
        error_info.message = enhanced_msg;
    }

    // Add debug info and format corrections to error data if present
    if !enhanced_result.debug_info.is_empty()
        || !enhanced_result.format_corrections.is_empty()
        || has_enhanced
    {
        let mut data_obj = error_info.data.unwrap_or_else(|| json!({}));

        if let Value::Object(map) = &mut data_obj {
            // Store original error message if we replaced it with enhanced message
            if has_enhanced {
                map.insert(
                    JSON_FIELD_ORIGINAL_ERROR.to_string(),
                    json!(original_error_message),
                );
            }

            // Add debug info only if debug mode is enabled
            if !enhanced_result.debug_info.is_empty() && brp_set_debug_mode::is_debug_enabled() {
                map.insert(
                    JSON_FIELD_DEBUG_INFO.to_string(),
                    json!(enhanced_result.debug_info),
                );
            }

            // Add format corrections
            if !enhanced_result.format_corrections.is_empty() {
                let corrections = enhanced_result
                    .format_corrections
                    .iter()
                    .map(|c| {
                        json!({
                            "component": c.component,
                            "hint": c.hint,
                            "original_format": c.original_format,
                            "corrected_format": c.corrected_format
                        })
                    })
                    .collect::<Vec<_>>();
                map.insert(
                    JSON_FIELD_FORMAT_CORRECTIONS.to_string(),
                    json!(corrections),
                );
            }
        }

        error_info.data = Some(data_obj);
    }

    formatter.format_error(error_info, metadata)
}

/// Unified handler for all BRP methods (both static and dynamic)
pub async fn handle_brp_request(
    _service: &BrpMcpService,
    request: rmcp::model::CallToolRequestParam,
    _context: RequestContext<RoleServer>,
    config: &BrpHandlerConfig,
) -> Result<CallToolResult, McpError> {
    // Create debug info and log the earliest entry point
    let mut debug_info = Vec::new();

    // Log raw MCP request at the earliest possible point
    debug_info.push(format!("MCP ENTRY - Tool: {}", request.name));
    debug_info.push(format!(
        "MCP ENTRY - Raw arguments: {}",
        serde_json::to_string(&request.arguments)
            .unwrap_or_else(|_| "SERIALIZATION_ERROR".to_string())
    ));

    // Extract all parameters from the request
    let params = extract_request_params(&request, config, &mut debug_info)?;
    let extracted = params.extracted;

    // Determine the actual method to call
    let method_name = resolve_brp_method(&extracted, config, &mut debug_info)?;

    // Add debug info about calling BRP
    debug_info.push("Calling BRP with validated parameters".to_string());

    // Call BRP using format discovery
    let enhanced_result = execute_brp_method_with_format_discovery(
        &method_name,
        extracted.params.clone(),
        Some(extracted.port),
        debug_info,
    )
    .await
    .map_err(|err| crate::error::report_to_mcp_error(&err))?;

    // Create formatter and metadata
    // Ensure port is included in params for extractors that need it
    let mut context_params = extracted.params.clone().unwrap_or_else(|| json!({}));
    if let Value::Object(ref mut map) = context_params {
        // Only add port if it's not already present (to avoid overwriting explicit port params)
        if !map.contains_key(JSON_FIELD_PORT) {
            map.insert(JSON_FIELD_PORT.to_string(), json!(extracted.port));
        }
    }

    let formatter_context = FormatterContext {
        params:             Some(context_params),
        brp_mcp_debug_info: None, // Will be populated later when processing responses
    };
    let formatter = config.formatter_factory.create(formatter_context.clone());

    // Use "brp_execute" for dynamic methods for special error formatting
    let metadata_method = if extracted.method.is_some() {
        "brp_execute"
    } else {
        &method_name
    };
    let metadata = BrpMetadata::new(metadata_method, extracted.port);

    // Process response using ResponseFormatter, including format corrections if present
    match &enhanced_result.result {
        BrpResult::Success(data) => {
            let context = ResponseContext {
                metadata,
                formatter_factory: &config.formatter_factory,
                formatter_context,
            };
            process_success_response(data.clone(), &enhanced_result, &method_name, context)
        }
        BrpResult::Error(error_info) => Ok(process_error_response(
            error_info.clone(),
            &enhanced_result,
            &formatter,
            &metadata,
        )),
    }
}
