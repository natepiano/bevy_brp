//! provides a unified request handle for all BRP method requests.

// Module organization
mod config;
mod constants;
mod format_discovery;
mod handler;
mod traits;

// Public exports
pub use config::{BrpHandlerConfig, FormatterContext};
pub use format_discovery::FormatCorrectionStatus;
pub use handler::handle_brp_request;
pub use traits::ExtractedParams;
