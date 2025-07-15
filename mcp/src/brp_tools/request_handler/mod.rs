//! provides a unified request handle for all BRP method requests.

// Module organization
mod constants;
mod format_discovery;
mod handler;

// Public exports
pub use handler::handle_brp_method_tool_call;
pub use format_discovery::{FormatCorrection, FormatCorrectionStatus, FORMAT_DISCOVERY_METHODS};
