//! BRP (Bevy Remote Protocol) client with unified execution interface
//!
//! This module provides a streamlined interface for communicating with BRP servers.
//! The `BrpClient` offers exactly 3 execution methods:
//! - `execute<R>()`: Primary API with automatic format discovery for result types that support it
//! - `execute_raw()`: Low-level API for debugging and format discovery engine
//! - `execute_streaming()`: Specialized API for watch operations with streaming responses

use serde_json::Value;
use tracing::warn;

use super::super::Port;
use super::constants::{BRP_EXTRAS_PREFIX, JSON_RPC_ERROR_METHOD_NOT_FOUND};
use super::format_discovery::FormatDiscoveryEngine;
use super::http_client::BrpHttpClient;
use super::types::{
    BrpClientCallJsonResponse, BrpClientError, ExecuteMode, ResponseStatus, ResultStructBrpExt,
};
use crate::brp_tools::FormatCorrectionStatus;
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

    /// Internal direct execution - does the actual http call - we wanted the internal version so we
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
        // Create engine with parameter validation
        let engine = FormatDiscoveryEngine::new(
            self.method,
            self.port,
            self.params.clone(),
            original_error.clone(),
        )?;

        // Execute discovery and recovery, then transform to typed result
        engine
            .attempt_discovery_with_recovery()
            .await?
            .into_typed_result::<R>(original_error)
    }

    /// Parse the JSON response from the BRP call to a running bevy app
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

    /// Convert the response JSON to a `ResponseStatus`
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
