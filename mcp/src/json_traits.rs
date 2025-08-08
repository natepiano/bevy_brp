//! Extension trait for type-safe JSON field access
//!
//! This module provides a generic trait for accessing JSON object fields
//! using any type that implements `AsRef<str>`, enabling type-safe field
//! access with enums throughout the codebase.

use serde_json::{Map, Value};

/// Extension trait for type-safe JSON field access
pub trait JsonFieldAccess {
    /// Get field value using any type that can be a string reference
    fn get_field<T: AsRef<str>>(&self, field: T) -> Option<&Value>;

    /// Get field value as string
    fn get_field_str<T: AsRef<str>>(&self, field: T) -> Option<&str>;

    /// Get field value as mutable object
    fn get_field_object_mut<T: AsRef<str>>(&mut self, field: T) -> Option<&mut Map<String, Value>>;
}

impl JsonFieldAccess for Value {
    fn get_field<T: AsRef<str>>(&self, field: T) -> Option<&Value> {
        self.get(field.as_ref())
    }

    fn get_field_str<T: AsRef<str>>(&self, field: T) -> Option<&str> {
        self.get(field.as_ref()).and_then(Self::as_str)
    }

    fn get_field_object_mut<T: AsRef<str>>(&mut self, field: T) -> Option<&mut Map<String, Value>> {
        self.get_mut(field.as_ref()).and_then(Self::as_object_mut)
    }
}
