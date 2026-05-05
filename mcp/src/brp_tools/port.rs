//! Port type for BRP connections
//!
//! Provides a type-safe wrapper around port numbers with built-in validation
//! and default values for BRP connections.

use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::constants::DEFAULT_BRP_EXTRAS_PORT;
use super::constants::VALID_PORT_RANGE;

/// Port number for BRP - defaults to 15702
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, Serialize, Deserialize)]
#[serde(try_from = "u16")]
pub struct Port(pub u16);

impl TryFrom<u16> for Port {
    type Error = String;

    fn try_from(port: u16) -> Result<Self, Self::Error> {
        if VALID_PORT_RANGE.contains(&port) {
            Ok(Self(port))
        } else {
            Err(format!(
                "Invalid port {port}: must be in range {}-{}",
                VALID_PORT_RANGE.start(),
                VALID_PORT_RANGE.end()
            ))
        }
    }
}

impl Default for Port {
    fn default() -> Self { Self(DEFAULT_BRP_EXTRAS_PORT) }
}

impl Display for Port {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
}

impl Deref for Port {
    type Target = u16;

    fn deref(&self) -> &Self::Target { &self.0 }
}
