//! Tool definitions for BRP and local MCP tools.
//!
//! This is a new version that uses the BrpToolDef and LocalToolDef structures.

use std::sync::Arc;

use super::brp_tool_def::{BrpMethodSource, BrpToolDef};
use super::constants::{
    BRP_METHOD_DESTROY, BRP_METHOD_GET, BRP_METHOD_QUERY, DESC_BEVY_DESTROY, DESC_BEVY_GET, DESC_BEVY_QUERY, DESC_LAUNCH_BEVY_EXAMPLE,
    TOOL_BEVY_DESTROY, TOOL_BEVY_GET, TOOL_BEVY_QUERY, TOOL_LAUNCH_BEVY_EXAMPLE,
};
use super::local_tool_def::LocalToolDef;
use super::parameters::{BrpParameter, BrpParameterName, LocalParameter, LocalParameterName};
use super::tool_definition::{PortParameter, ToolDefinition};
use crate::app_tools::brp_launch_bevy_example;
use crate::constants::{
    JSON_FIELD_COMPONENT_COUNT, JSON_FIELD_COMPONENTS, JSON_FIELD_ENTITY, JSON_FIELD_ENTITY_COUNT,
};
use crate::response::{
    FieldPlacement, ResponseExtractorType, ResponseField, ResponseSpecification,
};

/// Get all tool definitions for registration with the MCP service
pub fn get_all_tool_definitions() -> Vec<Box<dyn ToolDefinition>> {
    vec![
        // BrpToolDef/bevy_destroy
        Box::new(BrpToolDef {
            name:          TOOL_BEVY_DESTROY,
            description:   DESC_BEVY_DESTROY,
            method_source: BrpMethodSource::Static(BRP_METHOD_DESTROY),
            parameters:    vec![
                BrpParameter::entity("The entity ID to destroy", true),
            ],
            formatter:     ResponseSpecification {
                message_template: "Successfully destroyed entity {entity}",
                response_fields:  vec![ResponseField::FromRequest {
                    response_field_name:  JSON_FIELD_ENTITY,
                    parameter_field_name: JSON_FIELD_ENTITY,
                    placement:            FieldPlacement::Metadata,
                }],
            },
        }),
        
        // BrpToolDef/bevy_get
        Box::new(BrpToolDef {
            name:          TOOL_BEVY_GET,
            description:   DESC_BEVY_GET,
            method_source: BrpMethodSource::Static(BRP_METHOD_GET),
            parameters:    vec![
                BrpParameter::entity("The entity ID to get component data from", true),
                BrpParameter::components(
                    "Array of component types to retrieve. Each component must be a fully-qualified type name",
                    true,
                ),
            ],
            formatter:     ResponseSpecification {
                message_template: "Retrieved component data from entity {entity}",
                response_fields:  vec![
                    ResponseField::FromRequest {
                        response_field_name:  JSON_FIELD_ENTITY,
                        parameter_field_name: JSON_FIELD_ENTITY,
                        placement:            FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COMPONENTS,
                        response_extractor:  ResponseExtractorType::Field(JSON_FIELD_COMPONENTS),
                        placement:           FieldPlacement::Result,
                    },
                ],
            },
        }),
        // BrpToolDef/bevy_query
        Box::new(BrpToolDef {
            name:          TOOL_BEVY_QUERY,
            description:   DESC_BEVY_QUERY,
            method_source: BrpMethodSource::Static(BRP_METHOD_QUERY),
            parameters:    vec![
                BrpParameter::any(
                    BrpParameterName::Data,
                    "Object specifying what component data to retrieve. Properties: components (array), option (array), has (array)",
                    true,
                ),
                BrpParameter::any(
                    BrpParameterName::Filter,
                    "Object specifying which entities to query. Properties: with (array), without (array)",
                    true,
                ),
                BrpParameter::strict(),
            ],
            formatter:     ResponseSpecification {
                message_template: "Query completed successfully",
                response_fields:  vec![
                    ResponseField::DirectToResult,
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_ENTITY_COUNT,
                        response_extractor:  ResponseExtractorType::ItemCount,
                        placement:           FieldPlacement::Metadata,
                    },
                    ResponseField::FromResponse {
                        response_field_name: JSON_FIELD_COMPONENT_COUNT,
                        response_extractor:  ResponseExtractorType::QueryComponentCount,
                        placement:           FieldPlacement::Metadata,
                    },
                ],
            },
        }),
        // LocalToolDef/launch_bevy_example
        Box::new(LocalToolDef {
            name:           TOOL_LAUNCH_BEVY_EXAMPLE,
            description:    DESC_LAUNCH_BEVY_EXAMPLE,
            handler:        Arc::new(brp_launch_bevy_example::create_launch_bevy_example_handler()),
            parameters:     vec![
                LocalParameter::string(
                    LocalParameterName::ExampleName,
                    "Name of the Bevy example to launch",
                    true,
                ),
                LocalParameter::string(
                    LocalParameterName::Profile,
                    "Build profile to use (debug or release)",
                    false,
                ),
                LocalParameter::string(
                    LocalParameterName::Path,
                    "Path to use when multiple examples with the same name exist",
                    false,
                ),
            ],
            port_parameter: PortParameter::Required,
            formatter:      ResponseSpecification {
                message_template: "Launched Bevy example `{example_name}`",
                response_fields:  vec![ResponseField::DirectToMetadata],
            },
        }),
    ]
}
