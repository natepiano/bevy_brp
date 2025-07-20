use serde_json::{Value, json};

use super::format_discovery::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
use super::types::BrpMethodResult;
use crate::brp_tools::support::brp_client::BrpResult;
use crate::error;
use crate::response::ToolError;
use crate::tool::{
    BrpToolFn, HandlerContext, HandlerResponse, HandlerResult, HasMethod, HasPort, ToolResult,
};

/// BRP method handler V2 that implements `BrpToolFnV2` and returns `HandlerResponse`
pub struct BrpMethodHandlerV2;

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

impl BrpToolFn for BrpMethodHandlerV2 {
    fn call(&self, ctx: &HandlerContext<HasPort, HasMethod>) -> HandlerResponse<'_> {
        let ctx = ctx.clone();

        Box::pin(async move {
            // Extract parameters for the call based on the tool definition
            let params = ctx.extract_params_from_definition()?;
            let method_name = ctx.brp_method();

            // Execute with format discovery (reuse existing function)
            let enhanced_result =
                execute_brp_method_with_format_discovery(method_name, params, ctx.port())
                    .await
                    .map_err(|err| error::report_to_mcp_error(&err))?;

            // Convert to BrpMethodResult
            let result = convert_to_brp_method_result(enhanced_result, &ctx);
            let tool_result = ToolResult(result);
            Ok(Box::new(tool_result) as Box<dyn HandlerResult>)
        })
    }
}

/// Convert `EnhancedBrpResult` to `BrpMethodResult`
fn convert_to_brp_method_result<Port, Method>(
    enhanced_result: EnhancedBrpResult,
    ctx: &HandlerContext<Port, Method>,
) -> Result<BrpMethodResult, ToolError> {
    match enhanced_result.result {
        BrpResult::Success(data) => {
            Ok(BrpMethodResult {
                result:             data, // Direct BRP response data
                format_corrections: enhanced_result
                    .format_corrections
                    .iter()
                    .map(format_correction_to_json)
                    .collect(),
                format_corrected:   Some(enhanced_result.format_corrected),
            })
        }
        BrpResult::Error(ref err) => {
            // Process error enhancements (reuse logic from current handler)
            let enhanced_message = enhance_error_message(err, &enhanced_result, ctx);
            let mut error_data = enhance_error_data(err.data.clone(), &enhanced_result, ctx);

            // If message was enhanced, preserve the original error message
            if enhanced_message != err.message {
                let mut data_obj = error_data.unwrap_or_else(|| serde_json::json!({}));
                if let Value::Object(map) = &mut data_obj {
                    map.insert(
                        "original_error".to_string(),
                        Value::String(err.message.clone()),
                    );
                }
                error_data = Some(data_obj);
            }

            // Build ToolError with all the error context including format corrections
            let mut tool_error = ToolError::new(enhanced_message);
            tool_error.details = Some(serde_json::json!({
                "code": err.code,
                "error_data": error_data,
                "format_corrections": enhanced_result
                    .format_corrections
                    .iter()
                    .map(format_correction_to_json)
                    .collect::<Vec<_>>(),
                "format_corrected": enhanced_result.format_corrected
            }));

            Err(tool_error)
        }
    }
}

/// Enhance error message with format discovery insights
fn enhance_error_message<Port, Method>(
    err: &crate::brp_tools::support::brp_client::BrpError,
    enhanced_result: &EnhancedBrpResult,
    _ctx: &HandlerContext<Port, Method>,
) -> String {
    // Check if the enhanced result has a different error message
    if let BrpResult::Error(enhanced_error) = &enhanced_result.result {
        if enhanced_error.message != err.message {
            return enhanced_error.message.clone();
        }
    }

    // Check format corrections for educational hints
    if let Some(correction) = enhanced_result
        .format_corrections
        .iter()
        .find(|c| c.hint.contains("cannot be used with BRP"))
    {
        return correction.hint.clone();
    }

    // Use original message
    err.message.clone()
}

/// Enhance error data with format corrections
const fn enhance_error_data<Port, Method>(
    original_data: Option<Value>,
    _enhanced_result: &EnhancedBrpResult,
    _ctx: &HandlerContext<Port, Method>,
) -> Option<Value> {
    // V2 handler no longer duplicates format correction data in error_data
    // Format corrections are handled through ResponseField::FormatCorrection extractor
    // which extracts from the main BrpMethodResult.format_corrections field
    original_data
}
