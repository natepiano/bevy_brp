//! Instance count type for multi-instance launches
//!
//! Provides a type-safe wrapper around the number of instances to launch
//! with built-in validation and default values for parallel testing.

use std::ops::Deref;
use std::ops::RangeInclusive;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

use crate::support;

/// Minimum number of instances (1)
const MIN_INSTANCE_COUNT: u16 = 1;
/// Maximum number of instances (100)
const MAX_INSTANCE_COUNT: u16 = 100;
/// Valid range for instance count
const VALID_INSTANCE_RANGE: RangeInclusive<u16> = MIN_INSTANCE_COUNT..=MAX_INSTANCE_COUNT;

/// Count of instances to launch in sequence
/// Validates count is within 1-100 - defaults to 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize)]
pub struct InstanceCount(pub u16);

impl<'de> Deserialize<'de> for InstanceCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let count: u16 = support::deserialize_number_or_string(deserializer)?;
        if VALID_INSTANCE_RANGE.contains(&count) {
            Ok(Self(count))
        } else {
            Err(serde::de::Error::custom(format!(
                "Invalid instance count {count}: must be in range {}-{}",
                VALID_INSTANCE_RANGE.start(),
                VALID_INSTANCE_RANGE.end()
            )))
        }
    }
}

impl Default for InstanceCount {
    fn default() -> Self { Self(1) }
}

impl std::fmt::Display for InstanceCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
}

impl Deref for InstanceCount {
    type Target = u16;

    fn deref(&self) -> &Self::Target { &self.0 }
}
