//! Phases module for the format discovery engine refactoring
//! Each phase handles a specific part of the discovery process

pub mod context;
// pub mod error_analysis; // Removed: function was unused
// pub mod initial_attempt; // Removed: unused module
pub mod result_building;
pub mod tier_execution; // Legacy module - now only contains comments
