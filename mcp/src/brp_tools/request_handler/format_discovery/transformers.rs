//! Trait-based format transformation system
//!
//! This module consolidates the transformation logic into a clean trait-based system
//! that replaces the previous 1000+ line transformations.rs file.

use serde_json::Value;

use super::detection::ErrorPattern;
use crate::brp_tools::support::brp_client::BrpError;

// Import transformer implementations
pub mod common;
pub mod constants;
mod enum_variant;
mod math_type;
mod string_type;
mod tuple_struct;

pub use self::enum_variant::EnumVariantTransformer;
pub use self::math_type::MathTypeTransformer;
pub use self::string_type::StringTypeTransformer;
pub use self::tuple_struct::TupleStructTransformer;

/// Trait for format transformers that can handle specific error patterns
pub trait FormatTransformer {
    /// Check if this transformer can handle the given error pattern
    fn can_handle(&self, error_pattern: &ErrorPattern) -> bool;

    /// Transform the value to fix the format error
    /// Returns `Some((transformed_value, description))` if successful, `None` otherwise
    fn transform(&self, value: &Value) -> Option<(Value, String)>;

    /// Transform with additional context from the error
    /// Default implementation ignores the error and calls `transform()`
    fn transform_with_error(&self, value: &Value, _error: &BrpError) -> Option<(Value, String)> {
        self.transform(value)
    }

    /// Get the name of this transformer for debugging
    #[cfg(test)]
    fn name(&self) -> &'static str;
}

/// Registry for managing format transformers
pub struct TransformerRegistry {
    transformers: Vec<Box<dyn FormatTransformer>>,
}

impl TransformerRegistry {
    /// Create a new transformer registry
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
        }
    }

    /// Create a registry with all default transformers
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.add_default_transformers();
        registry
    }

    /// Add a transformer to the registry
    pub fn add_transformer(&mut self, transformer: Box<dyn FormatTransformer>) {
        self.transformers.push(transformer);
    }

    /// Add all default transformers
    fn add_default_transformers(&mut self) {
        self.add_transformer(Box::new(MathTypeTransformer::new()));
        self.add_transformer(Box::new(StringTypeTransformer::new()));
        self.add_transformer(Box::new(TupleStructTransformer::new()));
        self.add_transformer(Box::new(EnumVariantTransformer::new()));
    }

    /// Find a transformer that can handle the given error pattern
    pub fn find_transformer(&self, error_pattern: &ErrorPattern) -> Option<&dyn FormatTransformer> {
        self.transformers
            .iter()
            .find(|t| t.can_handle(error_pattern))
            .map(std::convert::AsRef::as_ref)
    }

    /// Try to transform the value using any applicable transformer
    pub fn transform(
        &self,
        value: &Value,
        error_pattern: &ErrorPattern,
        error: &BrpError,
    ) -> Option<(Value, String)> {
        self.find_transformer(error_pattern)
            .and_then(|transformer| transformer.transform_with_error(value, error))
    }

    /// Get the number of registered transformers
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.transformers.len()
    }

    /// Check if the registry is empty
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.transformers.is_empty()
    }

    /// Get names of all registered transformers
    #[cfg(test)]
    pub fn transformer_names(&self) -> Vec<&'static str> {
        self.transformers.iter().map(|t| t.name()).collect()
    }
}

impl Default for TransformerRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = TransformerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_transformer_names() {
        let registry = TransformerRegistry::new();
        let names = registry.transformer_names();
        assert!(names.is_empty());
    }

    // More tests will be added as transformers are implemented
}
