use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use tracing::{debug, trace};

use super::format_discovery::{
    EnhancedBrpResult, FORMAT_DISCOVERY_METHODS, FormatCorrection, FormatCorrectionStatus,
    execute_brp_method_with_format_discovery,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult};
use crate::constants::{
    JSON_FIELD_FORMAT_CORRECTED, JSON_FIELD_FORMAT_CORRECTIONS,
    JSON_FIELD_ORIGINAL_ERROR,
};
use crate::error;
use crate::response::{self, FormatterConfig, ResponseFormatter};
use crate::service::{BrpContext, HandlerContext};

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


/// Process a successful BRP response
fn process_success_response(
    data: Option<Value>,
    enhanced_result: &EnhancedBrpResult,
    formatter_config: FormatterConfig,
    handler_context: &HandlerContext<BrpContext>,
) -> CallToolResult {
    let response_data = data.unwrap_or(Value::Null);


    // Create formatter with appropriate message template
    let final_formatter_config =
        if enhanced_result.format_corrected == FormatCorrectionStatus::Succeeded {
            // Clone and modify the config for format correction success
            let mut modified_config = formatter_config;
            modified_config.success_template =
                Some("Request succeeded with format correction applied".to_string());
            modified_config
        } else {
            formatter_config
        };

    // Create new formatter
    let updated_formatter = ResponseFormatter::new(final_formatter_config);

    // Use format_success_with_corrections to include call_info and format corrections
    updated_formatter.format_success_with_corrections_brp(
        &response_data,
        handler_context,
        Some(&enhanced_result.format_corrections),
        Some(&enhanced_result.format_corrected),
    )
}

/// Process an error BRP response - routes ALL errors through enhanced `format_error_default`
fn process_error_response(
    mut error_info: BrpError,
    enhanced_result: &EnhancedBrpResult,
    handler_context: &HandlerContext<BrpContext>,
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
    if FORMAT_DISCOVERY_METHODS.contains(&handler_context.brp_method())
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
                
                // Add rich metadata from first correction to error metadata for better UX
                if let Some(first_correction) = enhanced_result.format_corrections.first() {
                    if let Some(ops) = &first_correction.supported_operations {
                        map.insert("supported_operations".to_string(), json!(ops));
                    }
                    if let Some(paths) = &first_correction.mutation_paths {
                        map.insert("mutation_paths".to_string(), json!(paths));
                    }
                    if let Some(cat) = &first_correction.type_category {
                        map.insert("type_category".to_string(), json!(cat));
                    }
                }
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
    response::format_error_default(error_info, handler_context)
}

/// Unified handler for all BRP methods (both static and dynamic)
pub async fn handle_brp_method_tool_call(
    handler_context: HandlerContext<BrpContext>,
    formatter_config: FormatterConfig,
) -> Result<CallToolResult, McpError> {
    // Log raw MCP request at the earliest possible point
    debug!("MCP ENTRY - Tool: {}", handler_context.request.name);
    trace!(
        "MCP ENTRY - Raw arguments: {}",
        serde_json::to_string(&handler_context.request.arguments)
            .unwrap_or_else(|_| "SERIALIZATION_ERROR".to_string())
    );

    // Extract parameters for the call based on the tool definition
    let params = handler_context.extract_params_from_definition()?;

    // Get the method directly from the typed context - no Options!
    let method_name = handler_context.brp_method();

    // Call BRP using format discovery
    let enhanced_result = execute_brp_method_with_format_discovery(
        method_name,
        params.clone(),
        handler_context.extract_port()?,
    )
    .await
    .map_err(|err| error::report_to_mcp_error(&err))?;

    // Process response using ResponseFormatter, including format corrections if present
    match &enhanced_result.result {
        BrpResult::Success(data) => Ok(process_success_response(
            data.clone(),
            &enhanced_result,
            formatter_config,
            &handler_context,
        )),
        BrpResult::Error(error_info) => Ok(process_error_response(
            error_info.clone(),
            &enhanced_result,
            &handler_context,
        )),
    }
}
