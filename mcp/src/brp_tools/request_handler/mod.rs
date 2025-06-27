// Module organization
mod config;
mod constants;
mod extractors;
mod format_discovery;
mod handler;
mod traits;

// Public exports
pub use config::{BrpHandlerConfig, FormatterContext};
pub use extractors::{
    BrpExecuteExtractor, EntityParamExtractor, PassthroughExtractor, RegistrySchemaParamExtractor,
    ResourceParamExtractor, SimplePortExtractor,
};
pub use handler::handle_brp_request;
pub use traits::ParamExtractor;
