//! provides a unified request handle for all BRP method requests.

// Module organization
mod constants;
mod format_discovery;
mod handler;

// Public exports
pub use format_discovery::FormatCorrectionStatus;
pub use handler::handle_brp_method_tool_call;
