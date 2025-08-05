//! Discovery engine module
//!
//! This module contains the discovery engine implementation for format recovery.
//! Phase 2 introduces the type state pattern with new modules while maintaining
//! backward compatibility with the old engine.

// Phase 2: New type state modules
mod new;
mod type_discovery;
mod types;

// Phase 1: Old engine for backward compatibility and delegation
mod old_engine;

// Export new type state API
pub use new::DiscoveryEngine;
// Export old engine API for backward compatibility (used internally)
pub(super) use old_engine::DiscoveryEngine as OldDiscoveryEngine;
#[allow(unused_imports)] // Used by tests
pub use types::TypeDiscovery;
