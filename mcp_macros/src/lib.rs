//! Procedural macros for bevy_brp_mcp

mod brp_tools;
mod field_placement;
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

/// Generates BRP tool implementations and constants from enum variants with `#[tool(...)]`
/// attributes.
///
/// # Example
///
/// ```ignore
/// #[derive(BrpTools)]
/// pub enum ToolName {
///     #[tool(brp_method = "bevy/destroy", params = "DestroyParams")]
///     BevyDestroy,
///
///     #[tool(brp_method = "bevy/get+watch")]
///     BevyGetWatch,  // Just the method, no params
/// }
/// ```
///
/// This will generate:
/// - Tool struct implementations for variants with params
/// - BRP method constants for all variants with brp_method
/// - All necessary trait implementations
/// - A `brp_method()` function on the enum
#[proc_macro_derive(BrpTools, attributes(tool))]
pub fn derive_brp_tools(input: TokenStream) -> TokenStream {
    brp_tools::derive_brp_tools_impl(input)
}

/// Derives field placement traits for parameter and response structs.
///
/// # Example for Parameters (fields remain public)
///
/// ```ignore
/// #[derive(ResultFieldPlacement)]
/// struct GetParams {
///     #[to_metadata]
///     pub entity: u64,
///
///     #[to_call_info]
///     pub port: u16,
/// }
/// ```
///
/// # Example for Results (fields become private with `#[to_message]`)
///
/// ```ignore
/// #[derive(ResultFieldPlacement)]
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
/// // Result structs with #[to_message] can ONLY be constructed via:
/// let result = GetResult::new(Some(value), 5);
/// // Or with custom template:
/// let result = GetResult::new(Some(value), 5)
///     .with_message_template("Custom: {count}");
/// ```
///
/// This will generate implementations for:
/// - `HasFieldPlacement` - provides field placement information
/// - `ResponseData` - for building MCP responses
/// - `CallInfoProvider` - if there are `#[to_call_info]` fields
/// - `MessageTemplateProvider` - if there's a `#[to_message]` field
/// - `::new()` constructor - if there's a `#[to_message]` field (makes fields private)
#[proc_macro_derive(
    ResultFieldPlacement,
    attributes(to_metadata, to_result, to_call_info, to_message, computed)
)]
pub fn derive_field_placement(input: TokenStream) -> TokenStream {
    field_placement::derive_field_placement_impl(input)
}
