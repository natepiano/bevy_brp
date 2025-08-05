//! Discovery engine module
//!
//! This module contains the discovery engine implementation for format recovery.
//! Phase 2 introduces the type state pattern with new modules while maintaining
//! backward compatibility with the old engine.

// Phase 2: New type state modules
mod discovery_context;
mod extras_discovery;
mod guidance;
mod new;
mod orchestrator;
mod pattern_correction;
mod recovery_result;
mod retry;
mod serialization_check;
mod type_discovery;
mod types;

// Phase 1: Old engine for backward compatibility and delegation
mod old_engine;

// Export new type state API
pub use orchestrator::discover_format_with_recovery;

// Internal tests
#[cfg(test)]
mod tests;
