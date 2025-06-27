//! Format discovery structures for BRP component introspection
//!
//! This module provides structures for discovering and representing component
//! format information that can be used for proper BRP operation formatting.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Complete format information for a component type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    /// The fully-qualified type name
    pub type_name:     String,
    /// Format information for spawning operations
    pub spawn_format:  SpawnInfo,
    /// Format information for mutation operations
    pub mutation_info: MutationInfo,
}

/// Information about how to format data for spawn operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnInfo {
    /// Example data structure for spawn operations
    pub example:     serde_json::Value,
    /// Description of the expected format
    pub description: String,
}

/// Information about available mutation paths and formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationInfo {
    /// Available field paths for mutation
    pub fields:      HashMap<String, FieldInfo>,
    /// Root-level description
    pub description: String,
}

/// Information about a specific field that can be mutated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    /// The mutation path (e.g., "translation.x")
    pub path:        String,
    /// The expected value type/format
    pub value_type:  String,
    /// Example value
    pub example:     serde_json::Value,
    /// Human-readable description
    pub description: String,
}
