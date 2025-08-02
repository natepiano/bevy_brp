mod client;
mod constants;
mod format_correction_fields;
mod format_discovery;
mod http_client;
mod json_rpc_builder;
mod types;

// Re-export public items
pub use client::BrpClient;
// Re-export error constant needed by external modules
pub use constants::JSON_RPC_ERROR_METHOD_NOT_FOUND;
// Re-export FormatCorrectionStatus for use by result_struct macro
pub use format_discovery::FormatCorrectionStatus;
// Re-export types needed by result_struct macro and client operations
pub use types::{BrpClientError, ExecuteMode, ResponseStatus, ResultStructBrpExt};
