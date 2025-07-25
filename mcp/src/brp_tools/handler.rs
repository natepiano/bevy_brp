use serde::Serialize;
use serde_json::{Value, json};

use super::FormatCorrectionStatus;
use super::brp_client::{BrpError, BrpResult};
use super::format_discovery::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
use crate::brp_tools::FormatCorrectionField;
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, HandlerContext, ParameterName};

/// Trait for parameter structs that have a port field
pub trait HasPortField {
    fn port(&self) -> u16;
}

/// Trait for BRP tools to provide their method at compile time
pub trait HasBrpMethod {
    /// Returns the BRP method for this tool
    fn brp_method() -> BrpMethod;
}

/// Result type for BRP method calls that follows local handler patterns
#[derive(Serialize, bevy_brp_mcp_macros::ResultFieldPlacement)]
pub struct BrpMethodResult {
    // Success data - the actual BRP response data
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    // BRP metadata - using existing field names
    // Only include if not empty - but skip_if_empty would be better than always including empty
    // vec
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrections: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrected:   Option<FormatCorrectionStatus>,
}

/// Convert a `FormatCorrection` to JSON representation with metadata
fn format_correction_to_json(correction: &FormatCorrection) -> Value {
    let mut correction_json = json!({
        FormatCorrectionField::Component.as_ref(): correction.component,
        FormatCorrectionField::OriginalFormat.as_ref(): correction.original_format,
        FormatCorrectionField::CorrectedFormat.as_ref(): correction.corrected_format,
        FormatCorrectionField::Hint.as_ref(): correction.hint
    });

    // Add rich metadata fields if available
    if let Some(obj) = correction_json.as_object_mut() {
        if let Some(ops) = &correction.supported_operations {
            obj.insert(
                FormatCorrectionField::SupportedOperations
                    .as_ref()
                    .to_string(),
                json!(ops),
            );
        }
        if let Some(paths) = &correction.mutation_paths {
            obj.insert(
                FormatCorrectionField::MutationPaths.as_ref().to_string(),
                json!(paths),
            );
        }
        if let Some(cat) = &correction.type_category {
            obj.insert(
                FormatCorrectionField::TypeCategory.as_ref().to_string(),
                json!(cat),
            );
        }
    }

    correction_json
}

/// Convert `EnhancedBrpResult` to `BrpMethodResult`
pub fn convert_to_brp_method_result(
    enhanced_result: EnhancedBrpResult,
    _ctx: &HandlerContext,
) -> Result<BrpMethodResult> {
    match enhanced_result.result {
        BrpResult::Success(data) => {
            let format_corrections = enhanced_result
                .format_corrections
                .iter()
                .map(format_correction_to_json)
                .collect::<Vec<_>>();

            Ok(BrpMethodResult {
                result:             data, // Direct BRP response data
                format_corrections: if format_corrections.is_empty() {
                    None
                } else {
                    Some(format_corrections)
                },
                format_corrected:   match enhanced_result.format_corrected {
                    FormatCorrectionStatus::NotAttempted => None,
                    other => Some(other),
                },
            })
        }
        BrpResult::Error(ref err) => {
            // Process error enhancements (reuse logic from current handler)
            let enhanced_message = enhance_error_message(err, &enhanced_result);
            let mut error_data = err.data.clone();

            // If message was enhanced, preserve the original error message
            if enhanced_message != err.message {
                let mut data_obj = error_data.unwrap_or_else(|| serde_json::json!({}));
                if let Value::Object(map) = &mut data_obj {
                    map.insert(
                        FormatCorrectionField::OriginalError.as_ref().to_string(),
                        Value::String(err.message.clone()),
                    );
                }
                error_data = Some(data_obj);
            }

            // Build Error with all the error context including format corrections
            let error_details = serde_json::json!({
                FormatCorrectionField::Code.as_ref(): err.code,
                FormatCorrectionField::ErrorData.as_ref(): error_data,
                FormatCorrectionField::FormatCorrections.as_ref(): enhanced_result
                    .format_corrections
                    .iter()
                    .map(format_correction_to_json)
                    .collect::<Vec<_>>(),
                FormatCorrectionField::FormatCorrected.as_ref(): enhanced_result.format_corrected
            });

            Err(Error::tool_call_failed_with_details(enhanced_message, error_details).into())
        }
    }
}

/// Enhance error message with format discovery insights
fn enhance_error_message(err: &BrpError, enhanced_result: &EnhancedBrpResult) -> String {
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
        let params = ctx.extract_parameter_values::<T>()?;
        let port = params.port(); // Type-safe port access through trait
        let mut params_json = serde_json::to_value(params)
            .map_err(|e| Error::InvalidArgument(format!("Failed to serialize parameters: {e}")))?;

        // Filter out null values and port field - BRP expects parameters to be
        // omitted entirely rather than explicitly null, and port is MCP-specific
        let brp_params = if let Value::Object(ref mut map) = params_json {
            map.retain(|key, value| !value.is_null() && key != ParameterName::Port.as_ref());
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
        let method = Tool::brp_method();
        let enhanced_result =
            execute_brp_method_with_format_discovery(method, brp_params, port).await?;

        // Convert result using existing conversion function
        convert_to_brp_method_result(enhanced_result, &ctx)
    }
}
