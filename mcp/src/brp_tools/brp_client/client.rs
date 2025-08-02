//! BRP (Bevy Remote Protocol) client with unified execution interface
//!
//! This module provides a streamlined interface for communicating with BRP servers.
//! The `BrpClient` offers exactly 3 execution methods:
//! - `execute<R>()`: Primary API with automatic format discovery for result types that support it
//! - `execute_raw()`: Low-level API for debugging and format discovery engine
//! - `execute_streaming()`: Specialized API for watch operations with streaming responses
//!
//! All BRP logic is centralized in this client, eliminating the need for scattered execution
//! functions.

use serde_json::Value;
use tracing::warn;

use super::super::Port;
use super::constants::{BRP_EXTRAS_PREFIX, JSON_RPC_ERROR_METHOD_NOT_FOUND};
use super::format_correction_fields::FormatCorrectionField;
use super::format_discovery::{
    CorrectionInfo, FormatCorrection, FormatCorrectionStatus, FormatDiscoveryEngine,
    FormatRecoveryResult,
};
use super::http_client::BrpHttpClient;
use super::types::{
    BrpClientCallJsonResponse, BrpClientError, ExecuteMode, FormatDiscoveryError, ResponseStatus,
    ResultStructBrpExt,
};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, ParameterName};

/// Client for executing a BRP operation
pub struct BrpClient {
    method: BrpMethod,
    port:   Port,
    params: Option<Value>,
}

impl BrpClient {
    /// Create a new BRP client for the given method, port, and parameters
    pub const fn new(method: BrpMethod, port: Port, params: Option<Value>) -> Self {
        Self {
            method,
            port,
            params,
        }
    }

