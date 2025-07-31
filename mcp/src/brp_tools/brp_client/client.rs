//! Low-level BRP (Bevy Remote Protocol) client for JSON-RPC communication
//!
//! This module provides a clean interface for communicating with BRP servers
//! without the MCP-specific formatting concerns. It handles raw BRP protocol
//! communication and returns a `BrpClientResult` that can be formatted by
//! higher-level tools.

use std::time::Duration;

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
    EnhancedBrpResult, FormatCorrectionStatus, execute_brp_method_with_format_discovery,
};
use super::constants::{BRP_DEFAULT_HOST, BRP_EXTRAS_PREFIX, BRP_HTTP_PROTOCOL, BRP_JSONRPC_PATH};
use super::json_rpc_builder::BrpJsonRpcBuilder;
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, JsonFieldAccess, ParameterName};

/// Client for executing a BRP operation
pub struct BrpClient {
    method:           BrpMethod,
    port:             Port,
    params:           Option<Value>,
    format_discovery: bool,
}

impl BrpClient {
    /// Create a new BRP client for the given method, port, and parameters
    pub const fn new(method: BrpMethod, port: Port, params: Option<Value>) -> Self {
        Self {
            method,
            port,
            params,
            format_discovery: false,
        }
    }

    /// Enable format discovery for this request (builder pattern)
    pub const fn with_format_discovery(mut self) -> Self {
        self.format_discovery = true;
        self
    }

    /// Execute the BRP request and return enhanced result with format discovery information
    pub async fn execute(self) -> Result<EnhancedBrpResult> {
        if self.format_discovery {
            // Clone params for potential retry attempts
            let params_for_retry = self.params.clone();
            self.execute_with_format_discovery_internal(params_for_retry)
                .await
        } else {
            // Execute directly and wrap in EnhancedBrpResult
            let result = self.execute_direct_internal().await?;
            Ok(EnhancedBrpResult {
                result,
                format_corrections: Vec::new(),
                format_corrected: FormatCorrectionStatus::NotApplicable,
            })
        }
    }

    /// Internal direct execution (current `execute()` logic moved here)
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

    /// Internal format discovery execution
    async fn execute_with_format_discovery_internal(
        self,
        _params_for_retry: Option<Value>,
    ) -> Result<EnhancedBrpResult> {
        // Delegate to existing engine (format discovery stays as sibling module)
        execute_brp_method_with_format_discovery(self.method, self.params, self.port).await
    }

    /// Execute the BRP request without format discovery (legacy compatibility)
    /// Returns just the `BrpClientResult` without enhanced information
    pub async fn execute_direct(self) -> Result<BrpClientResult> {
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
