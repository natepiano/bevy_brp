use crate::extractors::ExtractorType;

/// New response field specification that separates request and response concerns.
///
/// This enum replaces the old `ResponseField` struct, providing clear separation
/// between request parameter referencing and response data extraction specifications.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Temporarily allow during migration
pub enum ResponseFieldV2 {
    /// Reference a field from already-extracted request parameters.
    ///
    /// This variant references data that was already extracted and validated during
    /// the parameter extraction phase, avoiding redundant extraction and ensuring
    /// consistent validation.
    FromRequest {
        /// Name of the field in the response
        name:  &'static str,
        /// Field name in the `ExtractedParams` structure
        field: &'static str,
    },
    /// Extract a field from response data.
    ///
    /// This variant specifies extraction of data from the handler or BRP response payload.
    FromResponse {
        /// Name of the field in the response
        name:      &'static str,
        /// Extractor type for response data
        extractor: ResponseExtractorType,
    },
}

/// Extraction strategies for response data only.
///
/// This enum contains only the extractors that should be used for response data,
/// removing the request parameter extractors that were incorrectly mixed in the old system.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Temporarily allow during migration
pub enum ResponseExtractorType {
    /// Pass through the entire response and store it under "data" field without modification
    PassThroughData,
    /// Pass through response fields as peers of status/message (no data wrapper)
    PassThroughRaw,
    /// Extract a specific field from the response data structure
    Field(&'static str),
    /// Extract count field from response data
    Count,
    /// Count entities in response data
    EntityCount,
    /// Count components in response data
    ComponentCount,
    /// Extract total component count from nested query results
    QueryComponentCount,
    /// Extract entity ID from response data (for spawn operations)
    EntityId,
}

impl ResponseFieldV2 {
    /// Get the field name for this response field specification.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::FromRequest { name, .. } | Self::FromResponse { name, .. } => name,
        }
    }
}

/// Defines how to format the response for a tool
#[derive(Clone)]
pub struct ResponseSpecification {
    /// Type of formatter to use
    pub formatter_type:  FormatterType,
    /// Template for success messages
    pub template:        &'static str,
    /// Fields to include in the response
    pub response_fields: Vec<ResponseFieldCompat>,
}

/// Types of formatters available
#[derive(Clone)]
pub enum FormatterType {
    /// Entity operation formatter
    EntityOperation,
    /// Resource operation formatter
    ResourceOperation,
    /// Simple formatter (no special formatting)
    Simple,
    /// Formatter for local operations
    Local,
    /// Local passthrough formatter for handlers that return pre-structured responses
    LocalPassthrough,
}

/// Defines a field to include in the response
#[derive(Clone)]
pub struct ResponseField {
    /// Name of the field in the response
    pub name:      &'static str,
    /// Type of extractor to use
    pub extractor: ExtractorType,
}

/// Compatibility wrapper for `ResponseField` during migration.
///
/// This enum allows both the old `ResponseField` struct and the new `ResponseFieldV2` enum
/// to coexist during the migration process. Once all tools are migrated, this wrapper
/// will be removed.
#[derive(Clone)]
#[allow(dead_code)] // Temporarily allow during migration
pub enum ResponseFieldCompat {
    /// Legacy `ResponseField` struct (current format)
    V1(ResponseField),
    /// New `ResponseFieldV2` enum (target format)
    V2(ResponseFieldV2),
}
