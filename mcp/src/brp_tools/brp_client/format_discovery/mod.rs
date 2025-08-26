//! Auto-format discovery for BRP type serialization
//!
//! This module provides error-driven type format auto-discovery that intercepts
//! BRP responses and automatically detects and corrects type serialization format
//! errors with zero boilerplate in individual tools. Works with both components and resources.
//!
//! ## Format Discovery Strategy
//!
//! The format discovery system uses a 3-state approach:
//!
//! ### State 1: Serialization Check
//! - Checks if types support Serialize/Deserialize traits required for BRP operations
//! - Provides early feedback on incompatible types (e.g., `ClearColor` without traits)
//! - Terminal state if serialization issues are found
//!
//! ### State 2: TypeSchema Discovery (Terminal)
//! - Uses `TypeSchemaEngine` for authoritative type format information from the registry
//! - Every Component/Resource automatically gets mutation support and paths
//! - Applies built-in transformations (e.g., Vec3 object→array conversion)
//! - Always terminal - either returns retry corrections or guidance

mod discovery_context;
mod format_correction_fields;
mod guidance;
mod orchestrator;
mod recovery_result;
mod retry;
mod serialization_check;
mod state;
mod type_schema_discovery;
mod types;

// Export new type state API
pub use orchestrator::discover_format_with_recovery;
pub use types::FormatCorrectionStatus;

// Internal tests
#[cfg(test)]
mod tests;
