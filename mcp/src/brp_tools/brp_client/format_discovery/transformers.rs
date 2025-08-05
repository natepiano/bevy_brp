//! Trait-based format transformation system
//!
//! This module consolidates the transformation logic into a clean trait-based system
//! that replaces the previous 1000+ line transformations.rs file.
//!
//! Updated for Phase 3 to work with the unified type system using `UnifiedTypeInfo`
//! instead of the legacy `DiscoveredFacts` structure.

use serde_json::Value;

use super::detection::ErrorPattern;
use super::types::TransformationResult;
use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::BrpClientError;

// Import transformer implementations
pub mod common;
mod enum_variant;
mod math_type;
mod string_type;
mod tuple_struct;

pub use self::enum_variant::EnumVariantTransformer;
pub use self::math_type::MathTypeTransformer;
pub use self::string_type::StringTypeTransformer;
pub use self::tuple_struct::TupleStructTransformer;

/// Trait for format transformers that can handle specific error patterns
///
/// Updated for Phase 3 to work with `UnifiedTypeInfo` which provides comprehensive
/// type information including registry status, serialization support, and format examples.
pub trait FormatTransformer: Send + Sync {
    /// Check if this transformer can handle the given error pattern
    fn can_handle(&self, error_pattern: &ErrorPattern) -> bool;

    /// Transform the value to fix the format error
    /// Returns `Some(TransformationResult)` if successful, `None` otherwise
    fn transform(&self, value: &Value) -> Option<TransformationResult>;

    /// Transform with additional context from the error
    /// Default implementation ignores the error and calls `transform()`
    fn transform_with_error(
        &self,
        value: &Value,
        _error: &BrpClientError,
    ) -> Option<TransformationResult> {
        self.transform(value)
    }

    /// Transform with comprehensive type information from the unified type system
    ///
    /// This method provides access to complete type information including:
    /// - Registry status and reflection information
    /// - Serialization capability (Serialize/Deserialize traits)
    /// - Format examples from direct discovery
    /// - Mutation paths for complex types
    /// - Type category and child type information
    ///
    /// Default implementation falls back to error-only transformation for backward compatibility.
    fn transform_with_type_info(
        &self,
        value: &Value,
        error: &BrpClientError,
        _type_info: &UnifiedTypeInfo,
    ) -> Option<TransformationResult> {
        self.transform_with_error(value, error)
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

    /// Try to transform the value using any applicable transformer with comprehensive type
    /// information
    ///
    /// This method uses the unified type system to provide transformers with complete type
    /// information including format examples, mutation paths, and serialization capabilities.
    pub fn transform_with_type_info(
        &self,
        value: &Value,
        error_pattern: &ErrorPattern,
        error: &BrpClientError,
        type_info: &UnifiedTypeInfo,
    ) -> Option<TransformationResult> {
        self.find_transformer(error_pattern)
            .and_then(|transformer| transformer.transform_with_type_info(value, error, type_info))
    }

    /// Try to transform the value using any applicable transformer (legacy method)
    ///
    /// This method is maintained for backward compatibility during the transition.
    /// New code should use `transform_with_type_info()` instead.
    pub fn transform_legacy(
        &self,
        value: &Value,
        error_pattern: &ErrorPattern,
        error: &BrpClientError,
    ) -> Option<TransformationResult> {
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
}

impl Default for TransformerRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Get a singleton transformer registry with default transformers
///
/// This function provides a global, thread-safe registry instance that is
/// created once and reused for all format discovery operations. This
/// eliminates the allocation overhead of creating new registries for
/// each error and ensures consistent transformer behavior.
pub fn transformer_registry() -> &'static TransformerRegistry {
    static REGISTRY: std::sync::LazyLock<TransformerRegistry> =
        std::sync::LazyLock::new(TransformerRegistry::with_defaults);
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_registry_singleton() {
        // Test that the singleton returns the same instance
        let registry1 = transformer_registry();
        let registry2 = transformer_registry();

        // Both should point to the same instance
        assert!(std::ptr::eq(registry1, registry2));

        // Should have default transformers
        assert!(!registry1.is_empty());
        assert_eq!(registry1.len(), 4); // math, string, tuple_struct, enum_variant
    }

    // More tests will be added as transformers are implemented
}
