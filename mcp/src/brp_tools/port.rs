//! Port type for BRP connections
//!
//! Provides a type-safe wrapper around port numbers with built-in validation
//! and default values for BRP connections.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::brp_tools::constants::{DEFAULT_BRP_PORT, VALID_PORT_RANGE};

/// A validated port number for BRP connections
///
/// This type ensures port numbers are within the valid range and provides
/// a default value of 15702 (the standard BRP port).
#[derive(Debug, Clone, Copy, JsonSchema, Serialize)]
pub struct Port(pub u16);

impl<'de> Deserialize<'de> for Port {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let port = deserialize_port(deserializer)?;
        Ok(Self(port))
    }
}

impl Default for Port {
    fn default() -> Self {
        Self(DEFAULT_BRP_PORT)
    }
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Deserialize and validate port numbers
///
/// Ensures the port is within the valid range (1024-65534)
pub fn deserialize_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let port = u16::deserialize(deserializer)?;

    if VALID_PORT_RANGE.contains(&port) {
        Ok(port)
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid port {}: must be in range {}-{}",
            port,
            VALID_PORT_RANGE.start(),
            VALID_PORT_RANGE.end()
        )))
    }
}
