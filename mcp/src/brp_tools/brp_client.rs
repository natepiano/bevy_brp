//! Low-level BRP (Bevy Remote Protocol) client for JSON-RPC communication
//!
//! This module provides a clean interface for communicating with BRP servers
//! without the MCP-specific formatting concerns. It handles raw BRP protocol
//! communication and returns structured results that can be formatted by
//! higher-level tools.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};

use super::constants::{
    BRP_DEFAULT_HOST, BRP_ERROR_ACCESS_ERROR, BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
    BRP_EXTRAS_PREFIX, BRP_HTTP_PROTOCOL, BRP_JSONRPC_PATH, JSON_RPC_ERROR_INTERNAL_ERROR,
    JSON_RPC_ERROR_INVALID_PARAMS, JSON_RPC_ERROR_METHOD_NOT_FOUND,
};
use super::json_rpc_builder::BrpJsonRpcBuilder;
use crate::brp_tools::FormatCorrectionField;
use crate::error::{Error, Result};
use crate::tool::ParameterName;

/// Result of a BRP operation
#[derive(Debug, Clone)]
pub enum BrpResult {
    /// Successful operation with optional data
    Success(Option<Value>),
    /// Error with code, message and optional data
    Error(BrpError),
}

/// Error information from BRP operations
#[derive(Debug, Clone)]
pub struct BrpError {
    pub code:    i32,
    pub message: String,
    pub data:    Option<Value>,
}

impl BrpError {
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
struct BrpResponse {
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

/// Build a BRP URL for the given port
///
/// Constructs the full URL using standard BRP constants for consistent formatting
pub fn build_brp_url(port: u16) -> String {
    format!("{BRP_HTTP_PROTOCOL}://{BRP_DEFAULT_HOST}:{port}{BRP_JSONRPC_PATH}")
}

/// Execute a BRP method and return structured result
pub async fn execute_brp_method(
    method: &str,
    params: Option<Value>,
    port: u16,
) -> Result<BrpResult> {
    let url = build_brp_url(port);

    // Build JSON-RPC request body
    let request_body = build_request_body(method, params);

    // Send HTTP request
    let response = send_http_request(&url, request_body, method, port).await?;

    // Check HTTP status
    check_http_status(&response, method, port)?;

    // Parse JSON-RPC response
    let brp_response = parse_json_response(response, method, port).await?;

    // Convert to structured result
    Ok(convert_to_brp_result(brp_response, method))
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
    port: u16,
) -> Result<reqwest::Response> {
    // Always log HTTP errors to help debug intermittent failures
    warn!("BRP execute_brp_method: HTTP request failed - error={}", e);

    let error_details = format!(
        "HTTP Error at {}\nMethod: {}\nPort: {}{}\nURL: {}\nError: {:?}\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        method,
        port,
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
        if let Some(params) = body_json
            .get(ParameterName::Params.as_ref())
            .and_then(|p| p.as_object())
        {
            if let Some(entity) = params.get("entity") {
                context_info.push(format!("Entity: {entity}"));
            }
            if let Some(component) = params
                .get(FormatCorrectionField::Component.as_ref())
                .and_then(|c| c.as_str())
            {
                context_info.push(format!("Component: {component}"));
            }
            if let Some(path) = params.get("path").and_then(|p| p.as_str()) {
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
    port: u16,
) -> Result<reqwest::Response> {
    let client = super::http_client::get_client();

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
fn check_http_status(response: &reqwest::Response, method: &str, port: u16) -> Result<()> {
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
    port: u16,
) -> Result<BrpResponse> {
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

/// Convert `BrpResponse` to `BrpResult`
fn convert_to_brp_result(brp_response: BrpResponse, method: &str) -> BrpResult {
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

        let result = BrpResult::Error(BrpError {
            code:    error.code,
            message: enhanced_message,
            data:    error.data,
        });

        debug!("BRP execute_brp_method: Returning BrpResult::Error");

        result
    } else {
        BrpResult::Success(brp_response.result)
    }
}
