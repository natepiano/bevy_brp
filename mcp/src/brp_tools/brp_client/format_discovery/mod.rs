//! Auto-format discovery for BRP type serialization
//!
//! This module provides error-driven type format auto-discovery that intercepts
//! BRP responses and automatically detects and corrects type serialization format
//! errors with zero boilerplate in individual tools. Works with both components and resources.
//!
//! ## Format Discovery Strategy
//!
//! The format discovery system uses a tiered approach to intelligently correct type format errors:
//!
//! ### Tier 1: Serialization Diagnostics
//! - Checks if types support Serialize/Deserialize traits required for BRP operations
//! - Provides early feedback on incompatible types (e.g., `ClearColor` without traits)
//! - Uses `bevy_brp_extras/discover_format` when available
//!
//! ### Tier 2: Direct Discovery
//! - Queries `bevy_brp_extras` plugin for authoritative type format information
//! - Returns factual `TypeDiscoveryResponse` with real examples and mutation paths
//! - Validates format quality to avoid placeholder values like `["example_Color"]`
//! - Falls back to legacy format for backward compatibility
//!
//! ### Tier 3: Smart Pattern Matching
//! - Uses transformer-based system to apply deterministic pattern fixes
//! - Handles common errors like Vec3 object→array conversion, enum variant access
//! - Leverages error message analysis to identify specific transformation needs
//! - Provides detailed hints about the corrections applied
//!
//! ### Tier 4: Generic Fallback
//! - Attempts basic format transformations when pattern matching fails
//! - Last resort before giving up on format correction
//! - Provides minimal hints when successful
//!
//! ## Key Components
//!
//! - **Engine**: Main orchestration with `execute_brp_method_with_format_discovery()`
//! - **Detection**: Error pattern analysis and quality assessment
//! - **Transformers**: Trait-based format transformation system
//! - **Path Suggestions**: Mutation path guidance for complex types
//! - **Phases**: Modular tier execution with comprehensive debug logging
//!
//! ## Design Principles
//!
//! 1. **Zero Boilerplate**: Tools don't need format-specific code
//! 2. **Factual Information**: Never return placeholders or made-up examples
//! 3. **Graceful Degradation**: Each tier has fallback strategies
//! 4. **Comprehensive Debugging**: Full visibility into discovery process
//! 5. **Maintainable Architecture**: Clean separation of concerns

mod adapters;
mod constants;
mod detection;
mod engine;
mod extras_integration;
mod field_mapper;
mod flow_types;
mod path_parser;
mod recovery_engine;
pub mod registry_integration;
mod transformers;
pub mod types;
mod unified_types;

#[cfg(test)]
mod tests;

pub use self::engine::{FormatCorrection, FormatCorrectionStatus};
pub use self::flow_types::FormatRecoveryResult;
// Types will be publicly exposed once the refactoring is complete
pub use self::unified_types::{CorrectionInfo, UnifiedTypeInfo};
use super::super::format_correction_fields::FormatCorrectionField;

// New function for format recovery
pub async fn try_format_recovery_and_retry(
    method: crate::tool::BrpMethod,
    params: Option<serde_json::Value>,
    port: super::super::Port,
    original_error: &super::BrpClientError,
) -> crate::error::Result<flow_types::FormatRecoveryResult> {
    engine::try_format_recovery_and_retry(method, params, port, original_error).await
}
