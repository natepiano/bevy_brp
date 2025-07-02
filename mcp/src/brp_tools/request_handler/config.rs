use super::traits::ParamExtractor;
use crate::brp_tools::support::response_formatter::ResponseFormatterFactory;

/// Unified configuration for a BRP handler
/// Works for both static and dynamic methods
pub struct BrpHandlerConfig {
    /// The BRP method to call (static) or None for dynamic methods
    pub method:            Option<&'static str>,
    /// Function to extract and validate parameters
    pub param_extractor:   Box<dyn ParamExtractor>,
    /// Function to create the appropriate formatter
    pub formatter_factory: ResponseFormatterFactory,
}

/// Context passed to formatter factory
#[derive(Debug, Clone)]
pub struct FormatterContext {
    pub params: Option<serde_json::Value>,
}
