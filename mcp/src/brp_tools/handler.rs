use serde_json::{Value, json};

use super::brp_client::{BrpError, BrpResult};
use super::format_discovery::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
use super::{FormatCorrectionStatus, Port};
use crate::brp_tools::FormatCorrectionField;
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, ParameterName};

/// Trait for parameter structs that have a port field
pub trait HasPortField {
    fn port(&self) -> Port;
}

/// Trait for BRP tools to provide their method at compile time
pub trait HasBrpMethod {
    /// Returns the BRP method for this tool
    fn brp_method() -> BrpMethod;
}

/// Trait for converting BRP responses to result types
pub trait FromBrpValue: Sized {
    type Args;
    fn from_brp_value(args: Self::Args) -> Result<Self>;
}

/// Convert a `FormatCorrection` to JSON representation with metadata
pub fn format_correction_to_json(correction: &FormatCorrection) -> Value {
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

/// Extract common parameter processing logic used by both execution paths
fn prepare_brp_params<T: serde::Serialize + HasPortField>(
    params: T,
) -> Result<(Port, Option<Value>)> {
    let port = params.port();

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

    Ok((port, brp_params))
}

/// Shared implementation for all static BRP tools
/// This function handles the common pattern of:
/// 1. Extract typed parameters
/// 2. Convert to JSON for BRP call
/// 3. Use `Tool::brp_method()` for compile-time method name
/// 4. Use params.port for typed port parameter
/// 5. Call shared BRP infrastructure
/// 6. Convert result to tool's `ResultStruct`
pub async fn execute_static_brp_call_with_format_discovery<Tool, P, R>(params: P) -> Result<R>
where
    Tool: HasBrpMethod,
    P: serde::Serialize + HasPortField + Send + 'static,
    R: FromBrpValue<
        Args = (
            Option<Value>,
            Option<Vec<Value>>,
            Option<FormatCorrectionStatus>,
        ),
    >, // 3-param version
{
    tracing::debug!("execute_static_brp_call_with_format_discovery with extracted params");

    // Use shared parameter processing
    let (port, brp_params) = prepare_brp_params(params)?;
    let method = Tool::brp_method();

    // Execute with format discovery
    let enhanced_result =
        execute_brp_method_with_format_discovery(method, brp_params, port).await?;

    match enhanced_result.result {
        BrpResult::Success(data) => {
            // Format discovery tools know how to convert from enhanced result
            let format_corrections = if enhanced_result.format_corrections.is_empty() {
                None
            } else {
                Some(
                    enhanced_result
                        .format_corrections
                        .iter()
                        .map(format_correction_to_json)
                        .collect(),
                )
            };

            // Call the 3-parameter from_brp_value
            R::from_brp_value((
                data,
                format_corrections,
                Some(enhanced_result.format_corrected),
            ))
        }
        BrpResult::Error(ref err) => {
            // NOTE: Error handling kept as-is - will be converted to StructuredError pattern
            // in the subsequent format discovery migration
            let enhanced_message = enhance_error_message(err, &enhanced_result);
            let error_details = json!({
                "code": err.code,
                "error_data": err.data,
                "format_corrections": enhanced_result.format_corrections.iter()
                    .map(format_correction_to_json)
                    .collect::<Vec<_>>(),
                "format_corrected": enhanced_result.format_corrected
            });

            Err(Error::tool_call_failed_with_details(enhanced_message, error_details).into())
        }
    }
}

/// Simplified BRP call for tools that don't support format discovery
pub async fn execute_static_brp_call_simple<Tool, P, R>(params: P) -> Result<R>
where
    Tool: HasBrpMethod,
    P: serde::Serialize + HasPortField + Send + 'static,
    R: FromBrpValue<Args = Option<Value>>, // Ensures R has 1-param from_brp_value
{
    tracing::debug!("execute_static_brp_call_simple with extracted params");

    // Use shared parameter processing
    let (port, brp_params) = prepare_brp_params(params)?;
    let method = Tool::brp_method();

    // Direct BRP execution without format discovery
    let result = crate::brp_tools::execute_brp_method(method, brp_params, port).await?;

    // Convert directly to tool's ResultStruct
    match result {
        BrpResult::Success(data) => R::from_brp_value(data),
        BrpResult::Error(err) => Err(Error::tool_call_failed(err.message).into()),
    }
}
