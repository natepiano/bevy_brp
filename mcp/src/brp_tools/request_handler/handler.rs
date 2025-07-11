use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use tracing::{debug, trace};

use super::config::BrpHandlerConfig;
use super::format_discovery::{
    EnhancedBrpResult, FORMAT_DISCOVERY_METHODS, FormatCorrection,
    execute_brp_method_with_format_discovery,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult};
use crate::constants::{
    JSON_FIELD_FORMAT_CORRECTED, JSON_FIELD_FORMAT_CORRECTIONS, JSON_FIELD_METADATA,
    JSON_FIELD_ORIGINAL_ERROR, JSON_FIELD_PORT,
};
use crate::error::{Error, report_to_mcp_error};
use crate::extractors::ExtractedParams;
use crate::response::{self, FormatterContext, ResponseFormatterFactory};
use crate::tool::{BrpToolCallInfo, TOOL_BRP_EXECUTE};

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
                &error_stack::Report::new(Error::InvalidArgument(
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
    let corrections_value = if format_corrections.is_empty() {
        json!([])
    } else {
        json!(
            format_corrections
                .iter()
                .map(format_correction_to_json)
                .collect::<Vec<_>>()
        )
    };

    // If response_data is an object, add fields
    if let Value::Object(map) = response_data {
        map.insert(JSON_FIELD_FORMAT_CORRECTIONS.to_string(), corrections_value);
        // Note: format_corrected is now passed via FormatterContext, not serialized here
    } else {
        // If not an object, wrap it
        let wrapped = json!({
            JSON_FIELD_METADATA: response_data.clone(),
            JSON_FIELD_FORMAT_CORRECTIONS: corrections_value,
        });
        *response_data = wrapped;
    }
}

/// Context for processing responses
struct ResponseContext<'a> {
    call_info:         BrpToolCallInfo,
    formatter_factory: &'a ResponseFormatterFactory,
    formatter_context: FormatterContext,
    method:            &'a str,
}

/// Process a successful BRP response
fn process_success_response(
    data: Option<Value>,
    enhanced_result: &EnhancedBrpResult,
    context: ResponseContext<'_>,
) -> CallToolResult {
    let mut response_data = data.unwrap_or(Value::Null);

    // Debug info is now logged via tracing during execution

    // Only add format corrections for methods that support format discovery
    if FORMAT_DISCOVERY_METHODS.contains(&context.method) {
        // Add format corrections only (not debug info, as it will be handled separately)
        add_format_corrections_only(&mut response_data, &enhanced_result.format_corrections);
    }

    // Create new FormatterContext with format_corrected status only for supported methods
    let new_formatter_context = FormatterContext {
        params:           context.formatter_context.params.clone(),
        format_corrected: if FORMAT_DISCOVERY_METHODS.contains(&context.method) {
            Some(enhanced_result.format_corrected.clone())
        } else {
            None
        },
    };

    // Create new formatter with updated context
    let updated_formatter = context.formatter_factory.create(new_formatter_context);

    // Large response handling is now done inside the formatter
    updated_formatter.format_success(&response_data, context.call_info)
}

/// Process an error BRP response - routes ALL errors through enhanced `format_error_default`
fn process_error_response(
    mut error_info: BrpError,
    enhanced_result: &EnhancedBrpResult,
    call_info: &BrpToolCallInfo,
    method: &str,
) -> CallToolResult {
    let original_error_message = error_info.message.clone();

    // First, check if the enhanced result has a different error message (from educational guidance)
    let enhanced_message = if let BrpResult::Error(enhanced_error) = &enhanced_result.result {
        if enhanced_error.message == error_info.message {
            None
        } else {
            Some(enhanced_error.message.clone())
        }
    } else {
        None
    };

    // If no enhanced message from result, check format corrections as fallback
    let enhanced_message = enhanced_message.or_else(|| {
        enhanced_result
            .format_corrections
            .iter()
            .find(|correction| correction.hint.contains("cannot be used with BRP"))
            .map(|correction| correction.hint.clone())
    });

    // Use enhanced message if available, otherwise keep original
    let has_enhanced = enhanced_message.is_some();
    if let Some(enhanced_msg) = enhanced_message {
        error_info.message = enhanced_msg;
    }

    // Only add format corrections for methods that support format discovery
    if FORMAT_DISCOVERY_METHODS.contains(&method)
        && (!enhanced_result.format_corrections.is_empty() || has_enhanced)
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

            // Add format_corrected field to indicate whether format correction occurred
            map.insert(
                JSON_FIELD_FORMAT_CORRECTED.to_string(),
                json!(enhanced_result.format_corrected),
            );
        }

        error_info.data = Some(data_obj);
    }

    // Route ALL errors through the enhanced format_error_default
    response::format_error_default(error_info, call_info)
}

/// Unified handler for all BRP methods (both static and dynamic)
pub async fn handle_brp_method_tool_call(
    handler_context: crate::service::HandlerContext,
    config: &BrpHandlerConfig,
) -> Result<CallToolResult, McpError> {
    // Log raw MCP request at the earliest possible point
    debug!("MCP ENTRY - Tool: {}", handler_context.request.name);
    trace!(
        "MCP ENTRY - Raw arguments: {}",
        serde_json::to_string(&handler_context.request.arguments)
            .unwrap_or_else(|_| "SERIALIZATION_ERROR".to_string())
    );

    // Log request arguments with sanitization
    log_raw_request_arguments(&handler_context.request);

    // Get pre-extracted parameters directly
    let extracted = &config.extracted_params;

    // Determine the actual method to call
    let method_name = resolve_brp_method(extracted, config)?;

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
        params:           Some(context_params),
        format_corrected: None, // Initial context doesn't have format correction status yet
    };

    // Use "brp_execute" for dynamic methods for special error formatting
    let metadata_method = if extracted.method.is_some() {
        TOOL_BRP_EXECUTE
    } else {
        &method_name
    };
    let call_info = BrpToolCallInfo::new(metadata_method, extracted.port);

    // Process response using ResponseFormatter, including format corrections if present
    match &enhanced_result.result {
        BrpResult::Success(data) => {
            let context = ResponseContext {
                call_info,
                formatter_factory: &config.formatter_factory,
                formatter_context,
                method: &method_name,
            };
            Ok(process_success_response(
                data.clone(),
                &enhanced_result,
                context,
            ))
        }
        BrpResult::Error(error_info) => Ok(process_error_response(
            error_info.clone(),
            &enhanced_result,
            &call_info,
            &method_name,
        )),
    }
}
