//! Find-entities-by-name handler for BRP extras
//!
//! Locates entities by their `Name` component value using a small wildcard
//! syntax on the query pattern: `*suffix` (ends-with), `prefix*`
//! (starts-with), `*substr*` (contains), otherwise exact match.

use bevy::prelude::*;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde_json::Value;
use serde_json::json;

use crate::constants::PARAM_NAME;
use crate::constants::RESPONSE_ENTITY_FIELD;
use crate::constants::RESPONSE_NAME_FIELD;

/// Match mode inferred from `*` placement in the query pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchMode {
    Exact,
    StartsWith,
    EndsWith,
    Contains,
}

/// Splits a query pattern into a match mode and the bare needle (stars stripped).
fn classify(pattern: &str) -> (MatchMode, &str) {
    let leading = pattern.starts_with('*');
    let trailing = pattern.len() > 1 && pattern.ends_with('*');

    match (leading, trailing) {
        (true, true) => (MatchMode::Contains, &pattern[1..pattern.len() - 1]),
        (true, false) => (MatchMode::EndsWith, &pattern[1..]),
        (false, true) => (MatchMode::StartsWith, &pattern[..pattern.len() - 1]),
        (false, false) => (MatchMode::Exact, pattern),
    }
}

fn matches(mode: MatchMode, needle: &str, name: &str) -> bool {
    match mode {
        MatchMode::Exact => name == needle,
        MatchMode::StartsWith => name.starts_with(needle),
        MatchMode::EndsWith => name.ends_with(needle),
        MatchMode::Contains => name.contains(needle),
    }
}

/// Handler for `brp_extras/find_entities_by_name` requests.
///
/// Returns a JSON array of `{ entity, name }` matches. An entity whose name
/// equals the raw (unstripped) pattern exactly is sorted first; remaining
/// matches are ordered by entity id for determinism.
pub(crate) fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let pattern = params
        .as_ref()
        .and_then(|value| value.get(PARAM_NAME))
        .and_then(Value::as_str)
        .ok_or_else(|| BrpError {
            code:    INVALID_PARAMS,
            message: format!("Missing or non-string required '{PARAM_NAME}' parameter"),
            data:    None,
        })?;

    let (mode, needle) = classify(pattern);

    let mut matched: Vec<(Entity, String)> = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .filter(|(_, name)| matches(mode, needle, name.as_str()))
        .map(|(entity, name)| (entity, name.as_str().to_string()))
        .collect();

    matched.sort_by(|(entity_a, name_a), (entity_b, name_b)| {
        let exact_a = name_a == pattern;
        let exact_b = name_b == pattern;
        exact_b.cmp(&exact_a).then(entity_a.cmp(entity_b))
    });

    let entities: Vec<Value> = matched
        .into_iter()
        .map(|(entity, name)| {
            json!({
                RESPONSE_ENTITY_FIELD: entity.to_bits(),
                RESPONSE_NAME_FIELD: name,
            })
        })
        .collect();

    Ok(Value::Array(entities))
}
