//! Extension traits for JSON field access and string collection utilities
//!
//! This module provides generic traits for:
//! - Type-safe JSON field access using any type that implements `AsRef<str>`
//! - Converting iterators to string collections

use serde::Serialize;
use serde_json::{Map, Value};

/// Extension trait for type-safe JSON field access
pub trait JsonFieldAccess {
    /// Get field value using any type that can be a string reference
    fn get_field<T: AsRef<str>>(&self, field: T) -> Option<&Value>;

    /// Get field value as string
    fn get_field_str<T: AsRef<str>>(&self, field: T) -> Option<&str>;

    /// Get field value as mutable object
    fn get_field_object_mut<T: AsRef<str>>(&mut self, field: T) -> Option<&mut Map<String, Value>>;

    /// Insert field with value using any type that converts to String
    fn insert_field<T: Into<String>>(&mut self, field: T, value: Value);

    /// Insert field with string value using any type that converts to String
    fn insert_field_str<T: Into<String>>(&mut self, field: T, value: &str);

    /// Set or update a field value using types that can serialize
    fn set_field<F, V>(&mut self, field: F, value: V)
    where
        F: AsRef<str>,
        V: Serialize;
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

    fn insert_field<T: Into<String>>(&mut self, field: T, value: Value) {
        if let Some(obj) = self.as_object_mut() {
            obj.insert(field.into(), value);
        }
    }

    fn insert_field_str<T: Into<String>>(&mut self, field: T, value: &str) {
        if let Some(obj) = self.as_object_mut() {
            obj.insert(field.into(), Self::String(value.to_string()));
        }
    }

    fn set_field<F, V>(&mut self, field: F, value: V)
    where
        F: AsRef<str>,
        V: Serialize,
    {
        if let Some(obj) = self.as_object_mut() {
            if let Ok(json_value) = serde_json::to_value(value) {
                obj.insert((*field.as_ref()).to_string(), json_value);
            }
        }
    }
}

impl JsonFieldAccess for Map<String, Value> {
    fn get_field<T: AsRef<str>>(&self, field: T) -> Option<&Value> {
        self.get(field.as_ref())
    }

    fn get_field_str<T: AsRef<str>>(&self, field: T) -> Option<&str> {
        self.get(field.as_ref()).and_then(Value::as_str)
    }

    fn get_field_object_mut<T: AsRef<str>>(&mut self, field: T) -> Option<&mut Map<String, Value>> {
        self.get_mut(field.as_ref()).and_then(Value::as_object_mut)
    }

    fn insert_field<T: Into<String>>(&mut self, field: T, value: Value) {
        self.insert(field.into(), value);
    }

    fn insert_field_str<T: Into<String>>(&mut self, field: T, value: &str) {
        self.insert(field.into(), Value::String(value.to_string()));
    }

    fn set_field<F, V>(&mut self, field: F, value: V)
    where
        F: AsRef<str>,
        V: Serialize,
    {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.insert((*field.as_ref()).to_string(), json_value);
        }
    }
}

/// Extension trait for converting iterators to `Vec<String>`
///
/// This trait provides a convenient way to collect iterators of string-convertible
/// items into a vector of strings, replacing the common `.map(String::from).collect()`
/// pattern with a more expressive `.into_strings()` call.
///
/// # Examples
///
/// ```
/// use json_traits::IntoStrings;
///
/// // Convert iterator of &str to Vec<String>
/// let strings = ["a", "b", "c"].iter().into_strings();
///
/// // Works with filter chains
/// let filtered = ["hello", "", "world"]
///     .iter()
///     .filter(|s| !s.is_empty())
///     .into_strings();
///
/// // Works with enums that implement Into<String>
/// let variants = enum_values.iter().into_strings();
/// ```
pub trait IntoStrings<T> {
    /// Convert an iterator of items that can become strings into a `Vec<String>`
    fn into_strings(self) -> Vec<String>;
}

impl<I, T> IntoStrings<T> for I
where
    I: Iterator<Item = T>,
    T: Into<String>,
{
    fn into_strings(self) -> Vec<String> {
        self.map(Into::into).collect()
    }
}
