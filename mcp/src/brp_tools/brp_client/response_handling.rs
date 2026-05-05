//! BRP JSON-RPC response, status, and error types.

use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::constants::BRP_ERROR_ACCESS_ERROR;
use super::constants::BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE;
use super::constants::JSON_RPC_ERROR_INTERNAL_ERROR;
use super::constants::JSON_RPC_ERROR_INVALID_PARAMS;
use crate::error::Result;

/// Configuration trait for BRP tools to control enhanced error handling
pub trait BrpToolConfig {
    /// Whether this tool should use enhanced error handling with `type_guide` embedding
    const ADD_TYPE_GUIDE_TO_ERROR: bool = false;
}

/// Extension trait for `ResultStruct` types that handle BRP responses
pub trait ResultStructBrpExt: Sized {
    type Args;

    /// Construct from BRP client response
    fn from_brp_client_response(args: Self::Args) -> Result<Self>;
}

/// Error information from BRP operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrpClientError {
    pub code:    i32,
    pub message: String,
    pub data:    Option<Value>,
}

impl BrpClientError {
    /// Get the error code
    pub const fn get_code(&self) -> i32 { self.code }

    /// Get the error message
    pub fn get_message(&self) -> &str { &self.message }

    /// Check if this error indicates a format issue that can be recovered
    /// This function was constructed through trial and error via vibe coding with claude
    /// There is a bug in `bevy_remote` right now that we get a spurious "Unknown component type"
    /// when a `Component` doesn't have `Serialize`/`Deserialize` traits - this doesn't affect
    /// `Resource`s so the first section is probably correct.
    /// the second section I think is less correct but it will take some time to validate that
    /// moving to an "error codes only" approach doesn't have other issues
    pub const fn has_format_error_code(&self) -> bool {
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

impl Display for BrpClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.message) }
}

/// Raw BRP JSON-RPC response structure
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct BrpClientCallJsonResponse {
    pub jsonrpc: String,
    pub id:      u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<JsonRpcError>,
}

/// Raw BRP error structure from JSON-RPC response
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JsonRpcError {
    pub code:    i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:    Option<Value>,
}

/// Status of a BRP operation - determines `status` field in the `ToolCallJsonResponse`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Successful operation with optional data
    Success(Option<Value>),
    /// Error with code, message and optional data
    Error(BrpClientError),
}

/// Status of format correction attempts
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatCorrectionStatus {
    /// Format discovery was not enabled for this request
    NotApplicable,
    /// No format correction was attempted
    NotAttempted,
    /// Format correction was applied and the operation succeeded
    Succeeded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brp_tools::JSON_RPC_ERROR_METHOD_NOT_FOUND;

    #[test]
    fn test_brp_client_error_display() {
        let error = BrpClientError {
            code:    JSON_RPC_ERROR_INVALID_PARAMS,
            message: "Invalid params".to_string(),
            data:    None,
        };
        assert_eq!(error.to_string(), "Invalid params");
    }

    #[test]
    fn test_brp_client_error_is_format_error() {
        let format_error = BrpClientError {
            code:    JSON_RPC_ERROR_INVALID_PARAMS,
            message: "Invalid params".to_string(),
            data:    None,
        };
        assert!(format_error.has_format_error_code());

        let unknown_component_error = BrpClientError {
            code:    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
            message: "Unknown component type".to_string(),
            data:    None,
        };
        assert!(unknown_component_error.has_format_error_code());

        let non_format_error = BrpClientError {
            code:    JSON_RPC_ERROR_METHOD_NOT_FOUND,
            message: "Method not found".to_string(),
            data:    None,
        };
        assert!(!non_format_error.has_format_error_code());
    }
}
