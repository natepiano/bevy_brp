//! Instance count type for multi-instance launches
//!
//! Provides a type-safe wrapper around the number of instances to launch
//! with built-in validation and default values for parallel testing.

use std::ops::{Deref, RangeInclusive};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

/// Minimum number of instances (1)
pub const MIN_INSTANCE_COUNT: usize = 1;
/// Maximum number of instances (100)
pub const MAX_INSTANCE_COUNT: usize = 100;
/// Valid range for instance count
pub const VALID_INSTANCE_RANGE: RangeInclusive<usize> = MIN_INSTANCE_COUNT..=MAX_INSTANCE_COUNT;

/// A validated count of instances to launch in sequence
///
/// This type ensures the count is within 1-100 range and provides
/// a default value of 1 (single instance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize)]
pub struct InstanceCount(pub usize);

impl<'de> Deserialize<'de> for InstanceCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let count = deserialize_instance_count(deserializer)?;
        Ok(Self(count))
    }
}

impl Default for InstanceCount {
    fn default() -> Self {
        Self(1)
    }
}

impl std::fmt::Display for InstanceCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for InstanceCount {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Deserialize and validate instance count
///
/// Ensures the count is within the valid range (1-100)
/// Accepts both number and string inputs for compatibility
pub fn deserialize_instance_count<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    use std::fmt;

    use serde::de::{self, Visitor};

    struct InstanceCountVisitor;

    impl Visitor<'_> for InstanceCountVisitor {
        type Value = usize;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an instance count as usize or string (1-100)")
        }

        fn visit_u64<E>(self, value: u64) -> Result<usize, E>
        where
            E: de::Error,
        {
            usize::try_from(value)
                .map_err(|_| E::custom(format!("instance count {value} is out of usize range")))
        }

        fn visit_str<E>(self, value: &str) -> Result<usize, E>
        where
            E: de::Error,
        {
            value
                .parse::<usize>()
                .map_err(|_| E::custom(format!("invalid instance count string: {value}")))
        }
    }

    let count = deserializer.deserialize_any(InstanceCountVisitor)?;

    if VALID_INSTANCE_RANGE.contains(&count) {
        Ok(count)
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid instance count {}: must be in range {}-{}",
            count,
            VALID_INSTANCE_RANGE.start(),
            VALID_INSTANCE_RANGE.end()
        )))
    }
}
