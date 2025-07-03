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

use super::BrpJsonRpcBuilder;
use crate::brp_tools::constants::{
    BRP_DEFAULT_HOST, BRP_HTTP_PROTOCOL, BRP_JSONRPC_PATH, DEFAULT_BRP_PORT,
};
use crate::error::{Error, Result};
use crate::tools::BRP_EXTRAS_PREFIX;

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
    port: Option<u16>,
) -> Result<BrpResult> {
    let port = port.unwrap_or(DEFAULT_BRP_PORT);
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

    // Write detailed error to temp file for debugging
    let port_source = if port == DEFAULT_BRP_PORT {
        " (DEFAULT - port parameter missing!)"
    } else {
        " (explicit)"
    };
    let error_details = format!(
        "HTTP Error at {}\nMethod: {}\nPort: {}{}\nURL: {}\nError: {:?}\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        method,
        port,
        port_source,
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
        if let Some(params) = body_json.get("params").and_then(|p| p.as_object()) {
            if let Some(entity) = params.get("entity") {
                context_info.push(format!("Entity: {entity}"));
            }
            if let Some(component) = params.get("component").and_then(|c| c.as_str()) {
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

    // Add port source information
    if port == DEFAULT_BRP_PORT {
        context_info.push("Port source: DEFAULT (port parameter missing!)".to_string());
    } else {
        context_info.push(format!("Port source: explicit ({port})"));
    }

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
        let enhanced_message = if error.code == -32601 && method.starts_with(BRP_EXTRAS_PREFIX) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brp_error_creation() {
        let error = BrpError {
            code:    -32600,
            message: "Invalid Request".to_string(),
            data:    None,
        };
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_brp_result_success() {
        let result = BrpResult::Success(Some(serde_json::json!({"test": "value"})));
        matches!(result, BrpResult::Success(_));
    }

    #[test]
    fn test_brp_result_error() {
        let error = BrpError {
            code:    -1,
            message: "Test error".to_string(),
            data:    None,
        };
        let result = BrpResult::Error(error);
        matches!(result, BrpResult::Error(_));
    }
}
