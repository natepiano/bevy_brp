use rmcp::Error as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use tracing::{debug, trace};

use super::format_discovery::{
    EnhancedBrpResult, FORMAT_DISCOVERY_METHODS, FormatCorrection,
    execute_brp_method_with_format_discovery,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult};
use crate::constants::{
    JSON_FIELD_FORMAT_CORRECTED, JSON_FIELD_FORMAT_CORRECTIONS, JSON_FIELD_METADATA,
    JSON_FIELD_ORIGINAL_ERROR, PARAM_ENTITY, PARAM_PARAMS, PARAM_RESOURCE, PARAM_WITH_CRATES,
    PARAM_WITH_TYPES, PARAM_WITHOUT_CRATES, PARAM_WITHOUT_TYPES,
};
use crate::response::{self, FormatterContext, ResponseFormatterFactory};
use crate::service::{BrpContext, HandlerContext};
use crate::tool::BrpMethodParamCategory;

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
    formatter_factory: &'a ResponseFormatterFactory,
    formatter_context: FormatterContext,
    method:            &'a str,
}

/// Process a successful BRP response
fn process_success_response(
    data: Option<Value>,
    enhanced_result: &EnhancedBrpResult,
    context: ResponseContext<'_>,
    handler_context: &HandlerContext<BrpContext>,
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

    // Use format_success to include call_info
    updated_formatter.format_success(&response_data, handler_context)
}

/// Process an error BRP response - routes ALL errors through enhanced `format_error_default`
fn process_error_response(
    mut error_info: BrpError,
    enhanced_result: &EnhancedBrpResult,
    handler_context: &HandlerContext<BrpContext>,
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
    response::format_error_default(error_info, handler_context)
}

/// Unified handler for all BRP methods (both static and dynamic)
pub async fn handle_brp_method_tool_call(
    handler_context: HandlerContext<BrpContext>,
    formatter_factory: &ResponseFormatterFactory,
) -> Result<CallToolResult, McpError> {
    // Log raw MCP request at the earliest possible point
    debug!("MCP ENTRY - Tool: {}", handler_context.request.name);
    trace!(
        "MCP ENTRY - Raw arguments: {}",
        serde_json::to_string(&handler_context.request.arguments)
            .unwrap_or_else(|_| "SERIALIZATION_ERROR".to_string())
    );

    // Extract parameters for the call based on the tool definition
    let (params, port) = extract_params_for_type(&handler_context)?;

    // Get the method directly from the typed context - no Options!
    let method_name = handler_context.brp_method();

    // Call BRP using format discovery
    let enhanced_result =
        execute_brp_method_with_format_discovery(method_name, params.clone(), Some(port))
            .await
            .map_err(|err| crate::error::report_to_mcp_error(&err))?;

    // Create formatter and metadata
    let formatter_context = FormatterContext {
        params,
        format_corrected: None, // Initial context doesn't have format correction status yet
    };

    // Process response using ResponseFormatter, including format corrections if present
    match &enhanced_result.result {
        BrpResult::Success(data) => {
            let context = ResponseContext {
                formatter_factory,
                formatter_context,
                method: method_name,
            };
            Ok(process_success_response(
                data.clone(),
                &enhanced_result,
                context,
                &handler_context,
            ))
        }
        BrpResult::Error(error_info) => Ok(process_error_response(
            error_info.clone(),
            &enhanced_result,
            &handler_context,
            method_name,
        )),
    }
}

/// Extract parameters based on the extractor type
fn extract_params_for_type(
    ctx: &HandlerContext<BrpContext>,
) -> Result<(Option<serde_json::Value>, u16), McpError> {
    let port = ctx.extract_port()?;
    let extractor_type = &ctx.tool_def()?.parameter_extractor;

    match extractor_type {
        BrpMethodParamCategory::Passthrough => {
            // Pass through all arguments as params
            let params = ctx.request.arguments.clone().map(serde_json::Value::Object);
            Ok((params, port))
        }
        BrpMethodParamCategory::Entity { required } => {
            // Extract entity parameter
            let params = if *required {
                let entity = ctx.get_entity_id()?;
                Some(serde_json::json!({ PARAM_ENTITY: entity }))
            } else {
                // For optional entity (like list), include it if present
                ctx.entity_id()
                    .map(|entity| serde_json::json!({ PARAM_ENTITY: entity }))
            };

            Ok((params, port))
        }
        BrpMethodParamCategory::Resource => {
            // Extract resource parameter
            let resource = ctx.extract_required_string(PARAM_RESOURCE, "resource name")?;
            let params = Some(serde_json::json!({ PARAM_RESOURCE: resource }));

            Ok((params, port))
        }
        BrpMethodParamCategory::EmptyParams => {
            // Just extract port, no other params
            Ok((None, port))
        }
        BrpMethodParamCategory::BrpExecute => {
            // Extract params for brp_execute
            let params = ctx.extract_optional_named_field(PARAM_PARAMS).cloned();

            Ok((params, port))
        }
        BrpMethodParamCategory::RegistrySchema => {
            // Extract optional filter parameters for registry schema
            let with_crates = ctx.extract_optional_string_array(PARAM_WITH_CRATES);
            let without_crates = ctx.extract_optional_string_array(PARAM_WITHOUT_CRATES);
            let with_types = ctx.extract_optional_string_array(PARAM_WITH_TYPES);
            let without_types = ctx.extract_optional_string_array(PARAM_WITHOUT_TYPES);

            let mut params_obj = serde_json::Map::new();
            if let Some(crates) = with_crates {
                params_obj.insert(PARAM_WITH_CRATES.to_string(), serde_json::json!(crates));
            }
            if let Some(crates) = without_crates {
                params_obj.insert(PARAM_WITHOUT_CRATES.to_string(), serde_json::json!(crates));
            }
            if let Some(types) = with_types {
                params_obj.insert(PARAM_WITH_TYPES.to_string(), serde_json::json!(types));
            }
            if let Some(types) = without_types {
                params_obj.insert(PARAM_WITHOUT_TYPES.to_string(), serde_json::json!(types));
            }

            let params = if params_obj.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(params_obj))
            };

            Ok((params, port))
        }
    }
}
