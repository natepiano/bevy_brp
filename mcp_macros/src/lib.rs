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

/// Generates BRP tool implementations and constants from enum variants with `#[brp_method]`
/// and `#[brp_tool]` attributes.
///
/// # Example
///
/// ```ignore
/// #[derive(BrpTools)]
/// pub enum ToolName {
///     #[brp_method("bevy/destroy")]
///     #[brp_tool(params = "DestroyParams")]
///     BevyDestroy,
///
///     #[brp_method("bevy/get+watch")]
///     BevyGetWatch,  // No brp_tool, just the method
/// }
/// ```
///
/// This will generate:
/// - Tool struct implementations for variants with `#[brp_tool]`
/// - BRP method constants for all variants with `#[brp_method]`
/// - All necessary trait implementations
/// - A `brp_method()` function on the enum
#[proc_macro_derive(BrpTools, attributes(brp_method, brp_tool))]
pub fn derive_brp_tools(input: TokenStream) -> TokenStream {
    brp_tools::derive_brp_tools_impl(input)
}

/// Derives field placement traits for parameter and response structs.
///
/// # Example
///
/// ```ignore
/// #[derive(FieldPlacement)]
/// struct GetParams {
///     #[to_metadata]
///     pub entity: u64,
///
///     #[to_result]
///     pub components: Value,
///
///     #[to_call_info]
///     pub port: u16,
/// }
/// ```
///
/// This will generate implementations for:
/// - `HasFieldPlacement` - provides field placement information
/// - `CallInfoProvider` - if there are `#[to_call_info]` fields
#[proc_macro_derive(FieldPlacement, attributes(to_metadata, to_result, to_call_info))]
pub fn derive_field_placement(input: TokenStream) -> TokenStream {
    field_placement::derive_field_placement_impl(input)
}
