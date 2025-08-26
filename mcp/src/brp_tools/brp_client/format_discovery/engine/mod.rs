//! Discovery engine module
//!
//! This module contains the discovery engine implementation for format recovery.

mod discovery_context;
mod guidance;
mod orchestrator;
mod pattern_correction;
mod recovery_result;
mod retry;
mod serialization_check;
mod state;
mod type_schema_discovery;
pub mod types;
mod unified_types;

// Export new type state API
pub use orchestrator::discover_format_with_recovery;
pub use types::{FormatCorrectionStatus, TransformationResult};
pub use unified_types::UnifiedTypeInfo;

// Internal tests
#[cfg(test)]
mod tests;
