//! Type discovery context for managing type information from multiple sources
//!
//! `DiscoveryContext` provides a unified interface for accessing type information
//! discovered from various sources (registry, extras plugin, etc.). The context
//! automatically extracts type names and their original values from BRP method
//! parameters during construction, ensuring consistent value propagation throughout
//! the discovery process.
//!
//! # Value Propagation
//!
//! The context combines three key operations:
//! 1. Type extraction from method parameters (spawn components, mutation targets, etc.)
//! 2. Value extraction to preserve original user input
//! 3. Registry integration to fetch type metadata
//!
//! This unified approach eliminates repeated parameter parsing and ensures that
//! original values are available for format transformations at every discovery level.

// Submodules
mod comparison;
mod context;
mod hardcoded_formats;
mod registry_cache;
mod types;

// Re-export main types
pub use context::DiscoveryContext;
