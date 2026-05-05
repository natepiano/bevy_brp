//! Mutation path newtype for BRP operations.

use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;

use serde::Deserialize;
use serde::Serialize;

/// Newtype for a mutation path used in BRP operations (e.g., ".translation.x")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationPath(String);

impl Deref for MutationPath {
    type Target = String;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl From<String> for MutationPath {
    fn from(path: String) -> Self { Self(path) }
}

impl From<&str> for MutationPath {
    fn from(path: &str) -> Self { Self(path.to_string()) }
}

impl Display for MutationPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}
