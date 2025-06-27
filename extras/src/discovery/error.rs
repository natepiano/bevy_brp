//! Unified error handling for format discovery operations
//!
//! This module provides a comprehensive error handling system that eliminates
//! the need for separate debug and error handling function variants.

use serde_json::{Value, json};

use super::registry::RegistryError;

/// Comprehensive error type for all discovery operations
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),

    #[error("Unsupported type: {message}")]
    UnsupportedType { message: String },

    #[error("Format generation error: {message}")]
    FormatGeneration { message: String },
}

impl DiscoveryError {
    /// Create an unsupported type error
    pub fn unsupported_type(message: impl Into<String>) -> Self {
        Self::UnsupportedType {
            message: message.into(),
        }
    }

    /// Create a format generation error
    pub fn format_generation(message: impl Into<String>) -> Self {
        Self::FormatGeneration {
            message: message.into(),
        }
    }

    /// Create a standardized error for type not supported for operation
    pub fn type_not_supported_for(type_name: &str, operation: &str) -> Self {
        Self::UnsupportedType {
            message: format!("{operation} not supported for type: {type_name}"),
        }
    }

    /// Create a standardized error for type casting failures
    pub fn type_cast_failed(from_type: &str, to_type: &str) -> Self {
        Self::FormatGeneration {
            message: format!("Failed to cast {from_type} to {to_type}"),
        }
    }

    /// Create a standardized error for missing example
    pub fn no_example_for_type(type_name: &str) -> Self {
        Self::UnsupportedType {
            message: format!("No example available for type: {type_name}"),
        }
    }

    /// Convert to a JSON error map for BRP responses
    pub fn to_json_error(&self) -> serde_json::Map<String, Value> {
        let mut error_info = serde_json::Map::new();

        match self {
            Self::Registry(registry_error) => {
                return registry_error.to_json_error();
            }
            Self::UnsupportedType { message } => {
                error_info.insert("reason".to_string(), json!("Unsupported type"));
                error_info.insert("details".to_string(), json!(message));
            }
            Self::FormatGeneration { message } => {
                error_info.insert("reason".to_string(), json!("Format generation error"));
                error_info.insert("details".to_string(), json!(message));
            }
        }

        error_info
    }
}

/// Result type for discovery operations
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Context for collecting debug information during discovery operations
#[derive(Debug, Default)]
pub struct DebugContext {
    pub messages: Vec<String>,
}

impl DebugContext {
    /// Create a new debug context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a debug message
    pub fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    /// Convert to a mutable Vec for compatibility with existing code
    #[allow(clippy::missing_const_for_fn)] // False positive - can't be const with mutable return
    pub fn as_mut_vec(&mut self) -> &mut Vec<String> {
        &mut self.messages
    }
}
