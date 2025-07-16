//! provides a unified request handle for all BRP method requests.

// Module organization
mod constants;
mod format_discovery;
mod handler;

// Public exports
pub use format_discovery::{FORMAT_DISCOVERY_METHODS, FormatCorrection, FormatCorrectionStatus};
pub use handler::BrpMethodHandler;
