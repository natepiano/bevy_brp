//! provides a unified request handle for all BRP method requests.

// Module organization
mod config;
mod constants;
mod format_discovery;
mod handler;

// Public exports
pub use config::BrpHandlerConfig;
pub use format_discovery::FormatCorrectionStatus;
pub use handler::handle_brp_method_tool_call;
