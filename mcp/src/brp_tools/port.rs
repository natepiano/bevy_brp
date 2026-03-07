//! Port type for BRP connections
//!
//! Provides a type-safe wrapper around port numbers with built-in validation
//! and default values for BRP connections.

use std::ops::Deref;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

use crate::brp_tools::constants::DEFAULT_BRP_EXTRAS_PORT;
use crate::brp_tools::constants::VALID_PORT_RANGE;
use crate::serde_helpers::deserialize_number_or_string;

/// Port number for BRP - defaults to 15702
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize)]
pub struct Port(pub u16);

impl<'de> Deserialize<'de> for Port {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let port: u16 = deserialize_number_or_string(deserializer)?;
        if VALID_PORT_RANGE.contains(&port) {
            Ok(Self(port))
        } else {
            Err(serde::de::Error::custom(format!(
                "Invalid port {port}: must be in range {}-{}",
                VALID_PORT_RANGE.start(),
                VALID_PORT_RANGE.end()
            )))
        }
    }
}

impl Default for Port {
    fn default() -> Self { Self(DEFAULT_BRP_EXTRAS_PORT) }
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
}

impl Deref for Port {
    type Target = u16;

    fn deref(&self) -> &Self::Target { &self.0 }
}
