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
    ExtractedValue, FieldSpec, JsonFieldProvider, ParameterFieldType, ResponseFieldType,
    extract_parameter_field, extract_response_field,
};

// Re-export field definitions
pub use parameters::{Parameter, ParameterName, PortParameter};
pub use response_fields::ResponseFieldName;