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
use super::http_client::BrpHttpClient;
use super::types::{
    BrpClientCallJsonResponse, BrpClientError, BrpToolConfig, Operation, ResponseStatus,
    ResultStructBrpExt,
};
use crate::brp_tools::FormatCorrectionStatus;
use crate::brp_tools::brp_type_guide::TypeGuideEngine;
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

        // Only filter out the port field
        let brp_params = if let Value::Object(ref mut map) = params_json {
            map.retain(|key, _value| key != &String::from(ParameterName::Port));
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
    /// Appending the type guide to an error is only attempted for result types with
    /// `BrpToolConfig::ADD_TYPE_GUIDE_TO_ERROR = true`. Result types with `ADD_TYPE_GUIDE_TO_ERROR
    /// = false` will return errors immediately without added `TypeGuide` .
    pub async fn execute<R>(&self) -> Result<R>
    where
        R: ResultStructBrpExt<
                Args = (
                    Option<Value>,
                    Option<Vec<Value>>,
                    Option<FormatCorrectionStatus>,
                ),
            > + BrpToolConfig
            + Send
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
                // Check if this result type supports adding the `TypeGuide`
                if R::ADD_TYPE_GUIDE_TO_ERROR && err.has_format_error_code() {
                    // embed type_guide information
                    match self.try_add_type_guide_to_error(&err).await {
                        Ok(_) => {
                            unreachable!("ADD_TYPE_GUIDE_TO_ERROR error should always return Err")
                        }
                        Err(error_report) => Err(error_report),
                    }
                } else {
                    // Regular error
                    Err(Error::tool_call_failed(format!(
                        "{} (error {})",
                        err.get_message(),
                        err.get_code()
                    ))
                    .into())
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

    /// Raw BRP execution without any error enhancement (used internally to prevent recursion)
    ///
    /// This method is identical to `execute_direct_internal()` but bypasses all error enhancement
    /// to prevent recursion when `TypeSchemaEngine` needs to fetch registry data.
    pub async fn execute_direct_internal_no_enhancement(&self) -> Result<ResponseStatus> {
        // Create HTTP client with our data
        let http_client = BrpHttpClient::new(self.method, self.port, self.params.clone());

        // Send HTTP request (includes status check)
        let response = http_client.send_request().await?;

        // Parse JSON-RPC response
        let brp_response = self.parse_json_response(response).await?;

        // Convert to BrpClientResult with special handling for bevy_brp_extras
        // NO ERROR ENHANCEMENT - return directly
        Ok(self.to_response_status(brp_response))
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
                        .attach("Failed to parse BRP response JSON")
                        .attach(format!(
                            "Method: {}, Port: {}",
                            self.method.as_str(),
                            self.port
                        ))
                        .attach(format!("Error: {e}")),
                )
            }
        }
    }

    /// Extract type names from BRP error messages using regex patterns
    fn extract_types_from_error_message(error_msg: &str) -> Vec<String> {
        const ERROR_PATTERNS: &[&str] = &[
            r"Unknown component type: `([^`]+)`",
            r"([a-zA-Z0-9_:]+) is invalid:",
        ];

        ERROR_PATTERNS
            .iter()
            .filter_map(|pattern| {
                regex::Regex::new(pattern)
                    .ok()
                    .and_then(|regex| regex.captures(error_msg))
                    .and_then(|caps| caps.get(1))
                    .map(|m| (*m.as_str()).to_string())
            })
            .collect()
    }

    /// Enhanced format error creation with type guide embedding
    async fn try_add_type_guide_to_error(&self, error: &BrpClientError) -> Result<ResponseStatus> {
        // Step 1: Try parameter-based extraction using Operation enum
        let mut extracted_types = Operation::try_from(self.method).map_or_else(
            |_| Vec::new(),
            |operation| {
                let params = self.params.as_ref().unwrap_or(&serde_json::Value::Null);
                operation.extract_type_names(params)
            },
        );

        // Step 2: Fallback to error message parsing if parameter extraction failed
        if extracted_types.is_empty() {
            extracted_types = Self::extract_types_from_error_message(error.get_message());
        }

        // Step 3: Handle results based on whether types were extracted
        if extracted_types.is_empty() {
            Self::create_minimal_type_error(error)
        } else {
            self.add_type_guide_to_error(error, extracted_types).await
        }
    }

    /// Create minimal error when no types can be extracted
    fn create_minimal_type_error(error: &BrpClientError) -> Result<ResponseStatus> {
        Err(Error::tool_call_failed_with_details(
            "Format error occurred but could not extract type information",
            serde_json::json!({
                "original_error": error.get_message(),
                "type_guide": {
                    "help": "Unable to determine specific types that failed. Use the brp_type_guide tool to get spawn/insert/mutation information for the types you're working with.",
                    "suggested_action": "Check your BRP method parameters and ensure they match expected structure"
                }
            }),
        )
        .into())
    }

    /// Create full error with type guide embedded for extracted types
    async fn add_type_guide_to_error(
        &self,
        error: &BrpClientError,
        extracted_types: Vec<String>,
    ) -> Result<ResponseStatus> {
        // Create TypeGuideEngine and generate response for extracted types
        let engine = TypeGuideEngine::new(self.port).await?;
        let type_guide_response = engine.generate_response(&extracted_types);

        Err(Error::tool_call_failed_with_details(
            "Format error - see 'type_guide' field for correct format",
            serde_json::json!({
                "original_error": error.get_message(),
                "type_guide": type_guide_response
            }),
        )
        .into())
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
