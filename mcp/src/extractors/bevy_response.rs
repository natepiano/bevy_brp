//! Extractor for Bevy BRP responses

use serde_json::Value;

use crate::brp_tools::constants::JSON_FIELD_ENTITY;

/// Extractor for data from Bevy BRP responses
pub struct BevyResponseExtractor<'a> {
    response: &'a Value,
}

impl<'a> BevyResponseExtractor<'a> {
    /// Create a new extractor from a Bevy BRP response
    pub const fn new(response: &'a Value) -> Self {
        Self { response }
    }

    /// Pass through the BRP response data
    pub const fn pass_through(&self) -> &Value {
        self.response
    }

    /// Count elements in an array from the response data
    pub fn entity_count(&self) -> usize {
        // Check if data is wrapped in a structure with a "data" field
        self.response
            .as_object()
            .and_then(|obj| obj.get("data"))
            .map_or_else(
                || self.response.as_array().map_or(0, std::vec::Vec::len),
                |inner_data| inner_data.as_array().map_or(0, std::vec::Vec::len),
            )
    }

    /// Extract count from data for local operations - prioritizes "count" field over array length
    pub fn count(&self) -> Value {
        // Check if data is wrapped in a structure with a "count" field
        self.response
            .as_object()
            .and_then(|obj| obj.get("count"))
            .map_or_else(
                || {
                    self.response
                        .as_array()
                        .map_or(0, std::vec::Vec::len)
                        .into()
                },
                std::clone::Clone::clone,
            )
    }

    /// Extract entity ID from response data (for spawn operation)
    pub fn spawned_entity_id(&self) -> Value {
        self.response
            .get(JSON_FIELD_ENTITY)
            .cloned()
            .unwrap_or_else(|| Value::Number(serde_json::Number::from(0)))
    }

    /// Extract total component count from nested query results
    pub fn query_component_count(&self) -> Value {
        let total = self.response.as_array().map_or(0, |entities| {
            entities
                .iter()
                .filter_map(|e| e.as_object())
                .map(serde_json::Map::len)
                .sum::<usize>()
        });
        Value::Number(serde_json::Number::from(total))
    }
}
