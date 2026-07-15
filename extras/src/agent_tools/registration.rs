use bevy::prelude::*;
use schemars::JsonSchema;
use schemars::Schema;

/// Owned agent-facing metadata for an existing BRP method.
///
/// An `AgentTool` publishes documentation only. It does not register the backing BRP handler and
/// never creates a native MCP tool. Register the method separately through
/// [`RemoteMethods`](bevy_remote::RemoteMethods), then pass this value to
/// [`AppAgentToolExt::register_agent_tool`].
///
/// `name` is the stable identifier shown to an agent, while `method` is the exact BRP method name
/// to pass to `brp_execute`. `description` explains the method's purpose. Parameter and result
/// schemas describe the raw JSON-RPC values accepted and returned by that backing method without
/// an MCP arguments wrapper or a `{ "result": ... }` wrapper.
#[must_use]
pub struct AgentTool {
    name:          String,
    method:        String,
    description:   String,
    params_schema: Option<Schema>,
    result_schema: Option<Schema>,
}

impl AgentTool {
    const MAX_NAME_LENGTH: usize = 128;

    /// Creates agent-facing metadata for an existing BRP method.
    ///
    /// The supplied values are owned by the returned `AgentTool`. Validation occurs when the tool
    /// is passed to [`AppAgentToolExt::register_agent_tool`].
    pub fn new(
        name: impl Into<String>,
        method: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name:          name.into(),
            method:        method.into(),
            description:   description.into(),
            params_schema: None,
            result_schema: None,
        }
    }

    /// Documents the raw JSON-RPC `params` value forwarded to the backing BRP method.
    ///
    /// The schema is stored unchanged. It is not wrapped in an MCP arguments object. Without this
    /// call, the tool is documented as parameterless and agents should omit `params`.
    #[must_use = "schema setters return the updated agent tool"]
    pub fn params_schema(mut self, schema: Schema) -> Self {
        self.params_schema = Some(schema);
        self
    }

    /// Generates a schema for the raw JSON-RPC `params` value during application construction.
    ///
    /// `T` supplies metadata only. It does not decode requests and has no type relationship with
    /// the separately registered BRP handler. The generated schema is not wrapped in an MCP
    /// arguments object.
    #[must_use = "schema setters return the updated agent tool"]
    pub fn params_schema_for<T: JsonSchema>(self) -> Self {
        self.params_schema(schemars::schema_for!(T))
    }

    /// Documents the raw BRP JSON-RPC `result` value returned by the backing method.
    ///
    /// The schema is stored unchanged and is not wrapped in `{ "result": ... }`. Without this
    /// call, the backing method's result remains undocumented.
    #[must_use = "schema setters return the updated agent tool"]
    pub fn result_schema(mut self, schema: Schema) -> Self {
        self.result_schema = Some(schema);
        self
    }

    /// Generates a schema for the raw BRP JSON-RPC `result` value during application construction.
    ///
    /// `T` supplies metadata only. It does not encode responses and has no type relationship with
    /// the separately registered BRP handler. The generated schema is not wrapped in
    /// `{ "result": ... }`.
    #[must_use = "schema setters return the updated agent tool"]
    pub fn result_schema_for<T: JsonSchema>(self) -> Self {
        self.result_schema(schemars::schema_for!(T))
    }

    fn validate(&self) {
        assert!(
            Self::is_valid_name(&self.name),
            "agent tool `{}` rejected: field `name` must contain 1 to {} ASCII letters, digits, \
             periods, underscores, or hyphens",
            self.name,
            Self::MAX_NAME_LENGTH,
        );
        assert!(
            !self.method.trim().is_empty(),
            "agent tool `{}` rejected: field `method` must not be empty after trimming",
            self.name,
        );
        assert!(
            !self.description.trim().is_empty(),
            "agent tool `{}` rejected: field `description` must not be empty after trimming",
            self.name,
        );
    }

    fn is_valid_name(name: &str) -> bool {
        (1..=Self::MAX_NAME_LENGTH).contains(&name.len())
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    }
}

#[derive(Default, Resource)]
struct RegisteredAgentTools(Vec<AgentTool>);

/// Extends [`App`] with downstream-facing agent metadata publication.
///
/// This trait is a downstream extension point for application construction. Registering an
/// [`AgentTool`] publishes documentation for an existing BRP method; it does not register the BRP
/// handler itself. Add the backing method separately through
/// [`RemoteMethods`](bevy_remote::RemoteMethods).
pub trait AppAgentToolExt {
    /// Publishes agent-facing metadata immediately during application construction.
    ///
    /// Registration can occur before or after `BrpExtrasPlugin` is added. Registering metadata
    /// after [`App::run`] begins is unsupported.
    ///
    /// # Panics
    ///
    /// Panics if the agent name is not 1 to 128 characters drawn from ASCII letters, digits,
    /// periods, underscores, and hyphens; if the method or description is empty after trimming;
    /// or if the agent name was already registered. Panic messages include the rejected name,
    /// field, and reason.
    fn register_agent_tool(&mut self, agent_tool: AgentTool) -> &mut Self;
}

