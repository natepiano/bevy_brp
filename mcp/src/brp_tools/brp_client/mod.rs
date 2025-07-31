mod client;
mod constants;
mod format_correction_fields;
mod format_discovery;
mod json_rpc_builder;

// Re-export public items
pub use client::{BrpClient, BrpClientError, BrpClientResult};
// Re-export FormatCorrectionStatus for use by result_struct macro
pub use format_discovery::FormatCorrectionStatus;
