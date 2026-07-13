//! Find entities by their reflected Bevy [`Name`] component.

use std::any::type_name;
use std::collections::HashMap;

use async_trait::async_trait;
use bevy::prelude::Name;
use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use error_stack::Report;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::BrpClient;
use crate::brp_tools::Port;
use crate::brp_tools::ResponseStatus;
use crate::error::Error;
use crate::error::Result;
use crate::tool::BrpMethod;
use crate::tool::ToolFn;

/// How an entity's case-sensitive [`Name`] must match the requested text.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NameMatchMode {
    /// Match the complete name.
    #[default]
    Exact,
    /// Match names that start with the requested text.
    Prefix,
    /// Match names that end with the requested text.
    Suffix,
    /// Match names that contain the requested text.
    Contains,
}

/// Parameters for local entity-name discovery through standard BRP.
#[derive(Clone, Deserialize, JsonSchema, ParamStruct, Serialize)]
pub struct FindEntitiesByNameParams {
    /// Case-sensitive text to compare with reflected Bevy `Name` components.
    pub name:       String,
    /// Comparison mode. Defaults to `exact`; asterisks have no special meaning.
    #[serde(default)]
    pub match_mode: NameMatchMode,
    /// The BRP port (default: 15702).
    #[serde(default)]
    pub port:       Port,
}

/// One entity returned by name discovery.
#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct NamedEntity {
    /// Canonical Bevy entity ID for later BRP operations.
    pub entity: u64,
    /// The complete reflected Bevy `Name`.
    pub name:   String,
}

/// Result of local entity-name discovery.
#[derive(Serialize, ResultStruct)]
pub struct FindEntitiesByNameResult {
    /// Matching entities in ascending entity-ID order.
    #[to_result]
    pub entities:         Vec<NamedEntity>,
    /// Number of matching entities.
    #[to_metadata]
    pub entity_count:     usize,
    /// Message template for formatting responses.
    #[to_message(message_template = "Found {entity_count} named entities")]
    pub message_template: String,
}

/// Local MCP handler that composes a standard BRP `world.query` request.
pub struct WorldFindEntitiesByName;

#[async_trait]
impl ToolFn for WorldFindEntitiesByName {
    type Output = FindEntitiesByNameResult;
    type Params = FindEntitiesByNameParams;

    async fn handle_impl(
        &self,
        params: FindEntitiesByNameParams,
    ) -> Result<FindEntitiesByNameResult> {
        let entities = find_entities_by_name(&params.name, params.match_mode, params.port).await?;
        let entity_count = entities.len();
        Ok(FindEntitiesByNameResult::new(entities, entity_count))
    }
}

#[derive(Serialize)]
struct NameQueryData {
    components: Vec<String>,
}

#[derive(Serialize)]
struct NameQueryFilter {
    with: Vec<String>,
}

#[derive(Serialize)]
struct NameQueryParams {
    data:   NameQueryData,
    filter: NameQueryFilter,
}

#[derive(Deserialize)]
struct NameQueryRow {
    entity:     u64,
    components: HashMap<String, Value>,
}

/// Query and filter reflected names through standard BRP.
///
/// The screenshot MCP handler uses this operation with [`NameMatchMode::Exact`]
/// before sending a canonical entity ID to `bevy_brp_extras`.
pub(super) async fn find_entities_by_name(
    name: &str,
    match_mode: NameMatchMode,
    port: Port,
) -> Result<Vec<NamedEntity>> {
    let params = build_name_query_params()?;
    let client = BrpClient::new(BrpMethod::WorldQuery, port, Some(params));
    let response = client.execute_raw().await?;
    parse_name_query_response(response, name, match_mode, port)
}

fn build_name_query_params() -> Result<Value> {
    let component = type_name::<Name>().to_string();
    let params = NameQueryParams {
        data:   NameQueryData {
            components: vec![component.clone()],
        },
        filter: NameQueryFilter {
            with: vec![component],
        },
    };

    serde_json::to_value(params).map_err(|error| {
        Error::InvalidState(format!(
            "Failed to serialize the Name world.query request: {error}"
        ))
        .into()
    })
}

fn parse_name_query_response(
    response: ResponseStatus,
    requested_name: &str,
    match_mode: NameMatchMode,
    port: Port,
) -> Result<Vec<NamedEntity>> {
    match response {
        ResponseStatus::Success(Some(value)) => {
            parse_name_query_rows(value, requested_name, match_mode, port)
        },
        ResponseStatus::Success(None) => Err(name_query_decode_error(
            port,
            "world.query returned no result",
        )),
        ResponseStatus::Error(error) => Err(name_query_brp_error(
            port,
            error.code,
            error.message,
            error.data,
        )),
    }
}

