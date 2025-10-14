//! `world.mutate_components` tool - Mutate component fields

use crate::brp_tools::Port;
use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Parameters for the `world.mutate_components` tool
#[derive(Clone, Serialize, JsonSchema, ParamStruct)]
pub struct MutateComponentsParams {
    /// The entity ID containing the component to mutate
    pub entity: u64,

    /// The fully-qualified type name of the component to mutate
    pub component: String,

    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value: Value,

    /// The path to the field within the component (e.g., 'translation.x')
    #[serde(default)]
    pub path: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Added to deal with a coding agent malfunction that it could not reliably
/// construct parameters for this tool. Created to provide an improved
/// error message that hopefully allows the agent to correct itself.
impl<'de> Deserialize<'de> for MutateComponentsParams {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Entity,
            Component,
            Value,
            Path,
            Port,
        }

        struct ParamsVisitor;

        impl<'de> Visitor<'de> for ParamsVisitor {
            type Value = MutateComponentsParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct MutateComponentsParams")
            }

            fn visit_map<V>(self, mut map: V) -> core::result::Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut entity = None;
                let mut component = None;
                let mut value = None;
                let mut path = None;
                let mut port = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Entity => {
                            if entity.is_some() {
                                return Err(Error::duplicate_field("entity"));
                            }
                            entity = Some(map.next_value()?);
                        }
                        Field::Component => {
                            if component.is_some() {
                                return Err(Error::duplicate_field("component"));
                            }
                            component = Some(map.next_value()?);
                        }
                        Field::Value => {
                            if value.is_some() {
                                return Err(Error::duplicate_field("value"));
                            }
                            value = Some(map.next_value()?);
                        }
                        Field::Path => {
                            if path.is_some() {
                                return Err(Error::duplicate_field("path"));
                            }
                            path = Some(map.next_value()?);
                        }
                        Field::Port => {
                            if port.is_some() {
                                return Err(Error::duplicate_field("port"));
                            }
                            port = Some(map.next_value()?);
                        }
                    }
                }

                // Collect missing required fields for better error message
                let mut missing = Vec::new();
                if entity.is_none() {
                    missing.push("entity");
                }
                if component.is_none() {
                    missing.push("component");
                }
                if value.is_none() {
                    missing.push("value");
                }

                if !missing.is_empty() {
                    return Err(Error::custom(format!(
                        "Invalid parameter format for 'MutateComponentsParams': missing required \
                         fields: {}. All three parameters are required: entity (u64), component \
                         (string), value (any JSON value). Optional: path (string, defaults to \
                         empty), port (number, defaults to 15702)",
                        missing.join(", ")
                    )));
                }

                Ok(MutateComponentsParams {
                    entity: entity.unwrap(),
                    component: component.unwrap(),
                    value: value.unwrap(),
                    path: path.unwrap_or_default(),
                    port: port.unwrap_or_default(),
                })
            }
        }

        const FIELDS: &[&str] = &["entity", "component", "value", "path", "port"];
        deserializer.deserialize_struct("MutateComponentsParams", FIELDS, ParamsVisitor)
    }
}

/// Result for the `world.mutate_components` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(enhanced_errors = true)]
pub struct MutateComponentsResult {
    /// The raw BRP response data (empty for mutate)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Mutated {component} for entity {entity}")]
    pub message_template: String,
}