impl AppAgentToolExt for App {
    fn register_agent_tool(&mut self, agent_tool: AgentTool) -> &mut Self {
        self.init_resource::<RegisteredAgentTools>();
        agent_tool.validate();

        {
            let mut registered_agent_tools =
                self.world_mut().resource_mut::<RegisteredAgentTools>();
            assert!(
                registered_agent_tools
                    .0
                    .iter()
                    .all(|registered| registered.name != agent_tool.name),
                "agent tool `{}` rejected: field `name` must be unique; duplicate agent name",
                agent_tool.name,
            );
            registered_agent_tools.0.push(agent_tool);
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::panic::catch_unwind;

    use schemars::json_schema;

    use super::*;
    use crate::BrpExtrasPlugin;

    const DESCRIPTION: &str = "Runs a test operation.";
    const METHOD: &str = "test/operation";

    #[test]
    fn new_owns_string_inputs() {
        let name = String::from("test.owned");
        let method = String::from(METHOD);
        let description = String::from(DESCRIPTION);

        let agent_tool = AgentTool::new(name.clone(), method.clone(), description.clone());

        assert_eq!(agent_tool.name, name);
        assert_eq!(agent_tool.method, method);
        assert_eq!(agent_tool.description, description);
    }

    #[test]
    fn raw_schemas_are_stored_without_wrappers() {
        let params_schema = json_schema!({
            "type": "array",
            "items": { "type": "integer" }
        });
        let result_schema = json_schema!({ "type": "boolean" });

        let agent_tool = AgentTool::new("test.raw", METHOD, DESCRIPTION)
            .params_schema(params_schema.clone())
            .result_schema(result_schema.clone());

        assert_eq!(agent_tool.params_schema, Some(params_schema));
        assert_eq!(agent_tool.result_schema, Some(result_schema));
    }

    #[test]
    fn generated_schemas_are_stored_without_wrappers() {
        let agent_tool = AgentTool::new("test.generated", METHOD, DESCRIPTION)
            .params_schema_for::<BTreeMap<String, i64>>()
            .result_schema_for::<Vec<String>>();

        assert_eq!(
            agent_tool.params_schema,
            Some(schemars::schema_for!(BTreeMap<String, i64>)),
        );
        assert_eq!(
            agent_tool.result_schema,
            Some(schemars::schema_for!(Vec<String>)),
        );
    }

    #[test]
    fn schemas_are_omitted_by_default() {
        let agent_tool = AgentTool::new("test.omitted", METHOD, DESCRIPTION);

        assert!(agent_tool.params_schema.is_none());
        assert!(agent_tool.result_schema.is_none());
    }

    #[test]
    fn name_length_boundaries_are_enforced() {
        assert!(!registration_panics("a"));
        assert!(!registration_panics(
            &"a".repeat(AgentTool::MAX_NAME_LENGTH)
        ));
        assert!(registration_panics(""));
        assert!(registration_panics(
            &"a".repeat(AgentTool::MAX_NAME_LENGTH + 1)
        ));
    }

    #[test]
    fn name_accepts_every_allowed_character_class() {
        assert!(!registration_panics("AZaz09._-"));
    }

    #[test]
    fn name_rejects_invalid_characters() {
        for name in ["test/tool", "test tool", "test:tool", "tést"] {
            assert!(registration_panics(name));
        }
    }

    #[test]
    #[should_panic(expected = "agent tool `bad/name` rejected: field `name`")]
    fn invalid_name_panic_identifies_the_name_and_field() {
        App::new().register_agent_tool(AgentTool::new("bad/name", METHOD, DESCRIPTION));
    }

    #[test]
    #[should_panic(
        expected = "agent tool `empty-method` rejected: field `method` must not be empty after trimming"
    )]
    fn empty_method_is_rejected() {
        App::new().register_agent_tool(AgentTool::new("empty-method", " \t ", DESCRIPTION));
    }

    #[test]
    #[should_panic(
        expected = "agent tool `empty-description` rejected: field `description` must not be empty after trimming"
    )]
    fn empty_description_is_rejected() {
        App::new().register_agent_tool(AgentTool::new("empty-description", METHOD, "\n "));
    }

    #[test]
    #[should_panic(
        expected = "agent tool `duplicate` rejected: field `name` must be unique; duplicate agent name"
    )]
    fn duplicate_name_panic_identifies_the_name_field_and_reason() {
        let mut app = App::new();
        app.register_agent_tool(AgentTool::new("duplicate", "test/first", DESCRIPTION));
        app.register_agent_tool(AgentTool::new("duplicate", "test/second", DESCRIPTION));
    }

    #[test]
    fn duplicate_backing_methods_are_allowed() {
        let mut app = App::new();
        app.register_agent_tool(AgentTool::new("test.first", METHOD, DESCRIPTION));
        app.register_agent_tool(AgentTool::new("test.second", METHOD, DESCRIPTION));

        let registered_agent_tools = app.world().resource::<RegisteredAgentTools>();
        assert_eq!(registered_agent_tools.0.len(), 2);
    }

    #[test]
    fn registration_before_plugin_addition_is_retained() {
        let mut app = App::new();
        app.register_agent_tool(AgentTool::new("test.before", METHOD, DESCRIPTION));
        app.add_plugins(BrpExtrasPlugin);

        let registered_agent_tools = app.world().resource::<RegisteredAgentTools>();
        assert_eq!(registered_agent_tools.0.len(), 1);
        assert_eq!(registered_agent_tools.0[0].name, "test.before");
    }

    #[test]
    fn registration_after_plugin_addition_initializes_the_resource() {
        let mut app = App::new();
        app.add_plugins(BrpExtrasPlugin);
        app.register_agent_tool(AgentTool::new("test.after", METHOD, DESCRIPTION));

        let registered_agent_tools = app.world().resource::<RegisteredAgentTools>();
        assert_eq!(registered_agent_tools.0.len(), 1);
        assert_eq!(registered_agent_tools.0[0].name, "test.after");
    }

    fn registration_panics(name: &str) -> bool {
        let agent_tool = AgentTool::new(name, METHOD, DESCRIPTION);
        catch_unwind(|| {
            App::new().register_agent_tool(agent_tool);
        })
        .is_err()
    }
}
