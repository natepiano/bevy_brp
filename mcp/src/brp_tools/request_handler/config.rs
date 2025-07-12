use crate::extractors::ExtractedParams;
use crate::response::ResponseFormatterFactory;

/// Unified configuration for a BRP handler
/// Method is now provided via the typed `HandlerContext`
pub struct BrpHandlerConfig {
    /// Pre-extracted parameters
    pub extracted_params:  ExtractedParams,
    /// Function to create the appropriate formatter
    pub formatter_factory: ResponseFormatterFactory,
}
