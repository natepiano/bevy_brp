use serde_json::{Value, json};

use super::brp_client::{BrpClientError, BrpClientResult};
use super::format_correction_fields::FormatCorrectionField;
use super::format_discovery::{EnhancedBrpResult, FormatCorrection};
use super::tools::bevy_insert::InsertFormatError;
use super::tools::bevy_insert_resource::InsertResourceFormatError;
use super::tools::bevy_mutate_component::MutateComponentFormatError;
use super::tools::bevy_mutate_resource::MutateResourceFormatError;
use super::tools::bevy_spawn::SpawnFormatError;
use super::types::{ExecuteMode, ResultStructBrpExt};
use super::{FormatCorrectionStatus, Port};
use crate::brp_tools::BrpClient;
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, ParameterName, ResultStruct};

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
fn enhance_error_message(err: &BrpClientError, enhanced_result: &EnhancedBrpResult) -> String {
    // Check if the enhanced result has a different error message
    if let BrpClientResult::Error(enhanced_error) = &enhanced_result.result {
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

/// Helper function to prepare format corrections from enhanced result
fn prepare_format_corrections(enhanced_result: &EnhancedBrpResult) -> Option<Vec<Value>> {
    if enhanced_result.format_corrections.is_empty() {
        None
    } else {
        Some(
            enhanced_result
                .format_corrections
                .iter()
                .map(format_correction_to_json)
                .collect(),
        )
    }
}

/// Helper function to create structured error result
fn create_structured_error<T, R>(error: T) -> Result<R>
where
    T: ResultStruct + 'static,
    R: Send + 'static,
{
    Err(Error::Structured {
        result: Box::new(error),
    }
    .into())
}

/// Create spawn format discovery error
fn create_spawn_error<R>(
    err: &BrpClientError,
    enhanced_message: String,
    format_corrections: Option<Vec<Value>>,
    format_corrected: Option<FormatCorrectionStatus>,
) -> Result<R>
where
    R: Send + 'static,
{
    let message = if enhanced_message == err.message {
        format!("Failed to spawn entity with components: {}", err.message)
    } else {
        enhanced_message
    };
    let error = SpawnFormatError::new(
        format_corrections,
        format_corrected,
        err.code,
        Some(err.message.clone()),
    )
    .with_message_template(message);
    create_structured_error(error)
}

/// Create insert format discovery error
fn create_insert_error<R>(
    err: &BrpClientError,
    enhanced_message: String,
    format_corrections: Option<Vec<Value>>,
    format_corrected: Option<FormatCorrectionStatus>,
    params_value: &Value,
) -> Result<R>
where
    R: Send + 'static,
{
    let entity = params_value
        .get("entity")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let message = if enhanced_message == err.message {
        format!(
            "Failed to insert components into entity {entity}: {}",
            err.message
        )
    } else {
        enhanced_message
    };
    let error = InsertFormatError::new(
        entity,
        format_corrections,
        format_corrected,
        err.code,
        Some(err.message.clone()),
    )
    .with_message_template(message);
    create_structured_error(error)
}

/// Create mutate component format discovery error
fn create_mutate_component_error<R>(
    err: &BrpClientError,
    enhanced_message: String,
    format_corrections: Option<Vec<Value>>,
    format_corrected: Option<FormatCorrectionStatus>,
    params_value: &Value,
) -> Result<R>
where
    R: Send + 'static,
{
    let entity = params_value
        .get("entity")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let path = params_value
        .get("path")
        .and_then(|v| v.as_str())
        .map(String::from);
    let message = if enhanced_message == err.message {
        path.as_ref().map_or_else(
            || {
                format!(
                    "Failed to mutate component on entity {entity}: {}",
                    err.message
                )
            },
            |p| {
                format!(
                    "Failed to mutate component field {p} on entity {entity}: {}",
                    err.message
                )
            },
        )
    } else {
        enhanced_message
    };
    let error = MutateComponentFormatError::new(
        format_corrections,
        format_corrected,
        err.code,
        Some(err.message.clone()),
    )
    .with_message_template(message);
    create_structured_error(error)
}

/// Create mutate resource format discovery error
fn create_mutate_resource_error<R>(
    err: &BrpClientError,
    enhanced_message: String,
    format_corrections: Option<Vec<Value>>,
    format_corrected: Option<FormatCorrectionStatus>,
    params_value: &Value,
) -> Result<R>
where
    R: Send + 'static,
{
    let path = params_value
        .get("path")
        .and_then(|v| v.as_str())
        .map(String::from);
    let message = if enhanced_message == err.message {
        path.as_ref().map_or_else(
            || format!("Failed to mutate resource: {}", err.message),
            |p| format!("Failed to mutate resource field {p}: {}", err.message),
        )
    } else {
        enhanced_message
    };
    let error = MutateResourceFormatError::new(
        format_corrections,
        format_corrected,
        err.code,
        Some(err.message.clone()),
    )
    .with_message_template(message);
    create_structured_error(error)
}

/// Create insert resource format discovery error
fn create_insert_resource_error<R>(
    err: &BrpClientError,
    enhanced_message: String,
    format_corrections: Option<Vec<Value>>,
    format_corrected: Option<FormatCorrectionStatus>,
) -> Result<R>
where
    R: Send + 'static,
{
    let message = if enhanced_message == err.message {
        format!("Failed to insert resource: {}", err.message)
    } else {
        enhanced_message
    };
    let error = InsertResourceFormatError::new(
        format_corrections,
        format_corrected,
        err.code,
        Some(err.message.clone()),
    )
    .with_message_template(message);
    create_structured_error(error)
}

/// Create fallback error for tools that don't have format discovery errors
fn create_fallback_error<R>(err: &BrpClientError, enhanced_message: String) -> Result<R>
where
    R: Send + 'static,
{
    Err(Error::tool_call_failed_with_details(
        enhanced_message,
        err.data.clone().unwrap_or(Value::Null),
    )
    .into())
}

fn create_format_discovery_error<R>(
    method: BrpMethod,
    err: &BrpClientError,
    enhanced_result: &EnhancedBrpResult,
    params_value: Value,
) -> Result<R>
where
    R: Send + 'static,
{
    let enhanced_message = enhance_error_message(err, enhanced_result);
    let format_corrections = prepare_format_corrections(enhanced_result);

    match method {
        BrpMethod::BevySpawn => create_spawn_error(
            err,
            enhanced_message,
            format_corrections,
            Some(enhanced_result.format_corrected.clone()),
        ),
        BrpMethod::BevyInsert => create_insert_error(
            err,
            enhanced_message,
            format_corrections,
            Some(enhanced_result.format_corrected.clone()),
            &params_value,
        ),
        BrpMethod::BevyMutateComponent => create_mutate_component_error(
            err,
            enhanced_message,
            format_corrections,
            Some(enhanced_result.format_corrected.clone()),
            &params_value,
        ),
        BrpMethod::BevyMutateResource => create_mutate_resource_error(
            err,
            enhanced_message,
            format_corrections,
            Some(enhanced_result.format_corrected.clone()),
            &params_value,
        ),
        BrpMethod::BevyInsertResource => create_insert_resource_error(
            err,
            enhanced_message,
            format_corrections,
            Some(enhanced_result.format_corrected.clone()),
        ),
        _ => create_fallback_error(err, enhanced_message),
    }
}

/// Extract common parameter processing logic used by both execution paths
fn prepare_brp_params<T: serde::Serialize>(params: T) -> Result<Option<Value>> {
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

    Ok(brp_params)
}

/// Unified BRP call handler using the enhanced `BrpClient` API
pub async fn execute_static_brp_call<P, R>(method: BrpMethod, port: Port, params: P) -> Result<R>
where
    P: serde::Serialize + Send + 'static,
    R: ResultStructBrpExt<
            Args = (
                Option<Value>,
                Option<Vec<Value>>,
                Option<FormatCorrectionStatus>,
            ),
        > + Send
        + 'static,
{
    tracing::debug!("execute_static_brp_call with extracted params");

    // Prepare parameters once
    let brp_params = prepare_brp_params(params)?;

    // Create client with appropriate configuration
    let client = BrpClient::new(method, port, brp_params);
    let client = match R::brp_tool_execute_mode() {
        ExecuteMode::WithFormatDiscovery => client.with_format_discovery(),
        ExecuteMode::Standard => client,
    };

    // Execute and get unified result
    let enhanced_result = client.execute().await?;

    // Transform to tool-specific result type
    match enhanced_result.result {
        BrpClientResult::Success(data) => {
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

            R::from_brp_client_response((
                data,
                format_corrections,
                Some(enhanced_result.format_corrected),
            ))
        }
        BrpClientResult::Error(ref err) => {
            if enhanced_result.format_corrected == FormatCorrectionStatus::NotApplicable {
                // Simple error - format discovery was disabled
                Err(Error::tool_call_failed(err.message.clone()).into())
            } else {
                // Enhanced error with format discovery context
                create_format_discovery_error::<R>(method, err, &enhanced_result, Value::Null)
            }
        }
    }
}
