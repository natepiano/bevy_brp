//! Phases module for the format discovery engine refactoring
//! Each phase handles a specific part of the discovery process

pub mod context;
pub mod error_analysis;
pub mod initial_attempt;
pub mod result_building;
pub mod tier_execution;