fn parse_name_query_rows(
    value: Value,
    requested_name: &str,
    match_mode: NameMatchMode,
    port: Port,
) -> Result<Vec<NamedEntity>> {
    let rows = serde_json::from_value::<Vec<NameQueryRow>>(value)
        .map_err(|error| name_query_decode_error(port, error))?;
    let component = type_name::<Name>();
    let mut entities = rows
        .into_iter()
        .map(|row| {
            let name = row
                .components
                .get(component)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    name_query_decode_error(
                        port,
                        format!(
                            "entity {} has no string `{component}` component",
                            row.entity
                        ),
                    )
                })?;

            Ok(NamedEntity {
                entity: row.entity,
                name:   name.to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    entities.retain(|entity| name_matches(&entity.name, requested_name, match_mode));
    entities.sort_unstable_by_key(|entity| entity.entity);
    Ok(entities)
}

fn name_matches(candidate: &str, requested_name: &str, match_mode: NameMatchMode) -> bool {
    match match_mode {
        NameMatchMode::Exact => candidate == requested_name,
        NameMatchMode::Prefix => candidate.starts_with(requested_name),
        NameMatchMode::Suffix => candidate.ends_with(requested_name),
        NameMatchMode::Contains => candidate.contains(requested_name),
    }
}

fn name_query_decode_error(port: Port, error: impl ToString) -> Report<Error> {
    Error::tool_call_failed_with_details(
        format!("Unable to decode world.query name response from port {port}"),
        serde_json::json!({
            "stage": "decode",
            "method": BrpMethod::WorldQuery.as_str(),
            "port": port,
            "error": error.to_string(),
        }),
    )
    .into()
}

fn name_query_brp_error(
    port: Port,
    code: i32,
    message: String,
    data: Option<Value>,
) -> Report<Error> {
    Error::tool_call_failed_with_details(
        format!("world.query failed on port {port}: {message}"),
        serde_json::json!({
            "stage": "query",
            "method": BrpMethod::WorldQuery.as_str(),
            "port": port,
            "code": code,
            "data": data,
        }),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use bevy::prelude::Name;
    use serde_json::Value;
    use serde_json::json;

    use super::FindEntitiesByNameParams;
    use super::NameMatchMode;
    use super::NamedEntity;
    use super::build_name_query_params;
    use super::name_matches;
    use super::name_query_brp_error;
    use super::parse_name_query_rows;
    use crate::brp_tools::Port;
    use crate::error::Error;
    use crate::tool::BrpMethod;

    const TEST_ASTERISK: &str = "*";
    const TEST_ASTERISK_NAME: &str = "List*";
    const TEST_BRP_ERROR_CODE: i32 = -32_602;
    const TEST_BRP_ERROR_MESSAGE: &str = "Name is not reflected";
    const TEST_CONTAINS_NAME: &str = "tesL";
    const TEST_ENTITY_HIGH: u64 = 42;
    const TEST_ENTITY_LOW: u64 = 7;
    const TEST_ENTITY_OTHER: u64 = 20;
    const TEST_LOWERCASE_NAME: &str = "nateslist";
    const TEST_MISSING_NAME: &str = "Missing";
    const TEST_NAME: &str = "NatesList";
    const TEST_OTHER_NAME: &str = "Other";
    const TEST_PORT: Port = Port(15_702);
    const TEST_PREFIX_NAME: &str = "Nates";
    const TEST_SUFFIX_NAME: &str = "List";

    fn query_rows(rows: &[(u64, &str)]) -> Value {
        let component = type_name::<Name>();
        Value::Array(
            rows.iter()
                .map(|(entity, name)| {
                    json!({
                        "entity": entity,
                        "components": {(component): name},
                    })
                })
                .collect(),
        )
    }

    #[test]
    fn query_composition_uses_standard_brp_without_extras()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        let component = type_name::<Name>();
        let params = build_name_query_params()?;

        assert_eq!(BrpMethod::WorldQuery.as_str(), "world.query");
        assert_eq!(
            params,
            json!({
                "data": {"components": [component]},
                "filter": {"with": [component]},
            })
        );
        assert!(!params.to_string().contains("brp_extras"));
        Ok(())
    }

    #[test]
    fn match_modes_are_typed_and_default_to_exact() -> serde_json::Result<()> {
        let default_params = serde_json::from_value::<FindEntitiesByNameParams>(json!({
            "name": TEST_NAME,
            "port": TEST_PORT,
        }))?;
        assert_eq!(default_params.match_mode, NameMatchMode::Exact);

        for (wire_name, expected) in [
            ("exact", NameMatchMode::Exact),
            ("prefix", NameMatchMode::Prefix),
            ("suffix", NameMatchMode::Suffix),
            ("contains", NameMatchMode::Contains),
        ] {
            let params = serde_json::from_value::<FindEntitiesByNameParams>(json!({
                "name": TEST_SUFFIX_NAME,
                "match_mode": wire_name,
                "port": TEST_PORT,
            }))?;
            assert_eq!(params.match_mode, expected);
        }
        Ok(())
    }

    #[test]
    fn matching_is_case_sensitive_and_asterisks_are_literal() {
        assert!(name_matches(TEST_NAME, TEST_NAME, NameMatchMode::Exact));
        assert!(!name_matches(
            TEST_NAME,
            TEST_LOWERCASE_NAME,
            NameMatchMode::Exact
        ));
        assert!(name_matches(
            TEST_NAME,
            TEST_PREFIX_NAME,
            NameMatchMode::Prefix
        ));
        assert!(name_matches(
            TEST_NAME,
            TEST_SUFFIX_NAME,
            NameMatchMode::Suffix
        ));
        assert!(name_matches(
            TEST_NAME,
            TEST_CONTAINS_NAME,
            NameMatchMode::Contains
        ));
        assert!(name_matches(
            TEST_ASTERISK_NAME,
            TEST_ASTERISK,
            NameMatchMode::Contains
        ));
        assert!(!name_matches(
            TEST_NAME,
            TEST_ASTERISK,
            NameMatchMode::Contains
        ));
    }

    #[test]
    fn rows_are_filtered_and_sorted_by_entity_id()
    -> core::result::Result<(), Box<dyn std::error::Error>> {
        let entities = parse_name_query_rows(
            query_rows(&[
                (TEST_ENTITY_HIGH, TEST_NAME),
                (TEST_ENTITY_LOW, TEST_NAME),
                (TEST_ENTITY_OTHER, TEST_OTHER_NAME),
            ]),
            TEST_NAME,
            NameMatchMode::Exact,
            TEST_PORT,
        )?;

        assert_eq!(
            entities,
            vec![
                NamedEntity {
                    entity: TEST_ENTITY_LOW,
                    name:   TEST_NAME.to_string(),
                },
                NamedEntity {
                    entity: TEST_ENTITY_HIGH,
                    name:   TEST_NAME.to_string(),
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn no_matches_returns_an_empty_result() -> core::result::Result<(), Box<dyn std::error::Error>>
    {
        let entities = parse_name_query_rows(
            query_rows(&[(TEST_ENTITY_LOW, TEST_NAME)]),
            TEST_MISSING_NAME,
            NameMatchMode::Exact,
            TEST_PORT,
        )?;

        assert!(entities.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_rows_return_decode_errors() {
        let component = type_name::<Name>();
        let result = parse_name_query_rows(
            json!([{"entity": TEST_ENTITY_LOW, "components": {(component): 12}}]),
            TEST_NAME,
            NameMatchMode::Exact,
            TEST_PORT,
        );

        assert!(result.is_err());
        if let Err(report) = result {
            assert!(matches!(report.current_context(), Error::ToolCall { .. }));
            let Error::ToolCall { message, details } = report.current_context() else {
                return;
            };
            assert!(message.contains("Unable to decode world.query name response"));
            assert_eq!(
                details.as_ref().and_then(|value| value.get("stage")),
                Some(&json!("decode"))
            );
        }
    }

    #[test]
    fn raw_brp_errors_keep_message_code_and_data() {
        let report = name_query_brp_error(
            TEST_PORT,
            TEST_BRP_ERROR_CODE,
            TEST_BRP_ERROR_MESSAGE.to_string(),
            Some(json!({"component": type_name::<Name>()})),
        );

        assert!(matches!(report.current_context(), Error::ToolCall { .. }));
        let Error::ToolCall { message, details } = report.current_context() else {
            return;
        };
        assert!(message.contains(TEST_BRP_ERROR_MESSAGE));
        assert_eq!(
            details.as_ref().and_then(|value| value.get("code")),
            Some(&json!(TEST_BRP_ERROR_CODE))
        );
        assert_eq!(
            details
                .as_ref()
                .and_then(|value| value.get("data"))
                .and_then(|value| value.get("component")),
            Some(&json!(type_name::<Name>()))
        );
    }
}