    /// Prepare parameters for BRP calls by filtering nulls and removing MCP-specific fields
    pub fn prepare_params<T: serde::Serialize>(params: T) -> Result<Option<Value>> {
        let mut params_json = serde_json::to_value(params)
            .map_err(|e| Error::InvalidArgument(format!("Failed to serialize parameters: {e}")))?;

        // Filter out null values and port field
        let brp_params = if let Value::Object(ref mut map) = params_json {
            map.retain(|key, value| !value.is_null() && key != ParameterName::Port.as_ref());
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

    /// Primary execution method with automatic format discovery support
    ///
    /// This method implements the "execute-fail-discover" pattern:
    /// 1. Always executes the BRP request directly first
    /// 2. On success, returns the typed result immediately
    /// 3. On format errors, attempts format discovery if the result type supports it
    /// 4. Retries with corrected format if discovery succeeds
    ///
    /// Format discovery is only attempted for result types with `ExecuteMode::WithFormatDiscovery`.
    /// Result types with `ExecuteMode::DirectOnly` will return errors immediately without
    /// discovery.
    pub async fn execute<R>(&self) -> Result<R>
    where
        R: ResultStructBrpExt<
                Args = (
                    Option<Value>,
                    Option<Vec<Value>>,
                    Option<FormatCorrectionStatus>,
                ),
            > + Send
            + 'static,
    {
        // ALWAYS execute direct first
        let direct_result = self.execute_direct_internal().await?;

        match direct_result {
            ResponseStatus::Success(data) => {
                // Success - no format discovery needed
                R::from_brp_client_response((
                    data,
                    None,
                    Some(FormatCorrectionStatus::NotAttempted),
                ))
            }
            ResponseStatus::Error(err) => {
                // Only try format discovery if: 1) format error, 2) type supports it
                if err.is_format_error()
                    && matches!(R::brp_tool_execute_mode(), ExecuteMode::WithFormatDiscovery)
                {
                    // Try format discovery and maybe retry with corrected format
                    self.try_format_recovery::<R>(&err).await
                } else {
                    // Regular error - no format discovery
                    Err(Error::tool_call_failed(err.message).into())
                }
            }
        }
    }

    /// Low-level BRP execution without format discovery or result transformation
    ///
    /// This method provides direct access to BRP communication without any automatic
    /// format discovery or result type conversion. It returns raw `BrpClientResult`
    /// which can be either `Success(Option<Value>)` or `Error(BrpClientError)`.
    ///
    /// Primary use cases:
    /// - Debugging tools that need raw BRP responses (`brp_execute`)
    /// - Format discovery engine internal operations
    /// - Testing and diagnostic scenarios
    pub async fn execute_raw(&self) -> Result<ResponseStatus> {
        self.execute_direct_internal().await
    }

    /// Execute the BRP request and return a streaming response
    ///
    /// This method is designed for watch operations that need to handle
    /// Server-Sent Events (SSE) streams. Unlike `execute()`, it:
    /// - Uses no timeout (streaming connections stay open)
    /// - Returns the raw response for the caller to process
    /// - Provides the same rich error context as other `BrpClient` methods
    pub async fn execute_streaming(&self) -> Result<reqwest::Response> {
        // Create HTTP client with our data
        let http_client = BrpHttpClient::new(self.method, self.port, self.params.clone());

        // Send HTTP request using streaming version (no timeout, includes status check)
        let response = http_client.send_streaming_request().await?;

        Ok(response)
    }

    /// Internal direct execution - does the actual http call - we wanted the internal vresion so we
    /// can distinguish a canned call generated for a `ToolFn` by our macro, and the `execute_raw()`
    /// version we still allow to be called by bespoke tools like `brp_shutdown` and `brp_status`
    /// and the like.
    async fn execute_direct_internal(&self) -> Result<ResponseStatus> {
        // Create HTTP client with our data
        let http_client = BrpHttpClient::new(self.method, self.port, self.params.clone());

        // Send HTTP request (includes status check)
        let response = http_client.send_request().await?;

        // Parse JSON-RPC response
        let brp_response = self.parse_json_response(response).await?;

        // Convert to BrpClientResult with special handling for bevy_brp_extras
        Ok(self.to_response_status(brp_response))
    }

    /// Try format recovery and retry with corrected format
    async fn try_format_recovery<R>(&self, original_error: &BrpClientError) -> Result<R>
    where
        R: ResultStructBrpExt<
                Args = (
                    Option<Value>,
                    Option<Vec<Value>>,
                    Option<FormatCorrectionStatus>,
                ),
            > + Send
            + 'static,
    {
        // Validate that parameters exist for format discovery
        let Some(params) = self.params.clone() else {
            return Err(Error::InvalidArgument(
                "Format discovery requires parameters to extract type information".to_string(),
            )
            .into());
        };

        let engine = FormatDiscoveryEngine::new(self.method, self.port);
        let recovery_result = engine
            .try_recovery_and_retry(params, original_error)
            .await?;

        // Transform recovery result to appropriate error or success
        transform_recovery_result::<R>(recovery_result, original_error)
    }

    /// Parse the JSON response from the BRP server
    async fn parse_json_response(
        &self,
        response: reqwest::Response,
    ) -> Result<BrpClientCallJsonResponse> {
        match response.json().await {
            Ok(json_resp) => Ok(json_resp),
            Err(e) => {
                warn!("BRP execute_brp_method: JSON parsing failed - error={}", e);
                Err(
                    error_stack::Report::new(Error::JsonRpc("JSON parsing failed".to_string()))
                        .attach_printable("Failed to parse BRP response JSON")
                        .attach_printable(format!(
                            "Method: {}, Port: {}",
                            self.method.as_str(),
                            self.port
                        ))
                        .attach_printable(format!("Error: {e}")),
                )
            }
        }
    }

    /// Convert `BrpClientResponse` to `BrpClientResult`
    fn to_response_status(&self, brp_response_json: BrpClientCallJsonResponse) -> ResponseStatus {
        if let Some(error) = brp_response_json.error {
            warn!(
                "BRP execute_brp_method: BRP returned error - code={}, message={}",
                error.code, error.message
            );

            // Check if this is a bevy_brp_extras method that's not found
            let enhanced_message = if error.code == JSON_RPC_ERROR_METHOD_NOT_FOUND
                && self.method.as_str().starts_with(BRP_EXTRAS_PREFIX)
            {
                format!(
                    "{}. This method requires the bevy_brp_extras crate to be added to your Bevy app with the BrpExtrasPlugin",
                    error.message
                )
            } else {
                error.message
            };

            ResponseStatus::Error(BrpClientError {
                code:    error.code,
                message: enhanced_message,
                data:    error.data,
            })
        } else {
            ResponseStatus::Success(brp_response_json.result)
        }
    }
}

/// Convert a `FormatCorrection` to JSON representation with metadata
fn format_correction_to_json(correction: &FormatCorrection) -> Value {
    let mut correction_json = serde_json::json!({
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
                serde_json::json!(ops),
            );
        }
        if let Some(paths) = &correction.mutation_paths {
            obj.insert(
                FormatCorrectionField::MutationPaths.as_ref().to_string(),
                serde_json::json!(paths),
            );
        }
        if let Some(cat) = &correction.type_category {
            obj.insert(
                FormatCorrectionField::TypeCategory.as_ref().to_string(),
                serde_json::json!(cat),
            );
        }
    }

    correction_json
}

/// Create enhanced error for format discovery failures
fn create_format_discovery_error(
    original_error: &BrpClientError,
    reason: &str,
    corrections: &[CorrectionInfo],
) -> Error {
    // Build format corrections array with metadata
    // Always include the array (even if empty) to meet test expectations
    let format_corrections = Some(
        corrections
            .iter()
            .map(|c| {
                let correction = FormatCorrection {
                    component:            c.type_name.clone(),
                    original_format:      c.original_value.clone(),
                    corrected_format:     c.corrected_value.clone(),
                    hint:                 c.hint.clone(),
                    supported_operations: c
                        .type_info
                        .as_ref()
                        .map(|ti| ti.supported_operations.clone()),
                    mutation_paths:       c.type_info.as_ref().and_then(|ti| {
                        let paths = &ti.format_info.mutation_paths;
                        if paths.is_empty() {
                            None
                        } else {
                            Some(paths.keys().cloned().collect())
                        }
                    }),
                    type_category:        c.type_info.as_ref().map(|ti| {
                        // Use debug format since TypeCategory is not publicly accessible
                        format!("{:?}", ti.type_category)
                    }),
                };
                format_correction_to_json(&correction)
            })
            .collect(),
    );

    // Build hint message from corrections
    let hint = if corrections.is_empty() {
        "No format corrections available. Check that the types have Serialize/Deserialize traits."
            .to_string()
    } else {
        corrections
            .iter()
            .filter_map(|c| {
                if c.hint.is_empty() {
                    None
                } else {
                    Some(format!("- {}: {}", c.type_name, c.hint))
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let format_discovery_error = FormatDiscoveryError::new(
        "not_attempted".to_string(),
        if hint.is_empty() {
            "Format discovery found issues but could not provide specific guidance.".to_string()
        } else {
            hint
        },
        format_corrections,
        Some(original_error.code),
        reason.to_string(),
        original_error.message.clone(),
    );

    Error::Structured {
        result: Box::new(format_discovery_error),
    }
}

/// Convert `CorrectionInfo` to `FormatCorrection` (will be used in Phase 2)
fn convert_corrections(corrections: Vec<CorrectionInfo>) -> Vec<FormatCorrection> {
    corrections
        .into_iter()
        .map(|info| {
            // Extract rich metadata from type_info if available
            let (supported_operations, mutation_paths, type_category) = info
                .type_info
                .as_ref()
                .map_or((None, None, None), |type_info| {
                    (
                        Some(type_info.supported_operations.clone()),
                        Some(
                            type_info
                                .format_info
                                .mutation_paths
                                .keys()
                                .cloned()
                                .collect(),
                        ),
                        // Convert TypeCategory to string using serde serialization
                        serde_json::to_value(&type_info.type_category)
                            .ok()
                            .and_then(|v| v.as_str().map(ToString::to_string)),
                    )
                });

            FormatCorrection {
                component: info.type_name,
                original_format: info.original_value,
                corrected_format: info.corrected_value,
                hint: info.hint,
                supported_operations,
                mutation_paths,
                type_category,
            }
        })
        .collect()
}

/// Transform format recovery result into typed result
fn transform_recovery_result<R>(
    recovery_result: FormatRecoveryResult,
    original_error: &BrpClientError,
) -> Result<R>
where
    R: ResultStructBrpExt<
        Args = (
            Option<Value>,
            Option<Vec<Value>>,
            Option<FormatCorrectionStatus>,
        ),
    >,
{
    match recovery_result {
        FormatRecoveryResult::Recovered {
            corrected_result,
            corrections,
        } => {
            // Successfully recovered with format corrections
            // Extract the success value from corrected_result
            match corrected_result {
                ResponseStatus::Success(value) => {
                    // Convert CorrectionInfo to FormatCorrection if needed
                    let format_corrections = convert_corrections(corrections);
                    R::from_brp_client_response((
                        value,
                        Some(
                            format_corrections
                                .into_iter()
                                .map(|c| format_correction_to_json(&c))
                                .collect(),
                        ),
                        Some(FormatCorrectionStatus::Succeeded),
                    ))
                }
                ResponseStatus::Error(err) => {
                    // Recovery succeeded but result contains error - shouldn't happen
                    Err(Error::tool_call_failed(format!(
                        "Format recovery succeeded but result contains error: {}",
                        err.message
                    ))
                    .into())
                }
            }
        }
        FormatRecoveryResult::NotRecoverable { corrections } => {
            // Format discovery couldn't fix it but has guidance
            let enhanced_error = create_format_discovery_error(
                original_error,
                "Format errors not recoverable but guidance available",
                &corrections,
            );
            Err(enhanced_error.into())
        }
        FormatRecoveryResult::CorrectionFailed {
            retry_error,
            corrections,
        } => {
            // Format discovery tried but the correction failed
            let retry_error_msg = match retry_error {
                ResponseStatus::Error(ref err) => &err.message,
                ResponseStatus::Success(_) => "Unknown error",
            };
            let enhanced_error = create_format_discovery_error(
                original_error,
                &format!("Correction attempted but failed: {retry_error_msg}"),
                &corrections,
            );
            Err(enhanced_error.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // Note: We can't directly test convert_corrections because CorrectionInfo creation
    // requires private types. The integration tests in format_discovery.md will verify
    // the full flow. Here we test the JSON serialization which is what the API returns.

    #[test]
    #[allow(clippy::expect_used)]
    fn test_format_correction_to_json() {
        // Test that format_correction_to_json properly includes metadata when available
        let correction_with_metadata = FormatCorrection {
            component:            "bevy_transform::components::transform::Transform".to_string(),
            original_format:      json!({"translation": {"x": 1.0, "y": 2.0, "z": 3.0}}),
            corrected_format:     json!({"translation": [1.0, 2.0, 3.0]}),
            hint:                 "Transformed to array format".to_string(),
            supported_operations: Some(vec!["spawn".to_string(), "insert".to_string()]),
            mutation_paths:       Some(vec![
                ".translation.x".to_string(),
                ".translation.y".to_string(),
            ]),
            type_category:        Some("Component".to_string()),
        };

        let json_output = format_correction_to_json(&correction_with_metadata);

        // Verify all fields are present
        assert_eq!(
            json_output["component"],
            "bevy_transform::components::transform::Transform"
        );
        assert_eq!(json_output["hint"], "Transformed to array format");
        assert_eq!(
            json_output["supported_operations"],
            json!(["spawn", "insert"])
        );
        assert_eq!(
            json_output["mutation_paths"],
            json!([".translation.x", ".translation.y"])
        );
        assert_eq!(json_output["type_category"], "Component");

        // Test without metadata
        let correction_without_metadata = FormatCorrection {
            component:            "TestType".to_string(),
            original_format:      json!({}),
            corrected_format:     json!({}),
            hint:                 "Test".to_string(),
            supported_operations: None,
            mutation_paths:       None,
            type_category:        None,
        };

        let json_output = format_correction_to_json(&correction_without_metadata);

        // Verify metadata fields are not present when None
        let json_obj = json_output
            .as_object()
            .expect("JSON output should be an object");
        assert!(!json_obj.contains_key("supported_operations"));
        assert!(!json_obj.contains_key("mutation_paths"));
        assert!(!json_obj.contains_key("type_category"));
    }
}
