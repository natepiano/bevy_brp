//! Public API response types for the `brp_type_guide` tool
//!
//! This module contains the strongly-typed structures that form the public API
//! for type schema discovery results. These types are separate from the internal
//! processing types to provide a clean, stable API contract.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

pub use super::brp_type_name::BrpTypeName;
use super::type_guide::TypeGuide;
use super::type_kind::TypeKind;

/// Enum for BRP supported operations
/// Each operation has specific requirements based on type registration and traits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, AsRefStr, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BrpSupportedOperation {
    /// Get operation - requires type in registry
    Get,
    /// Insert operation - requires Serialize + Deserialize traits
    Insert,
    /// Mutate operation - requires mutable type (struct/tuple)
    Mutate,
    /// Query operation - requires type in registry
    Query,
    /// Spawn operation - requires Serialize + Deserialize traits
    Spawn,
}

impl From<BrpSupportedOperation> for String {
    fn from(op: BrpSupportedOperation) -> Self {
        op.as_ref().to_string()
    }
}

/// Schema information extracted from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Category of the type (Struct, Enum, etc.) from registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_kind: Option<TypeKind>,
    /// Field definitions from the registry schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
    /// Required fields list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Module path of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
    /// Crate name of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crate_name: Option<String>,
}

/// Bevy reflection trait names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum ReflectTrait {
    Component,
    Resource,
    Serialize,
    Deserialize,
}

/// response structure
#[derive(Debug, Clone, Serialize)]
pub struct TypeGuideResponse {
    /// Number of types successfully discovered
    pub discovered_count: usize,
    /// List of type names that were requested
    pub requested_types: Vec<String>,
    /// Summary statistics for the discovery operation
    pub summary: TypeGuideSummary,
    /// Detailed information for each type, keyed by type name
    pub type_guide: HashMap<BrpTypeName, TypeGuide>,
}

/// Summary statistics for the discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeGuideSummary {
    /// Number of types that failed discovery
    pub failed_discoveries: usize,
    /// Number of types successfully discovered
    pub successful_discoveries: usize,
    /// Total number of types requested
    pub total_requested: usize,
}
