use crate::brp_tools::support::response_formatter::ResponseFormatterFactory;
use crate::extractors::ExtractedParams;

/// Unified configuration for a BRP handler
/// Works for both static and dynamic methods
pub struct BrpHandlerConfig {
    /// The BRP method to call (static) or None for dynamic methods
    pub method:            Option<&'static str>,
    /// Pre-extracted parameters
    pub extracted_params:  ExtractedParams,
    /// Function to create the appropriate formatter
    pub formatter_factory: ResponseFormatterFactory,
}
