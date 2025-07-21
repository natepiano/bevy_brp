//! Unified JSON field extraction module.
//!
//! This module consolidates all JSON field extraction logic, including parameter
//! and response field definitions, into a single cohesive module.

// Internal modules
mod extraction;
mod parameters;
mod response_fields;

// Re-export core types and traits for public use
pub use extraction::{
    FieldSpec, JsonFieldProvider, ParameterFieldType, ResponseFieldType, extract_response_field,
};
// Re-export field definitions
pub use parameters::ParameterName;
pub use response_fields::ResponseFieldName;
