use serde::Serialize;
use serde_json::{Value, json};

use super::FormatCorrectionStatus;
use super::brp_client::{BrpError, BrpResult};
use super::format_discovery::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
use crate::error::{Error, Result};

/// Trait for parameter structs that have a port field
pub trait HasPortField {
    fn port(&self) -> u16;
}

use crate::tool::{HandlerContext, HasBrpMethod};

/// Result type for BRP method calls that follows local handler patterns
#[derive(Serialize)]
pub struct BrpMethodResult {
    // Success data - the actual BRP response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    // BRP metadata - using existing field names
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub format_corrections: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_corrected:   Option<FormatCorrectionStatus>,
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

/// Convert `EnhancedBrpResult` to `BrpMethodResult`
pub fn convert_to_brp_method_result(
    enhanced_result: EnhancedBrpResult,
    ctx: &HandlerContext,
) -> Result<BrpMethodResult> {
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
            let mut error_data = err.data.clone();

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

            // Build Error with all the error context including format corrections
            let error_details = serde_json::json!({
                "code": err.code,
                "error_data": error_data,
                "format_corrections": enhanced_result
                    .format_corrections
                    .iter()
                    .map(format_correction_to_json)
                    .collect::<Vec<_>>(),
                "format_corrected": enhanced_result.format_corrected
            });

            Err(Error::tool_call_failed_with_details(enhanced_message, error_details).into())
        }
    }
}

/// Enhance error message with format discovery insights
fn enhance_error_message(
    err: &BrpError,
    enhanced_result: &EnhancedBrpResult,
    _ctx: &HandlerContext,
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


/// Shared implementation for all static BRP tools
/// This function handles the common pattern of:
/// 1. Extract typed parameters
/// 2. Convert to JSON for BRP call
/// 3. Use `Tool::brp_method()` for compile-time method name
/// 4. Use params.port for typed port parameter
/// 5. Call shared BRP infrastructure
/// 6. Convert result to `BrpMethodResult`
pub fn execute_static_brp_call<Tool, T>(
    ctx: &HandlerContext,
) -> impl std::future::Future<Output = Result<BrpMethodResult>> + Send + 'static
where
    Tool: HasBrpMethod,
    T: serde::de::DeserializeOwned + serde::Serialize + HasPortField + Send,
{
    let ctx = ctx.clone();

    async move {
        // Extract typed parameters
        let params = ctx.extract_typed_params::<T>()?;
        let port = params.port(); // Type-safe port access through trait
        let mut params_json = serde_json::to_value(params)
            .map_err(|e| Error::InvalidArgument(format!("Failed to serialize parameters: {e}")))?;

        // Filter out null values and port field - BRP expects parameters to be
        // omitted entirely rather than explicitly null, and port is MCP-specific
        let brp_params = if let Value::Object(ref mut map) = params_json {
            map.retain(|key, value| !value.is_null() && key != "port");
            // If the object is empty after filtering, send None to BRP
            if map.is_empty() {
                None
            } else {
                Some(params_json)
            }
        } else {
            Some(params_json)
        };

        // Use Tool::brp_method() to get method from trait at compile time
        let enhanced_result =
            execute_brp_method_with_format_discovery(Tool::brp_method(), brp_params, port).await?;

        // Convert result using existing conversion function
        convert_to_brp_method_result(enhanced_result, &ctx)
    }
}
