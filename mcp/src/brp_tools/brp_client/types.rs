//! Common types for BRP tools

use bevy_brp_mcp_macros::ResultStruct;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::constants::{
    BRP_ERROR_ACCESS_ERROR, BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE, JSON_RPC_ERROR_INTERNAL_ERROR,
    JSON_RPC_ERROR_INVALID_PARAMS,
};
use crate::error::Result;

/// Extension trait for `ResultStruct` types that handle BRP responses
pub trait ResultStructBrpExt: Sized {
    type Args;

    /// Determine the execution mode for this tool
    fn brp_tool_execute_mode() -> ExecuteMode;

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
pub(super) struct BrpClientCallJsonResponse {
    pub jsonrpc: String,
    pub id:      u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<JsonRpcError>,
}

/// Execution mode for BRP tool responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecuteMode {
    /// Use format discovery for enhanced error handling
    WithFormatDiscovery,
    /// Standard processing without format discovery
    Standard,
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
    error_code: Option<i32>,

    #[to_error_info]
    reason: String,

    #[to_error_info]
    error_message: String,

    #[to_message(message_template = "{reason}: {error_message}")]
    message_template: String,
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
