//! Port type for BRP connections
//!
//! Provides a type-safe wrapper around port numbers with built-in validation
//! and default values for BRP connections.

use std::ops::Deref;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::brp_tools::constants::{DEFAULT_BRP_EXTRAS_PORT, VALID_PORT_RANGE};

/// A validated port number for BRP connections
///
/// This type ensures port numbers are within the valid range and provides
/// a default value of 15702 (the standard BRP port).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize)]
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
        Self(DEFAULT_BRP_EXTRAS_PORT)
    }
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for Port {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Deserialize and validate port numbers
///
/// Ensures the port is within the valid range (1024-65534)
/// Accepts both number and string inputs for compatibility
pub fn deserialize_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    use std::fmt;

    use serde::de::{self, Visitor};

    struct PortVisitor;

    impl Visitor<'_> for PortVisitor {
        type Value = u16;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a port number as u16 or string")
        }

        fn visit_u16<E>(self, value: u16) -> Result<u16, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<u16, E>
        where
            E: de::Error,
        {
            u16::try_from(value)
                .map_err(|_| E::custom(format!("port number {value} is out of u16 range")))
        }

        fn visit_str<E>(self, value: &str) -> Result<u16, E>
        where
            E: de::Error,
        {
            value
                .parse::<u16>()
                .map_err(|_| E::custom(format!("invalid port string: {value}")))
        }
    }

    let port = deserializer.deserialize_any(PortVisitor)?;

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
