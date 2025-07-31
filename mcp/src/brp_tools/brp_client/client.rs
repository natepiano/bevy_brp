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

use std::time::Duration;

use bevy_brp_mcp_macros::ResultStruct;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};

use super::super::Port;
use super::super::constants::{
    BRP_ERROR_ACCESS_ERROR, BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE, JSON_RPC_ERROR_INTERNAL_ERROR,
    JSON_RPC_ERROR_INVALID_PARAMS, JSON_RPC_ERROR_METHOD_NOT_FOUND,
};
use super::super::format_correction_fields::FormatCorrectionField;
use super::super::format_discovery::{
    CorrectionInfo, FormatCorrection, FormatCorrectionStatus, FormatRecoveryResult,
    try_format_recovery_and_retry,
};
use super::super::types::{ExecuteMode, ResultStructBrpExt};
use super::constants::{BRP_DEFAULT_HOST, BRP_EXTRAS_PREFIX, BRP_HTTP_PROTOCOL, BRP_JSONRPC_PATH};
use super::json_rpc_builder::BrpJsonRpcBuilder;
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, JsonFieldAccess, ParameterName};

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
    pub async fn execute<R>(self) -> Result<R>
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
        // Store params for potential format discovery
        let params_for_discovery = self.params.clone();
        let method = self.method;
        let port = self.port;

        // ALWAYS execute direct first
        let direct_result = self.execute_direct_internal().await?;

        match direct_result {
            BrpClientResult::Success(data) => {
                // Success - no format discovery needed
                R::from_brp_client_response((
                    data,
                    None,
                    Some(FormatCorrectionStatus::NotAttempted),
                ))
            }
            BrpClientResult::Error(err) => {
                // Only try format discovery if: 1) format error, 2) type supports it
                if err.is_format_error()
                    && matches!(R::brp_tool_execute_mode(), ExecuteMode::WithFormatDiscovery)
                {
                    // Try format discovery and maybe retry with corrected format
                    let recovery_result = try_format_recovery_and_retry(
                        method,
                        params_for_discovery.clone(),
                        port,
                        &err,
                    )
                    .await?;
                    // Transform recovery result to appropriate error or success
                    transform_recovery_result::<R>(recovery_result, &err)
                } else {
                    // Regular error - no format discovery
                    Err(Error::tool_call_failed(err.message).into())
                }
            }
        }
    }

    /// Internal direct execution - does the actual http call - we wanted the internal vresion so we
    /// can distinguish a canned call generated for a `ToolFn` by our macro, and the `execute_raw()`
    /// version we still allow to be called by bespoke tools like `brp_shutdown` and `brp_status`
    /// and the like.
    async fn execute_direct_internal(self) -> Result<BrpClientResult> {
        let url = Self::build_brp_url(self.port);
        let method_str = self.method.as_str();

        // Build JSON-RPC request body
        let request_body = build_request_body(method_str, self.params);

        // Send HTTP request
        let response = send_http_request(&url, request_body, method_str, self.port).await?;

        // Check HTTP status
        check_http_status(&response, method_str, self.port)?;

        // Parse JSON-RPC response
        let brp_response = parse_json_response(response, method_str, self.port).await?;

        // Convert to BrpClientResult with special handling for bevy_brp_extras
        Ok(convert_to_brp_result(brp_response, method_str))
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
    pub async fn execute_raw(self) -> Result<BrpClientResult> {
        self.execute_direct_internal().await
    }

    /// Execute the BRP request and return a streaming response
    ///
    /// This method is designed for watch operations that need to handle
    /// Server-Sent Events (SSE) streams. Unlike `execute()`, it:
    /// - Uses no timeout (streaming connections stay open)
    /// - Returns the raw response for the caller to process
    /// - Provides the same rich error context as other `BrpClient` methods
    pub async fn execute_streaming(self) -> Result<reqwest::Response> {
        let url = Self::build_brp_url(self.port);
        let method_str = self.method.as_str();

        // Build JSON-RPC request body (reuse existing function)
        let request_body = build_request_body(method_str, self.params);

        // Create client with no timeout for streaming
        let client = reqwest::Client::builder()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        // Send HTTP request
        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(request_body.clone())
            .send()
            .await
        {
            Ok(response) => {
                // Check HTTP status
                check_http_status(&response, method_str, self.port)?;
                Ok(response)
            }
            Err(e) => handle_http_error(e, &url, &request_body, method_str, self.port),
        }
    }

    /// Build a BRP URL for the given port
    ///
    /// This is a utility function for cases where you need just the URL
    /// without executing a full BRP request (e.g., for streaming connections)
    pub fn build_brp_url(port: Port) -> String {
        format!("{BRP_HTTP_PROTOCOL}://{BRP_DEFAULT_HOST}:{port}{BRP_JSONRPC_PATH}")
    }
}

