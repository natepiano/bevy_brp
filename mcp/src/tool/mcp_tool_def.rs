use super::parameters::Parameter;
use super::{BrpMethodParamCategory, HandlerType};
use crate::response::ResponseSpecification;

/// Complete definition of a BRP tool
#[derive(Clone)]
pub struct McpToolDef {
    /// Tool name (e.g., "`bevy_destroy`")
    pub name:                &'static str,
    /// Tool description
    pub description:         &'static str,
    /// Handler type (BRP or Local)
    pub handler:             HandlerType,
    /// Parameters for the tool
    pub parameters:          Vec<Parameter>,
    /// Parameter extractor type
    pub parameter_extractor: BrpMethodParamCategory,
    /// Response formatter definition
    pub formatter:           ResponseSpecification,
}

impl McpToolDef {
    /// Generate tool registration from this declarative definition
    pub fn to_tool(&self) -> rmcp::model::Tool {
        use super::parameters::ParamType;
        use crate::support::schema;

        let mut builder = schema::SchemaBuilder::new();

        // Add all parameters to the schema
        for param in &self.parameters {
            builder = match param.param_type {
                ParamType::Number => {
                    builder.add_number_property(param.name, param.description, param.required)
                }
                ParamType::String => {
                    builder.add_string_property(param.name, param.description, param.required)
                }
                ParamType::Boolean => {
                    builder.add_boolean_property(param.name, param.description, param.required)
                }
                ParamType::StringArray => {
                    builder.add_string_array_property(param.name, param.description, param.required)
                }
                ParamType::Any => {
                    builder.add_any_property(param.name, param.description, param.required)
                }
            };
        }

        rmcp::model::Tool {
            name:         self.name.into(),
            description:  self.description.into(),
            input_schema: builder.build(),
        }
    }
}
