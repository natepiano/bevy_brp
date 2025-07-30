//! Procedural macros for bevy_brp_mcp

mod brp_tools;
mod param_struct;
mod result_struct;
mod shared;
mod tool_description;

use proc_macro::TokenStream;

/// Derives a `description()` method for tool enums that loads help text from files.
///
/// # Example
///
/// ```ignore
/// #[derive(ToolDescription)]
/// #[tool_description(path = "../../help_text")]
/// pub enum ToolName {
///     BevyList,
///     BevyGet,
/// }
/// ```
///
/// This will generate:
///
/// ```ignore
/// impl ToolName {
///     pub const fn description(&self) -> &'static str {
///         match self {
///             ToolName::BevyList => include_str!("../../help_text/bevy_list.txt"),
///             ToolName::BevyGet => include_str!("../../help_text/bevy_get.txt"),
///         }
///     }
/// }
/// ```
#[proc_macro_derive(ToolDescription, attributes(tool_description))]
pub fn derive_tool_description(input: TokenStream) -> TokenStream {
    tool_description::derive_tool_description_impl(input)
}

/// Generates BRP tool implementations and constants from enum variants with `#[brp_tool(...)]`
/// attributes.
///
/// # Example
///
/// ```ignore
/// #[derive(BrpTools)]
/// pub enum ToolName {
///     #[brp_tool(brp_method = "bevy/destroy", params = "DestroyParams")]
///     BevyDestroy,
///
///     #[brp_tool(brp_method = "bevy/get+watch")]
///     BevyGetWatch,  // Just the method, no params
/// }
/// ```
///
/// This will generate:
/// - Tool struct implementations for variants with params
/// - BRP method constants for all variants with brp_method
/// - All necessary trait implementations
/// - A `brp_method()` function on the enum
#[proc_macro_derive(BrpTools, attributes(brp_tool))]
pub fn derive_brp_tools(input: TokenStream) -> TokenStream {
    brp_tools::derive_brp_tools_impl(input)
}

/// Derives field placement traits for parameter structs.
///
/// Parameter structs are deserialized from JSON and have public fields.
/// They cannot have `#[to_message]` attributes.
///
/// # Example
///
/// ```ignore
/// #[derive(ParamStruct)]
/// struct GetParams {
///     pub entity: u64,
///
///     #[to_call_info]
///     pub port: Port,
/// }
/// ```
///
/// This will generate implementations for:
/// - `HasFieldPlacement` - provides field placement information
/// - `ResponseData` - for building MCP responses
#[proc_macro_derive(ParamStruct, attributes(to_metadata, to_call_info))]
pub fn derive_param_struct(input: TokenStream) -> TokenStream {
    param_struct::derive_param_struct_impl(input)
}

/// Derives field placement traits for result structs.
///
/// Result structs have private fields and require a `#[to_message]` attribute.
/// They can only be constructed via the generated `::new()` method.
///
/// # Message Template Patterns
///
/// The behavior depends on the field type and whether a default template is provided:
///
/// - **`String` with default template**: Direct construction, optional override
///   - `#[to_message(message_template = "Found {count} items")]`
///   - Usage: `MyResult::new(...)` or `MyResult::new(...).with_message_template("custom")`
///
/// - **`String` without default**: Compile error - not allowed
///
/// - **`Option<String>` with default**: Direct construction like String
///   - `#[to_message(message_template = "Default message")]`
///   - Usage: `MyResult::new(...)` or `MyResult::new(...).with_message_template("custom")`
///
/// - **`Option<String>` without default**: Builder pattern required
///   - `#[to_message]`
///   - Usage: `MyResult::new(...).with_message_template("required message")`
///
/// # Example
///
/// ```ignore
/// #[derive(ResultStruct)]
/// struct GetResult {
///     #[to_result]
///     result: Option<Value>,  // Private field!
///
///     #[to_metadata]
///     count: usize,           // Private field!
///
///     #[to_message(message_template = "Found {count} items")]
///     message_template: String,  // Private field!
/// }
///
/// // Result structs can ONLY be constructed via:
/// let result = GetResult::new(Some(value), 5);
/// // Or with custom template:
/// let result = GetResult::new(Some(value), 5)
///     .with_message_template("Custom: {count}");
/// ```
///
/// This will generate implementations for:
/// - `HasFieldPlacement` - provides field placement information
/// - `ResponseData` - for building MCP responses
/// - `MessageTemplateProvider` - for message template handling
/// - `::new()` constructor and `::from_brp_client_response()` method
#[proc_macro_derive(
    ResultStruct,
    attributes(
        to_metadata,
        to_result,
        to_message,
        to_error_info,
        computed,
        brp_result
    )
)]
pub fn derive_result_struct(input: TokenStream) -> TokenStream {
    result_struct::derive_result_struct_impl(input)
}
