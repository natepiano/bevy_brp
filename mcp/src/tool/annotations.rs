//! Ergonomic tool annotations for BRP tools

use rmcp::model::ToolAnnotations;
use strum::AsRefStr;

/// Tool categories for logical grouping and sorting
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr)]
pub enum ToolCategory {
    #[strum(serialize = "App")]
    App,
    #[strum(serialize = "Component")]
    Component,
    #[strum(serialize = "Discovery")]
    Discovery,
    #[strum(serialize = "Dynamic BRP")]
    DynamicBrp,
    #[strum(serialize = "Entity")]
    Entity,
    #[strum(serialize = "Extras")]
    Extras,
    #[strum(serialize = "Logging")]
    Logging,
    #[strum(serialize = "Resource")]
    Resource,
    #[strum(serialize = "Watch")]
    Watch,
    #[strum(serialize = "Watch Monitoring")]
    WatchMonitoring,
}

/// Ergonomic tool annotations for BRP tools
#[derive(Debug, Clone)]
pub struct BrpToolAnnotations {
    pub title:                 String,
    pub category:              ToolCategory,
    pub environment_impact:    EnvironmentImpact,
    pub domain_of_interaction: DomainOfInteraction,
}

/// Describes how a tool interacts with its environment
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentImpact {
    /// Tool only reads data, does not modify environment
    ReadOnly,
    /// Tool destroys/removes data, safe to repeat with same args
    DestructiveIdempotent,
    /// Tool destroys/removes data, may have side effects if repeated
    DestructiveNonIdempotent,
    /// Tool adds/updates data, safe to repeat with same args
    AdditiveIdempotent,
    /// Tool adds/creates new data, creates new things if repeated
    AdditiveNonIdempotent,
}

/// Describes the domain of interaction for a tool
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainOfInteraction {
    /// Tool interacts with external entities/systems
    OpenWorld,
    /// Tool operates within a closed domain (our BRP ecosystem)
    LocalOnly,
}

impl BrpToolAnnotations {
    pub fn new(
        title: impl Into<String>,
        category: ToolCategory,
        environment_impact: EnvironmentImpact,
    ) -> Self {
        Self {
            title: title.into(),
            category,
            environment_impact,
            domain_of_interaction: DomainOfInteraction::LocalOnly, // Default for all our tools
        }
    }

    pub const fn with_domain(mut self, domain: DomainOfInteraction) -> Self {
        self.domain_of_interaction = domain;
        self
    }
}

impl From<BrpToolAnnotations> for ToolAnnotations {
    fn from(brp: BrpToolAnnotations) -> Self {
        let (read_only, destructive, idempotent) = match brp.environment_impact {
            EnvironmentImpact::ReadOnly => (Some(true), None, None),
            EnvironmentImpact::DestructiveIdempotent | EnvironmentImpact::AdditiveIdempotent => {
                // MCP client requires destructive_hint: Some(true) to show annotations
                // So we mark additive tools as "destructive" even though they're safe
                (Some(false), Some(true), Some(true))
            }
            EnvironmentImpact::DestructiveNonIdempotent
            | EnvironmentImpact::AdditiveNonIdempotent => (Some(false), Some(true), Some(false)),
        };

        let open_world = match brp.domain_of_interaction {
            DomainOfInteraction::OpenWorld => Some(true),
            DomainOfInteraction::LocalOnly => Some(false),
        };

        Self {
            title:            Some(brp.title),
            read_only_hint:   read_only,
            destructive_hint: destructive,
            idempotent_hint:  idempotent,
            open_world_hint:  open_world,
        }
    }
}
