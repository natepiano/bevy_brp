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
    JSON_FIELD_ORIGINAL_ERROR, PARAM_ENTITY, PARAM_METHOD, PARAM_PORT,
};
use crate::response::{self, FormatterContext, ResponseFormatterFactory};
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
    let params = extract_params_from_definition(&handler_context)?;

    // Get the method directly from the typed context - no Options!
    let method_name = handler_context.brp_method();

    // Call BRP using format discovery
    let enhanced_result = execute_brp_method_with_format_discovery(
        method_name,
        params.clone(),
        handler_context.extract_port()?,
    )
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

/// Extract parameters from tool definition instead of using category
#[allow(clippy::too_many_lines)]
fn extract_params_from_definition(
    ctx: &HandlerContext<BrpContext>,
) -> Result<Option<serde_json::Value>, McpError> {
    use crate::tool::ParamType;

    // Get the tool definition
    let tool_def = ctx.tool_def()?;

    // Special case: brp_execute tool handles "method" separately
    let is_brp_execute = tool_def.name == "brp_execute";

    // Build params from parameter definitions
    let mut params_obj = serde_json::Map::new();
    let mut has_params = false;

    for param in &tool_def.parameters {
        // Skip method parameter for brp_execute (it's extracted separately)
        if is_brp_execute && param.name() == PARAM_METHOD {
            continue;
        }

        // Extract parameter value based on type
        let value = match param.param_type() {
            ParamType::Number => {
                if param.required() {
                    // Special handling for entity parameter
                    if param.name() == PARAM_ENTITY {
                        Some(json!(ctx.get_entity_id()?))
                    } else {
                        Some(json!(
                            ctx.extract_required_u64(param.name(), param.description())?
                        ))
                    }
                } else {
                    ctx.extract_optional_named_field(param.name())
                        .and_then(serde_json::Value::as_u64)
                        .map(|v| json!(v))
                }
            }
            ParamType::String => {
                if param.required() {
                    Some(json!(
                        ctx.extract_required_string(param.name(), param.description())?
                    ))
                } else {
                    ctx.extract_optional_named_field(param.name())
                        .and_then(|v| v.as_str())
                        .map(|s| json!(s))
                }
            }
            ParamType::Boolean => {
                if param.required() {
                    // For required boolean, we need to check if it exists and is a bool
                    let value = ctx
                        .extract_optional_named_field(param.name())
                        .and_then(serde_json::Value::as_bool)
                        .ok_or_else(|| {
                            crate::error::report_to_mcp_error(
                                &error_stack::Report::new(crate::error::Error::InvalidArgument(
                                    format!("Missing {} parameter", param.description()),
                                ))
                                .attach_printable(format!("Field name: {}", param.name()))
                                .attach_printable("Expected: boolean value"),
                            )
                        })?;
                    Some(json!(value))
                } else {
                    ctx.extract_optional_named_field(param.name())
                        .and_then(serde_json::Value::as_bool)
                        .map(|v| json!(v))
                }
            }
            ParamType::StringArray => {
                if param.required() {
                    let array = ctx
                        .extract_optional_string_array(param.name())
                        .ok_or_else(|| {
                            crate::error::report_to_mcp_error(
                                &error_stack::Report::new(crate::error::Error::InvalidArgument(
                                    format!("Missing {} parameter", param.description()),
                                ))
                                .attach_printable(format!("Field name: {}", param.name()))
                                .attach_printable("Expected: array of strings"),
                            )
                        })?;
                    Some(json!(array))
                } else {
                    ctx.extract_optional_string_array(param.name())
                        .map(|v| json!(v))
                }
            }
            ParamType::Any => {
                if param.required() {
                    let value = ctx
                        .extract_optional_named_field(param.name())
                        .ok_or_else(|| {
                            crate::error::report_to_mcp_error(
                                &error_stack::Report::new(crate::error::Error::InvalidArgument(
                                    format!("Missing {} parameter", param.description()),
                                ))
                                .attach_printable(format!("Field name: {}", param.name()))
                                .attach_printable("Expected: JSON value"),
                            )
                        })?;
                    Some(value.clone())
                } else {
                    ctx.extract_optional_named_field(param.name()).cloned()
                }
            }
        };

        // Add to params if value exists
        if let Some(val) = value {
            params_obj.insert(param.name().to_string(), val);
            has_params = true;
        }
    }

    // Special case: For passthrough tools (tools that had Passthrough category),
    // we need to pass all arguments except port
    // We can detect these by checking if they have no parameters defined
    if tool_def.parameters.is_empty() && ctx.request.arguments.is_some() {
        // This is a passthrough tool - pass all arguments except port
        if let Some(args) = &ctx.request.arguments {
            let mut passthrough_args = args.clone();
            passthrough_args.remove(PARAM_PORT);

            if !passthrough_args.is_empty() {
                return Ok(Some(Value::Object(passthrough_args)));
            }
        }
    }

    // Return params
    let params = if has_params {
        Some(Value::Object(params_obj))
    } else {
        None
    };

    Ok(params)
}
