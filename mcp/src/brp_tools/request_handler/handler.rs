use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::{Value, json};
use tracing::{debug, trace};

use super::config::{BrpHandlerConfig, FormatterContext};
use super::format_discovery::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
use super::traits::ExtractedParams;
use crate::BrpMcpService;
use crate::brp_tools::constants::{
    JSON_FIELD_DATA, JSON_FIELD_FORMAT_CORRECTIONS, JSON_FIELD_ORIGINAL_ERROR, JSON_FIELD_PORT,
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
) -> Result<RequestParams, McpError> {
    log_raw_request_arguments(request);

    debug!("Starting parameter extraction");

    // Extract parameters using the configured extractor
    let extracted = config.param_extractor.extract(request)?;

    // Log extracted method
    if let Some(ref method) = extracted.method {
        debug!("Extracted method: {}", method);
    }

    log_port_information(request, &extracted);
    log_extracted_parameters(&extracted);

    Ok(RequestParams { extracted })
}

/// Log raw request arguments with sanitization
fn log_raw_request_arguments(request: &rmcp::model::CallToolRequestParam) {
    if let Some(ref args) = request.arguments {
        let sanitized_args = serde_json::to_string(args)
            .unwrap_or_else(|_| "<serialization error>".to_string())
            .replace("\"value\":{", "\"value\":\"Hidden\",\"_original\":{")
            .replace("\"value\":[", "\"value\":\"Hidden\",\"_original\":[");

        trace!("Raw request arguments: {}", sanitized_args);
    } else {
        trace!("Raw request arguments: None");
    }
}

/// Log port information and validation
fn log_port_information(request: &rmcp::model::CallToolRequestParam, extracted: &ExtractedParams) {
    // Check if port was explicitly provided in the request
    let port_provided = request
        .arguments
        .as_ref()
        .and_then(|args| args.get(JSON_FIELD_PORT))
        .is_some();

    if port_provided {
        debug!("Extracted port: {} (explicitly provided)", extracted.port);
    } else {
        debug!(
            "Extracted port: {} (using default - NOT provided in request)",
            extracted.port
        );
    }

    // Add more detailed port debugging for mutate_component operations
    if request.name.contains("mutate_component") {
        debug!(
            "CRITICAL PORT DEBUG: mutate_component operation - port source: {}",
            if port_provided {
                "explicit"
            } else {
                "DEFAULT (missing port parameter!)"
            }
        );
    }
}

/// Log extracted parameters with security considerations
fn log_extracted_parameters(extracted: &ExtractedParams) {
    if let Some(ref params) = extracted.params {
        log_common_brp_parameters(params);
    } else {
        debug!("Extracted params: None");
    }
}

/// Log specific extracted parameters based on common BRP patterns
fn log_common_brp_parameters(params: &Value) {
    if let Some(entity) = params.get("entity").and_then(serde_json::Value::as_u64) {
        debug!("Extracted entity: {}", entity);
    }

    if let Some(component) = params.get("component").and_then(serde_json::Value::as_str) {
        debug!("Extracted component: {}", component);
    }

    if let Some(resource) = params.get("resource").and_then(serde_json::Value::as_str) {
        debug!("Extracted resource: {}", resource);
    }

    if let Some(path) = params.get("path").and_then(serde_json::Value::as_str) {
        debug!("Extracted path: {}", path);
    }

    if params.get("value").is_some() {
        debug!("Extracted value: [Hidden for security]");
    }

    if let Some(components) = params.get("components") {
        if let Some(obj) = components.as_object() {
            debug!("Extracted components: {} types", obj.len());
            for key in obj.keys() {
                trace!("  - Component type: {}", key);
            }
        }
    }
}

/// Resolve the actual BRP method name to call
fn resolve_brp_method(
    extracted: &ExtractedParams,
    config: &BrpHandlerConfig,
) -> Result<String, McpError> {
    debug!("Starting method resolution");

    // Log the method resolution sources
    if let Some(ref method) = extracted.method {
        debug!("Method from request: {}", method);
    } else {
        debug!("Method from request: None");
    }

    if let Some(config_method) = config.method {
        debug!("Method from config: {}", config_method);
    } else {
        debug!("Method from config: None");
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

    debug!("Method resolution: {}", resolved_method);

    Ok(resolved_method)
}

/// Convert a `FormatCorrection` to JSON representation with metadata
fn format_correction_to_json(correction: &FormatCorrection) -> Value {
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
}

/// Add only format corrections to response data (not debug info)
fn add_format_corrections_only(response_data: &mut Value, format_corrections: &[FormatCorrection]) {
    if format_corrections.is_empty() {
        return;
    }

    let corrections_value = json!(
        format_corrections
            .iter()
            .map(format_correction_to_json)
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

    // Debug info is now logged via tracing during execution

    // Add format corrections only (not debug info, as it will be handled separately)
    add_format_corrections_only(&mut response_data, &enhanced_result.format_corrections);

    // Create new FormatterContext
    let new_formatter_context = FormatterContext {
        params: context.formatter_context.params.clone(),
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

    // Add format corrections to error data if present
    if !enhanced_result.format_corrections.is_empty() || has_enhanced {
        let mut data_obj = error_info.data.unwrap_or_else(|| json!({}));

        if let Value::Object(map) = &mut data_obj {
            // Store original error message if we replaced it with enhanced message
            if has_enhanced {
                map.insert(
                    JSON_FIELD_ORIGINAL_ERROR.to_string(),
                    json!(original_error_message),
                );
            }

            // Debug info is now handled via tracing system

            // Add format corrections
            if !enhanced_result.format_corrections.is_empty() {
                let corrections = enhanced_result
                    .format_corrections
                    .iter()
                    .map(format_correction_to_json)
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
    // Log raw MCP request at the earliest possible point
    debug!("MCP ENTRY - Tool: {}", request.name);
    trace!(
        "MCP ENTRY - Raw arguments: {}",
        serde_json::to_string(&request.arguments)
            .unwrap_or_else(|_| "SERIALIZATION_ERROR".to_string())
    );

    // Extract all parameters from the request
    let params = extract_request_params(&request, config)?;
    let extracted = params.extracted;

    // Determine the actual method to call
    let method_name = resolve_brp_method(&extracted, config)?;

    // Add debug info about calling BRP
    debug!("Calling BRP with validated parameters");

    // Call BRP using format discovery
    let enhanced_result = execute_brp_method_with_format_discovery(
        &method_name,
        extracted.params.clone(),
        Some(extracted.port),
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
        params: Some(context_params),
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
