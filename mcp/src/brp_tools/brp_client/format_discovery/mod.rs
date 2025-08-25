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
//! - Uses registry schema via a call to `brp_type_schema`
//!
//! ### Tier 2: Direct Discovery
//! - Uses registry schema for authoritative type format information
//! - Returns factual `TypeDiscoveryResponse` with real examples and mutation paths
//! - Validates format quality to avoid placeholder values like `["example_Color"]`
//! - Falls back to legacy format for backward compatibility
//!
//! ### Tier 3: Pattern Matching
//! - Uses transformer-based system to apply deterministic pattern fixes
//! - Handles common errors like Vec3 object→array conversion, enum variant access
//! - Leverages error message analysis to identify specific transformation needs
//! - Provides detailed hints about the corrections applied
//!
//! ### Tier 4: Generic Fallback
//! - Attempts basic format transformations when pattern matching fails
//! - Last resort before giving up on format correction
//! - Provides minimal hints when successfulå

mod constants;
mod detection;
pub mod engine;
mod field_mapper;
mod format_correction_fields;
mod transformers;
mod types;

#[cfg(test)]
mod tests;

pub use engine::{FormatCorrectionStatus, discover_format_with_recovery};
