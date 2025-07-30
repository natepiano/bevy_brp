/// Trait for types that can access fields in JSON objects
pub trait JsonFieldAccess: AsRef<str> {
    /// Get field value from a JSON object
    fn get_from<'a>(&self, value: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
        value.get(self.as_ref())
    }

    /// Get field value as string from a JSON object
    fn get_str_from<'a>(&self, value: &'a serde_json::Value) -> Option<&'a str> {
        value.get(self.as_ref()).and_then(|v| v.as_str())
    }

    /// Get field value as object from a JSON object
    fn get_object_from<'a>(
        &self,
        value: &'a serde_json::Value,
    ) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
        value.get(self.as_ref()).and_then(|v| v.as_object())
    }
}
