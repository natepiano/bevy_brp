mod client;
mod constants;
mod http_client;
mod json_rpc_builder;
mod operation;
mod response_handling;

// Re-export public items
pub use client::BrpClient;
// Re-export error constant needed by external modules
pub use constants::JSON_RPC_ERROR_METHOD_NOT_FOUND;
// Re-export types needed by result_struct macro and client operations
pub use response_handling::BrpToolConfig;
pub use response_handling::FormatCorrectionStatus;
pub use response_handling::ResponseStatus;
pub use response_handling::ResultStructBrpExt;
