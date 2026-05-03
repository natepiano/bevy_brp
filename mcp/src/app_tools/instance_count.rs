//! Instance count type for multi-instance launches
//!
//! Provides a type-safe wrapper around the number of instances to launch
//! with built-in validation and default values for parallel testing.

use std::ops::Deref;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::constants::VALID_INSTANCE_RANGE;

/// Count of instances to launch in sequence
/// Validates count is within 1-100 - defaults to 1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
#[serde(try_from = "u16")]
pub struct InstanceCount(pub u16);

impl TryFrom<u16> for InstanceCount {
    type Error = String;

    fn try_from(count: u16) -> Result<Self, Self::Error> {
        if VALID_INSTANCE_RANGE.contains(&count) {
            Ok(Self(count))
        } else {
            Err(format!(
                "Invalid instance count {count}: must be in range {}-{}",
                VALID_INSTANCE_RANGE.start(),
                VALID_INSTANCE_RANGE.end()
            ))
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
