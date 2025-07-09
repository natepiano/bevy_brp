//! Extraction type definitions for the MCP tools extraction system.
//!
//! This module defines the types used to specify how data should be extracted
//! from different sources during tool execution.

use serde_json::Value;

/// Types of extractors for response fields.
///
/// This enum defines the different strategies for extracting data during tool execution.
/// Each variant corresponds to a specific extraction pattern and data source:
///
/// # Data Source Categories:
///
/// ## Input Extraction (uses `McpCallExtractor`)
/// - `EntityFromParams` - Extract entity ID from user input parameters
/// - `ParamFromContext` - Extract specific parameter from user input context
/// - `QueryParamsFromContext` - Extract query parameters from user input context
/// - `ResourceFromParams` - Extract resource name from user input parameters
///
/// ## Output Extraction (uses `BevyResponseExtractor`)
/// - `ComponentCountFromData` - Count components in response data
/// - `CountFromData` - Extract count field from response data
/// - `DataField` - Extract specific field from response data structure
/// - `EntityCountFromData` - Count entities in response data
/// - `EntityFromResponse` - Extract entity ID from response data (spawn operations)
/// - `PassThroughData` - Pass through response data without modification
/// - `QueryComponentCount` - Count components in nested query results
///
/// ## Direct Extraction (no extractor needed)
/// - `PassThroughResult` - Pass through entire result without processing
#[derive(Clone)]
pub enum ExtractorType {
    // === INPUT EXTRACTION (uses McpCallExtractor) ===
    /// Extract entity from params
    EntityFromParams,
    /// Extract specific parameter from request context
    ParamFromContext(&'static str),
    /// Extract query parameters from request context
    QueryParamsFromContext,
    /// Extract resource from params
    ResourceFromParams,

    // === OUTPUT EXTRACTION (uses BevyResponseExtractor) ===
    /// Extract component count from data
    ComponentCountFromData,
    /// Extract count from data for local operations
    CountFromData,
    /// Extract field from data structure (for local handler results)
    DataField(&'static str),
    /// Extract entity count from data
    EntityCountFromData,
    /// Extract entity from response data (for spawn operation)
    EntityFromResponse,
    /// Pass through data from BRP response
    PassThroughData,
    /// Extract total component count from nested query results
    QueryComponentCount,

    // === DIRECT EXTRACTION (no extractor needed) ===
    /// Pass through entire result
    PassThroughResult,
}

/// Defines a field to include in the response
#[derive(Clone)]
pub struct ResponseField {
    /// Name of the field in the response
    pub name:      &'static str,
    /// Type of extractor to use
    pub extractor: ExtractorType,
}

/// Result of parameter extraction
pub struct ExtractedParams {
    /// The method name for dynamic handlers, None for static
    pub method: Option<String>,
    /// The extracted parameters
    pub params: Option<Value>,
    /// The BRP port to use
    pub port:   u16,
}

/// Context passed to formatter factory
#[derive(Debug, Clone)]
pub struct FormatterContext {
    pub params:           Option<Value>,
    pub format_corrected: Option<crate::brp_tools::request_handler::FormatCorrectionStatus>,
}
