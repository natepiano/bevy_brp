use rmcp::Error as McpError;
use thiserror::Error;

// Error message prefixes
const MSG_FAILED_TO_PREFIX: &str = "Failed to";
const MSG_CANNOT_PREFIX: &str = "Cannot";
const MSG_INVALID_PREFIX: &str = "Invalid";
const MSG_MISSING_PREFIX: &str = "Missing";
const MSG_UNEXPECTED_PREFIX: &str = "Unexpected";

// Internal error types for detailed error categorization
#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("Mutex poisoned: {0}")]
    MutexPoisoned(String),

    #[error("BRP communication failed: {0}")]
    BrpCommunication(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("Format discovery error: {0}")]
    FormatDiscovery(String),

    #[error("File operation failed: {0}")]
    FileOperation(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("File or path not found error: {0}")]
    FileOrPathNotFound(String),

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

    #[error("{0}")]
    General(String),
}

/// Result type for the `bevy_brp_mcp` library
pub type Result<T> = error_stack::Result<T, Error>;

impl Error {
    // Builder methods for common patterns

    /// Create a "Failed to X" error with appropriate variant
    pub fn failed_to(action: &str, details: impl std::fmt::Display) -> Self {
        let message = format!("{MSG_FAILED_TO_PREFIX} {action}: {details}");
        Self::categorize_error(&message)
    }

    /// Create a "Cannot X" error
    pub fn cannot(action: &str, reason: impl std::fmt::Display) -> Self {
        let message = format!("{MSG_CANNOT_PREFIX} {action}: {reason}");
        Self::categorize_error(&message)
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
        let message = format!("{MSG_UNEXPECTED_PREFIX} {what}: {details}");
        Self::categorize_error(&message)
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

    /// Categorize error based on content
    fn categorize_error(message: &str) -> Self {
        // Simple heuristic categorization
        if message.contains("watch") || message.contains("subscription") {
            Self::WatchOperation(message.to_string())
        } else if message.contains("process")
            || message.contains("kill")
            || message.contains("launch")
        {
            Self::ProcessManagement(message.to_string())
        } else if message.contains("file")
            || message.contains("log")
            || message.contains("read")
            || message.contains("write")
        {
            Self::LogOperation(message.to_string())
        } else if message.contains("parameter")
            || message.contains("extract")
            || message.contains("invalid")
        {
            Self::InvalidArgument(message.to_string())
        } else {
            Self::General(message.to_string()) // Default fallback
        }
    }
}

// Conversion to McpError for API boundaries
impl From<Error> for McpError {
    fn from(err: Error) -> Self {
        match err {
            Error::BrpCommunication(msg)
            | Error::JsonRpc(msg)
            | Error::FormatDiscovery(msg)
            | Error::FileOrPathNotFound(msg)
            | Error::InvalidArgument(msg) => Self::invalid_params(msg, None),
            Error::PathDisambiguation { message, .. } => {
                // For path disambiguation, we want to preserve the detailed message
                // as an invalid_params error since it's a parameter issue that can be resolved
                Self::invalid_params(message, None)
            }
            Error::MutexPoisoned(msg)
            | Error::FileOperation(msg)
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
