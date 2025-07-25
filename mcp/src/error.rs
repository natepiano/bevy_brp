use rmcp::ErrorData as McpError;
use thiserror::Error;

// Error message prefixes
const MSG_FAILED_TO_PREFIX: &str = "Failed to";
const MSG_CANNOT_PREFIX: &str = "Cannot";
const MSG_INVALID_PREFIX: &str = "Invalid";
const MSG_MISSING_PREFIX: &str = "Missing";
const MSG_UNEXPECTED_PREFIX: &str = "Unexpected";

/// Result type for the `bevy_brp_mcp` library
pub type Result<T> = error_stack::Result<T, Error>;

// Internal error types for detailed error categorization
#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("BRP communication failed: {0}")]
    BrpCommunication(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("File operation failed: {0}")]
    FileOperation(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("File or path not found error: {0}")]
    FileOrPathNotFound(String),

    #[error("Unable to extract parameters: {0}")]
    ParameterExtraction(String),

    #[error("Watch operation failed: {0}")]
    WatchOperation(String),

    #[error("Process management error: {0}")]
    ProcessManagement(String),

    #[error("Path disambiguation required: {message}")]
    PathDisambiguation {
        message:         String,
        item_type:       String,
        item_name:       String,
        available_paths: Vec<String>,
    },

    #[error("Log operation failed: {0}")]
    LogOperation(String),

    #[error("MCP client communication failed: {0}")]
    McpClientCommunication(String),

    #[error("Tool call error: {message}")]
    ToolCall {
        message: String,
        details: Option<serde_json::Value>,
    },

    #[error("{0}")]
    General(String),
}

impl Error {
    // Builder methods for common patterns

    /// Create a "Failed to X" error
    pub fn failed_to(action: &str, details: impl std::fmt::Display) -> Self {
        Self::General(format!("{MSG_FAILED_TO_PREFIX} {action}: {details}"))
    }

    /// Create a "Cannot X" error
    pub fn cannot(action: &str, reason: impl std::fmt::Display) -> Self {
        Self::General(format!("{MSG_CANNOT_PREFIX} {action}: {reason}"))
    }

    /// Create an "Invalid X" error
    pub fn invalid(what: &str, details: impl std::fmt::Display) -> Self {
        Self::InvalidArgument(format!("{MSG_INVALID_PREFIX} {what}: {details}"))
    }

    /// Create a "Missing X" error
    pub fn missing(what: &str) -> Self {
        Self::InvalidArgument(format!("{MSG_MISSING_PREFIX} {what}"))
    }

    /// Create an "Unexpected X" error
    pub fn unexpected(what: &str, details: impl std::fmt::Display) -> Self {
        Self::General(format!("{MSG_UNEXPECTED_PREFIX} {what}: {details}"))
    }

    /// Create error for IO operations
    pub fn io_failed(
        operation: &str,
        path: &std::path::Path,
        error: impl std::fmt::Display,
    ) -> Self {
        Self::LogOperation(format!(
            "{MSG_FAILED_TO_PREFIX} {operation} {}: {error}",
            path.display()
        ))
    }

    /// Create error for process operations
    pub fn process_failed(operation: &str, process: &str, error: impl std::fmt::Display) -> Self {
        Self::ProcessManagement(format!(
            "{MSG_FAILED_TO_PREFIX} {operation} process '{process}': {error}"
        ))
    }

    /// Create error for watch operations
    pub fn watch_failed(
        operation: &str,
        entity: Option<u32>,
        error: impl std::fmt::Display,
    ) -> Self {
        entity.map_or_else(
            || Self::WatchOperation(format!("{MSG_FAILED_TO_PREFIX} {operation}: {error}")),
            |id| {
                Self::WatchOperation(format!(
                    "{MSG_FAILED_TO_PREFIX} {operation} for entity {id}: {error}"
                ))
            },
        )
    }

    /// Create error for BRP request failures
    pub fn brp_request_failed(operation: &str, error: impl std::fmt::Display) -> Self {
        Self::BrpCommunication(format!(
            "{MSG_FAILED_TO_PREFIX} {operation} BRP request: {error}"
        ))
    }

    /// Create error for validation failures
    pub fn validation_failed(what: &str, reason: impl std::fmt::Display) -> Self {
        Self::InvalidArgument(format!("Validation failed for {what}: {reason}"))
    }

    /// Create error for stream operations
    pub fn stream_failed(operation: &str, limit: impl std::fmt::Display) -> Self {
        Self::WatchOperation(format!(
            "{MSG_FAILED_TO_PREFIX} {operation}: limit {limit} exceeded"
        ))
    }

    /// Create a tool error with just a message
    pub fn tool_call_failed(message: impl Into<String>) -> Self {
        Self::ToolCall {
            message: message.into(),
            details: None,
        }
    }

    /// Create a tool error with message and details
    pub fn tool_call_failed_with_details(
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::ToolCall {
            message: message.into(),
            details: Some(details),
        }
    }
}

// Conversion to McpError for API boundaries
impl From<Error> for McpError {
    fn from(err: Error) -> Self {
        match err {
            Error::BrpCommunication(msg)
            | Error::JsonRpc(msg)
            | Error::FileOrPathNotFound(msg)
            | Error::InvalidArgument(msg)
            | Error::ParameterExtraction(msg) => Self::invalid_params(msg, None),
            Error::PathDisambiguation { message, .. } => {
                // For path disambiguation, we want to preserve the detailed message
                // as an invalid_params error since it's a parameter issue that can be resolved
                Self::invalid_params(message, None)
            }
            Error::ToolCall { message, details } => {
                // Tool errors are typically parameter/request issues
                Self::invalid_params(message, details)
            }
            Error::FileOperation(msg)
            | Error::InvalidState(msg)
            | Error::WatchOperation(msg)
            | Error::ProcessManagement(msg)
            | Error::LogOperation(msg)
            | Error::McpClientCommunication(msg)
            | Error::General(msg) => Self::internal_error(msg, None),
        }
    }
}

// Helper function to convert error-stack Report to McpError
pub fn report_to_mcp_error(report: &error_stack::Report<Error>) -> McpError {
    (*report.current_context()).clone().into()
}
