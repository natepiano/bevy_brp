//! Auto-format discovery for BRP type serialization
//!
//! This module provides error-driven type format auto-discovery that intercepts
//! BRP responses and automatically detects and corrects type serialization format
//! errors with zero boilerplate in individual tools. Works with both components and resources.

mod constants;
mod detection;
mod engine;
mod field_mapper;
mod path_parser;
pub mod phases;
mod transformers;
pub mod types;
mod utilities;

#[cfg(test)]
mod tests;

pub use self::engine::{
    EnhancedBrpResult, FormatCorrection, execute_brp_method_with_format_discovery,
};
