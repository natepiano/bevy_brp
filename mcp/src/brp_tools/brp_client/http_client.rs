//! HTTP client for BRP (Bevy Remote Protocol) communication
//!
//! This module provides a dedicated HTTP client for making BRP-specific HTTP requests.
//! It encapsulates all HTTP-related operations including URL building, request sending,
//! status checking, and response parsing.

use std::time::Duration;

use serde_json::Value;
use tracing::{debug, warn};

use super::constants::{BRP_DEFAULT_HOST, BRP_HTTP_PROTOCOL, BRP_JSONRPC_PATH};
use super::json_rpc_builder::BrpJsonRpcBuilder;
use crate::brp_tools::Port;
use crate::error::{Error, Result};
use crate::json_object::JsonObjectAccess;
use crate::tool::{BrpMethod, ParameterName};

/// HTTP client for BRP communication
pub struct BrpHttpClient {
    method: BrpMethod,
    port:   Port,
    params: Option<Value>,
}

impl BrpHttpClient {
    /// Create a new BRP HTTP client
    pub const fn new(method: BrpMethod, port: Port, params: Option<Value>) -> Self {
        Self {
            method,
            port,
            params,
        }
    }

    /// Build the BRP URL for this client's port
    fn build_url(&self) -> String {
        format!(
            "{BRP_HTTP_PROTOCOL}://{BRP_DEFAULT_HOST}:{}{BRP_JSONRPC_PATH}",
            self.port
        )
    }

    /// Build the JSON-RPC request body for this client
    fn build_request_body(&self) -> String {
        let method_str = self.method.as_str();
        let mut builder = BrpJsonRpcBuilder::new(method_str);
        if let Some(ref params) = self.params {
            debug!(
                "BRP execute_brp_method: Added params - {}",
                serde_json::to_string(params)
                    .unwrap_or_else(|_| "Failed to serialize params".to_string())
            );
            builder = builder.params(params.clone());
        }
        let request_body = builder.build().to_string();

        debug!("BRP execute_brp_method: Request body - {}", request_body);

        request_body
    }

    /// Send an HTTP request with timeout
    pub async fn send_request(&self) -> Result<reqwest::Response> {
        let url = self.build_url();
        let body = self.build_request_body();
        let client = reqwest::Client::new();

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body.clone())
            .timeout(Duration::from_secs(30))
            .send()
            .await;

        let response = match response {
            Ok(resp) => resp,
            Err(e) => return self.handle_error(e, &url, &body),
        };

        // Check HTTP status before returning
        self.check_status(&response)?;
        Ok(response)
    }

    /// Send an HTTP request for streaming (no timeout)
    pub async fn send_streaming_request(&self) -> Result<reqwest::Response> {
        let url = self.build_url();
        let body = self.build_request_body();
        // Create client with no timeout for streaming
        let client = reqwest::Client::new();

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body.clone())
            .send()
            .await;

        let response = match response {
            Ok(resp) => resp,
            Err(e) => return self.handle_error(e, &url, &body),
        };

        // Check HTTP status before returning
        self.check_status(&response)?;
        Ok(response)
    }

    /// Check if the HTTP response status is successful
    fn check_status(&self, response: &reqwest::Response) -> Result<()> {
        if !response.status().is_success() {
            warn!(
                "BRP execute_brp_method: HTTP status error - status={}",
                response.status()
            );
            return Err(
                error_stack::Report::new(Error::JsonRpc("HTTP error".to_string()))
                    .attach(format!(
                        "BRP server returned HTTP error {}: {}",
                        response.status(),
                        response
                            .status()
                            .canonical_reason()
                            .unwrap_or("Unknown error")
                    ))
                    .attach(format!(
                        "Method: {}, Port: {}",
                        self.method.as_str(),
                        self.port
                    )),
            );
        }

        Ok(())
    }

    /// Handle HTTP errors with detailed context
    fn handle_error(
        &self,
        e: reqwest::Error,
        url: &str,
        request_body: &str,
    ) -> Result<reqwest::Response> {
        // Always log HTTP errors to help debug intermittent failures
        warn!("BRP execute_brp_method: HTTP request failed - error={}", e);

        let error_details = format!(
            "HTTP Error at {}\nMethod: {}\nPort: {}\nURL: {}\nError: {:?}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            self.method.as_str(),
            self.port,
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
            format!("Method: {}", self.method.as_str()),
            format!("Port: {}", self.port),
            format!("URL: {url}"),
        ];

        // Try to parse request body to extract component/entity info for mutations
        if let Ok(body_json) = serde_json::from_str::<Value>(request_body)
            && let Some(params) = body_json.get_field(ParameterName::Params)
        {
            if let Some(entity) = params.get_field(ParameterName::Entity) {
                context_info.push(format!("Entity: {entity}"));
            }
            if let Some(component) = params.get_field_str(ParameterName::Component) {
                context_info.push(format!("Component: {component}"));
            }
            if let Some(path) = params.get_field_str(ParameterName::Path) {
                context_info.push(format!("Path: {path}"));
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
        context_info.push(format!("Port: ({})", self.port));

        let error_msg = format!(
            "HTTP request failed for {} operation - {error_type}: {e}",
            self.method.as_str()
        );

        Err(error_stack::Report::new(Error::JsonRpc(error_msg))
            .attach(context_info.join(", "))
            .attach(format!("Full error: {e:?}"))
            .attach(format!(
                "Request body (first 500 chars): {}",
                &request_body.chars().take(500).collect::<String>()
            )))
    }
}
