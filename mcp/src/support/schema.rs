use std::sync::Arc;

use serde_json::{Map, Value};

/// Builder for creating JSON schemas for tool registration
pub struct SchemaBuilder {
    properties: Map<String, Value>,
    required:   Vec<String>,
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            properties: Map::new(),
            required:   Vec::new(),
        }
    }

    /// Add a string property to the schema
    pub fn add_string_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "string".into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a string array property to the schema
    pub fn add_string_array_property(
        mut self,
        name: &str,
        description: &str,
        required: bool,
    ) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "array".into());

        let mut items = Map::new();
        items.insert("type".to_string(), "string".into());
        prop.insert("items".to_string(), items.into());

        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a number property to the schema
    pub fn add_number_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "number".into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a boolean property to the schema
    pub fn add_boolean_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), "boolean".into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Add a property that can be any type (object, array, null, etc.)
    pub fn add_any_property(mut self, name: &str, description: &str, required: bool) -> Self {
        let mut prop = Map::new();
        prop.insert("type".to_string(), vec!["object", "array", "null"].into());
        prop.insert("description".to_string(), description.into());
        self.properties.insert(name.to_string(), prop.into());

        if required {
            self.required.push(name.to_string());
        }

        self
    }

    /// Build the final schema
    pub fn build(self) -> Arc<Map<String, Value>> {
        let mut schema = Map::new();
        schema.insert("type".to_string(), "object".into());
        schema.insert("properties".to_string(), self.properties.into());

        if !self.required.is_empty() {
            schema.insert("required".to_string(), self.required.into());
        }

        Arc::new(schema)
    }
}
