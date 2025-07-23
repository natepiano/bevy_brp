//! Traits for field placement system
//!
//! These traits work with the `FieldPlacement` derive macro to provide:
//! - Field placement information
//! - Direct field access without JSON serialization
//! - Automatic `CallInfo` generation

use crate::response::extraction::{ExtractedValue, ResponseFieldType};

/// Information about where a field should be placed in the response
#[derive(Debug, Clone)]
pub struct FieldPlacementInfo {
    /// The name of the field
    pub field_name:   &'static str,
    /// Where to place this field (metadata or result)
    pub placement:    super::FieldPlacement,
    /// Optional source path for response fields (e.g., "result.entities")
    pub source_path:  Option<&'static str>,
    /// The type of the field for extraction
    pub field_type:   ResponseFieldType,
    /// Whether to skip this field if it's None
    pub skip_if_none: bool,
}

/// Trait for types that have field placement information
pub trait HasFieldPlacement {
    /// Get the field placement information for this type
    fn field_placements() -> Vec<FieldPlacementInfo>;
}

/// Trait for direct field access without JSON serialization
pub trait FieldAccessor {
    /// Get a field value by name
    fn get_field(&self, name: &str) -> Option<ExtractedValue>;
}

/// Trait for adding response fields directly to the builder
pub trait ResponseData {
    /// Add all response fields to the builder
    fn add_response_fields(
        &self,
        builder: super::builder::ResponseBuilder,
    ) -> crate::error::Result<super::builder::ResponseBuilder>;
}
