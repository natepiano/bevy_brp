//! Discovery engine module
//!
//! This module contains the discovery engine implementation for format recovery.
//! During Phase 1 of the type state pattern refactoring, this temporarily re-exports
//! everything from the old engine to maintain compatibility.

// Temporarily re-export everything from old_engine
mod old_engine;
pub use old_engine::*;