/// Result of a BRP operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrpClientResult {
    /// Successful operation with optional data
    Success(Option<Value>),
    /// Error with code, message and optional data
    Error(BrpClientError),
}

/// Error information from BRP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrpClientError {
    pub code:    i32,
    pub message: String,
    pub data:    Option<Value>,
}

impl BrpClientError {
    /// Check if this error indicates a format issue that can be recovered
    /// This function was constructed through trial and error via vibe coding with claude
    /// There is a bug in `bevy_remote` right now that we get a spurious "Unknown component type"
    /// when a Component doesn't have Serialize/Deserialize traits - this doesn't affect
    /// Resources so the first section is probably correct.
    /// the second section I think is less correct but it will take some time to validate that
    /// moving to an "error codes only" approach doesn't have other issues
    pub const fn is_format_error(&self) -> bool {
        // Common format error codes that indicate type issues
        matches!(
            self.code,
            JSON_RPC_ERROR_INVALID_PARAMS
                | JSON_RPC_ERROR_INTERNAL_ERROR
                | BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE
                | BRP_ERROR_ACCESS_ERROR
        )
    }
}

/// Raw BRP JSON-RPC response structure
#[derive(Debug, Serialize, Deserialize)]
struct BrpClientResponse {
    jsonrpc: String,
    id:      u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error:   Option<JsonRpcError>,
}

/// Raw BRP error structure from JSON-RPC response
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code:    i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data:    Option<Value>,
}

/// Build the JSON-RPC request body
fn build_request_body(method: &str, params: Option<Value>) -> String {
    let mut builder = BrpJsonRpcBuilder::new(method);
    if let Some(params) = params {
        debug!(
            "BRP execute_brp_method: Added params - {}",
            serde_json::to_string(&params)
                .unwrap_or_else(|_| "Failed to serialize params".to_string())
        );
        builder = builder.params(params);
    }
    let request_body = builder.build().to_string();

    debug!("BRP execute_brp_method: Request body - {}", request_body);

    request_body
}

/// Send the HTTP request to the BRP server
fn handle_http_error(
    e: reqwest::Error,
    url: &str,
    request_body: &str,
    method: &str,
    port: Port,
) -> Result<reqwest::Response> {
    // Always log HTTP errors to help debug intermittent failures
    warn!("BRP execute_brp_method: HTTP request failed - error={}", e);

    let error_details = format!(
        "HTTP Error at {}\nMethod: {}\nPort: {}\nURL: {}\nError: {:?}\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        method,
        port,
        url,
        e
    );
    if let Some(temp_dir) = std::env::temp_dir().to_str() {
        let error_file = format!(
            "{}/bevy_brp_http_error_{}.log",
            temp_dir,
            std::process::id()
        );
        let _ = std::fs::write(&error_file, &error_details);
        debug!("HTTP error details written to: {}", error_file);
    }

    // Extract additional context from the request body for better error reporting
    let mut context_info = vec![
        format!("Method: {method}"),
        format!("Port: {port}"),
        format!("URL: {url}"),
    ];

    // Try to parse request body to extract component/entity info for mutations
    if let Ok(body_json) = serde_json::from_str::<Value>(request_body) {
        if let Some(params) = ParameterName::Params.get_from(&body_json) {
            if let Some(entity) = ParameterName::Entity.get_from(params) {
                context_info.push(format!("Entity: {entity}"));
            }
            if let Some(component) = FormatCorrectionField::Component.get_str_from(params) {
                context_info.push(format!("Component: {component}"));
            }
            if let Some(path) = ParameterName::Path.get_str_from(params) {
                context_info.push(format!("Path: {path}"));
            }
        }
    }

    // Determine error type and details
    let error_type = if e.is_timeout() {
        "Timeout"
    } else if e.is_connect() {
        "Connection failed"
    } else if e.is_request() {
        "Request error"
    } else if e.is_body() {
        "Body error"
    } else if e.is_decode() {
        "Decode error"
    } else {
        "Unknown error type"
    };

    context_info.push(format!("Error type: {error_type}"));

    // Add port info
    context_info.push(format!("Port: ({port})"));

    let error_msg = format!("HTTP request failed for {method} operation - {error_type}: {e}");

    Err(error_stack::Report::new(Error::JsonRpc(error_msg))
        .attach_printable(context_info.join(", "))
        .attach_printable(format!("Full error: {e:?}"))
        .attach_printable(format!(
            "Request body (first 500 chars): {}",
            &request_body.chars().take(500).collect::<String>()
        )))
}

