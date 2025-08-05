use std::ops::Deref;

use serde_json::Value;

use super::discovery_context::DiscoveryContext;
use super::types::Correction;
use crate::brp_tools::{BrpClientError, Port};
use crate::tool::BrpMethod;

/// Generic discovery engine with type state validation
///
/// The `State` parameter ensures that only valid operations can be called
/// for the current discovery phase.
pub struct DiscoveryEngine<State> {
    pub method:         BrpMethod,
    pub port:           Port,
    pub params:         Value,
    pub original_error: BrpClientError,
    #[allow(dead_code)] // Used in future phases when State contains data
    pub state: State,
}

/// Marker type for the `TypeDiscovery` state.
/// This state is responsible for creating the discovery context by calling
/// the registry and optional extras plugin.
pub struct TypeDiscovery;

/// State type for the `SerializationCheck` state.
/// This state holds a discovery context and is responsible for checking
/// if types have required serialization traits (Bevy 0.16 workaround).
pub struct SerializationCheck(pub DiscoveryContext);

/// State type for the `ExtrasDiscovery` state.
/// This state holds a discovery context and is responsible for building
/// corrections from extras data when no serialization issues are found.
#[allow(dead_code)]
pub struct ExtrasDiscovery(pub DiscoveryContext);

/// State type for the `PatternCorrection` state.
/// This state holds a discovery context and is responsible for applying
/// pattern-based corrections when extras discovery is unavailable or fails.
#[allow(dead_code)]
pub struct PatternCorrection(pub DiscoveryContext);

/// Terminal state for retryable corrections.
/// This state holds a discovery context and retryable corrections that can
/// be applied to modify parameters and retry the original BRP call.
pub struct Retry {
    pub context:     DiscoveryContext,
    pub corrections: Vec<Correction>, /* Only retryable corrections (Correction::Candidate with
                                       * real values) */
}

/// Terminal state for educational/guidance corrections.
/// This state holds a discovery context and educational corrections that
/// provide guidance but cannot be automatically retried.
pub struct Guidance {
    pub context:     DiscoveryContext,
    pub corrections: Vec<Correction>, /* Educational/metadata corrections (Uncorrectable +
                                       * guidance-only Candidates) */
}

// Implement Deref for states that wrap DiscoveryContext
impl Deref for SerializationCheck {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for ExtrasDiscovery {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for PatternCorrection {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Retry {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl Deref for Guidance {
    type Target = DiscoveryContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

// Provide methods to extract the inner DiscoveryContext
impl SerializationCheck {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

impl ExtrasDiscovery {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

impl PatternCorrection {
    pub fn into_inner(self) -> DiscoveryContext {
        self.0
    }
}

impl Retry {
    pub const fn new(context: DiscoveryContext, corrections: Vec<Correction>) -> Self {
        Self {
            context,
            corrections,
        }
    }
}

impl Guidance {
    pub const fn new(context: DiscoveryContext, corrections: Vec<Correction>) -> Self {
        Self {
            context,
            corrections,
        }
    }
}
