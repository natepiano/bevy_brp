//! Traits for field placement system
//!
//! These traits work with the `FieldPlacement` derive macro to provide:
//! - Field placement information
//! - Direct field access without JSON serialization
//! - Automatic `CallInfo` generation

/// Specifies where a response field should be placed in the output JSON
#[derive(Clone, Debug)]
pub enum FieldPlacement {
    /// Place field in the metadata object
    Metadata,
    /// Place field in the result object
    Result,
    /// Place field in the `error_info` object
    ErrorInfo,
}

/// Information about where a field should be placed in the response
///
/// Note: appears unused but is actually used by the `FieldPlacement` derive macro
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FieldPlacementInfo {
    /// The name of the field
    pub field_name:   &'static str,
    /// Where to place this field (metadata or result)
    pub placement:    FieldPlacement,
    /// Optional source path for response fields (e.g., "result.entities")
    pub source_path:  Option<&'static str>,
    /// Whether to skip this field if it's None
    pub skip_if_none: bool,
}

/// Trait for types that have field placement information
///
/// Note: appears unused but is actually used by the `FieldPlacement` derive macro
#[allow(dead_code)]
pub trait HasFieldPlacement {
    /// Get the field placement information for this type
    fn field_placements() -> Vec<FieldPlacementInfo>;
}
