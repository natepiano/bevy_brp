use thiserror::Error;

use crate::tool::ResultStruct;

// Error message prefixes
const MSG_FAILED_TO_PREFIX: &str = "Failed to";
const MSG_CANNOT_PREFIX: &str = "Cannot";
const MSG_INVALID_PREFIX: &str = "Invalid";
const MSG_MISSING_PREFIX: &str = "Missing";
const MSG_UNEXPECTED_PREFIX: &str = "Unexpected";

/// Result type for the `bevy_brp_mcp` library
pub type Result<T> = error_stack::Result<T, Error>;

// Internal error types for detailed error categorization
#[derive(Error)]
pub enum Error {
    #[error("BRP communication failed: {0}")]
    BrpCommunication(String),

    #[error("File operation failed: {0}")]
    FileOperation(String),

    #[error("File or path not found error: {0}")]
    FileOrPathNotFound(String),

    #[error("{0}")]
    General(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("Log operation failed: {0}")]
    LogOperation(String),

    #[error("MCP client communication failed: {0}")]
    McpClientCommunication(String),

    #[error("Configuration error: {0}")]
    MissingMessageTemplate(String),

    #[error("Unable to extract parameters: {0}")]
    ParameterExtraction(String),

    #[error("Process management error: {0}")]
    ProcessManagement(String),

    #[error("Schema processing error: {0}")]
    SchemaProcessing(String),

    #[error("Structured error")] // Generic message, the real message comes from the ResultStruct
    Structured { result: Box<dyn ResultStruct> },

    #[error("Type not registered: {type_name}")]
    TypeNotRegistered { type_name: String },

    #[error("Tool call error: {message}")]
    ToolCall {
        message: String,
        details: Option<serde_json::Value>,
    },

    #[error("Watch operation failed: {0}")]
    WatchOperation(String),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BrpCommunication(s) => f.debug_tuple("BrpCommunication").field(s).finish(),
            Self::FileOperation(s) => f.debug_tuple("FileOperation").field(s).finish(),
            Self::FileOrPathNotFound(s) => f.debug_tuple("FileOrPathNotFound").field(s).finish(),
            Self::General(s) => f.debug_tuple("General").field(s).finish(),
            Self::InvalidArgument(s) => f.debug_tuple("InvalidArgument").field(s).finish(),
            Self::InvalidState(s) => f.debug_tuple("InvalidState").field(s).finish(),
            Self::JsonRpc(s) => f.debug_tuple("JsonRpc").field(s).finish(),
            Self::LogOperation(s) => f.debug_tuple("LogOperation").field(s).finish(),
            Self::McpClientCommunication(s) => {
                f.debug_tuple("McpClientCommunication").field(s).finish()
            }
            Self::MissingMessageTemplate(s) => f.debug_tuple("Configuration").field(s).finish(),
            Self::ParameterExtraction(s) => f.debug_tuple("ParameterExtraction").field(s).finish(),
            Self::ProcessManagement(s) => f.debug_tuple("ProcessManagement").field(s).finish(),
            Self::SchemaProcessing(s) => f.debug_tuple("SchemaProcessing").field(s).finish(),
            Self::Structured { .. } => f
                .debug_struct("Structured")
                .field("result", &"<dyn ResultStruct>")
                .finish(),
            Self::TypeNotRegistered { type_name } => f
                .debug_struct("TypeNotRegistered")
                .field("type_name", type_name)
                .finish(),
            Self::ToolCall { message, details } => f
                .debug_struct("ToolCall")
                .field("message", message)
                .field("details", details)
                .finish(),
            Self::WatchOperation(s) => f.debug_tuple("WatchOperation").field(s).finish(),
        }
    }
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

// Note: We don't implement From<Error> for McpError because our errors
// are handled internally and converted to structured responses.
// Errors should never escape our tool handlers.