async fn send_http_request(
    url: &str,
    request_body: String,
    method: &str,
    port: Port,
) -> Result<reqwest::Response> {
    let client = reqwest::Client::new();

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(request_body.clone())
        .timeout(Duration::from_secs(30))
        .send()
        .await;

    match response {
        Ok(resp) => Ok(resp),
        Err(e) => handle_http_error(e, url, &request_body, method, port),
    }
}

/// Check if the HTTP response status is successful
fn check_http_status(response: &reqwest::Response, method: &str, port: Port) -> Result<()> {
    if !response.status().is_success() {
        warn!(
            "BRP execute_brp_method: HTTP status error - status={}",
            response.status()
        );
        return Err(
            error_stack::Report::new(Error::JsonRpc("HTTP error".to_string()))
                .attach_printable(format!(
                    "BRP server returned HTTP error {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ))
                .attach_printable(format!("Method: {method}, Port: {port}")),
        );
    }

    Ok(())
}

/// Parse the JSON response from the BRP server
async fn parse_json_response(
    response: reqwest::Response,
    method: &str,
    port: Port,
) -> Result<BrpClientResponse> {
    match response.json().await {
        Ok(json_resp) => Ok(json_resp),
        Err(e) => {
            warn!("BRP execute_brp_method: JSON parsing failed - error={}", e);
            Err(
                error_stack::Report::new(Error::JsonRpc("JSON parsing failed".to_string()))
                    .attach_printable("Failed to parse BRP response JSON")
                    .attach_printable(format!("Method: {method}, Port: {port}"))
                    .attach_printable(format!("Error: {e}")),
            )
        }
    }
}

/// Convert `BrpClientResponse` to `BrpClientResult`
fn convert_to_brp_result(brp_response: BrpClientResponse, method: &str) -> BrpClientResult {
    if let Some(error) = brp_response.error {
        warn!(
            "BRP execute_brp_method: BRP returned error - code={}, message={}",
            error.code, error.message
        );

        // Check if this is a bevy_brp_extras method that's not found
        let enhanced_message = if error.code == JSON_RPC_ERROR_METHOD_NOT_FOUND
            && method.starts_with(BRP_EXTRAS_PREFIX)
        {
            format!(
                "{}. This method requires the bevy_brp_extras crate to be added to your Bevy app with the BrpExtrasPlugin",
                error.message
            )
        } else {
            error.message
        };

        let result = BrpClientResult::Error(BrpClientError {
            code:    error.code,
            message: enhanced_message,
            data:    error.data,
        });

        debug!("BRP execute_brp_method: Returning BrpResult::Error");

        result
    } else {
        BrpClientResult::Success(brp_response.result)
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

/// Structured error for format discovery failures
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct FormatDiscoveryError {
    #[to_error_info]
    format_corrected: String,

    #[to_error_info]
    hint: String,

    #[to_error_info(skip_if_none)]
    format_corrections: Option<Vec<Value>>,

    #[to_error_info(skip_if_none)]
    original_error_code: Option<i32>,

    #[to_message]
    message: Option<String>,
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

    let format_discovery_error = FormatDiscoveryError {
        format_corrected: "not_attempted".to_string(),
        hint: if hint.is_empty() {
            "Format discovery found issues but could not provide specific guidance.".to_string()
        } else {
            hint
        },
        format_corrections,
        original_error_code: Some(original_error.code),
        message: Some(format!("{}: {}", reason, original_error.message)),
    };

    Error::Structured {
        result: Box::new(format_discovery_error),
    }
}

/// Convert `CorrectionInfo` to `FormatCorrection` (will be used in Phase 2)
fn convert_corrections(corrections: Vec<CorrectionInfo>) -> Vec<FormatCorrection> {
    corrections
        .into_iter()
        .map(|info| {
            FormatCorrection {
                component:            info.type_name,
                original_format:      info.original_value, // Fixed: was original_format
                corrected_format:     info.corrected_value, // Fixed: was corrected_format
                hint:                 info.hint,
                supported_operations: None, // Not available in CorrectionInfo
                mutation_paths:       None, // Not available in CorrectionInfo
                type_category:        None, // Not available in CorrectionInfo
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
                BrpClientResult::Success(value) => {
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
                BrpClientResult::Error(err) => {
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
                BrpClientResult::Error(ref err) => &err.message,
                BrpClientResult::Success(_) => "Unknown error",
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
