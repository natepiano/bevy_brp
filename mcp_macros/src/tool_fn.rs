use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Error;
use syn::Lit;
use syn::Result;
use syn::parse2;

/// Derive macro for implementing the `ToolFn` trait
///
/// This macro generates the standard `ToolFn` implementation pattern that is
/// repeated across all tools in the codebase. It handles parameter extraction,
/// calling the `handle_impl` function, and wrapping the result.
///
/// # Usage
///
/// ```rust
/// #[derive(ToolFn)]
/// #[tool_fn(params = "MyParams", output = "MyOutput")]
/// pub struct MyTool;
/// ```
///
/// The macro expects:
/// - A `params` attribute specifying the parameter type
/// - An `output` attribute specifying the output type
/// - A `handle_impl` function in scope with signature: `async fn handle_impl(params: MyParams) ->
///   Result<MyOutput>`
pub(crate) fn derive_tool_fn(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;

    // Extract the struct name
    let struct_name = &input.ident;

    // Find the tool_fn attribute to get params and output types
    let tool_fn_attr = input
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("tool_fn"))
        .ok_or_else(|| {
            Error::new_spanned(
                &input,
                "ToolFn derive requires #[tool_fn(params = \"...\", output = \"...\")] attribute",
            )
        })?;

    let mut params_type = None;
    let mut output_type = None;

    // Parse the attribute arguments
    tool_fn_attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("params") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                params_type = Some(s.value());
            }
        } else if meta.path.is_ident("output") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                output_type = Some(s.value());
            }
        }
        Ok(())
    })?;

    let params_type = params_type
        .ok_or_else(|| Error::new_spanned(tool_fn_attr, "Missing 'params' in tool_fn attribute"))?;
    let output_type = output_type
        .ok_or_else(|| Error::new_spanned(tool_fn_attr, "Missing 'output' in tool_fn attribute"))?;

    // Parse the type strings into TokenStreams
    let params_type: TokenStream = params_type
        .parse()
        .map_err(|_| Error::new_spanned(tool_fn_attr, "Invalid params type"))?;
    let output_type: TokenStream = output_type
        .parse()
        .map_err(|_| Error::new_spanned(tool_fn_attr, "Invalid output type"))?;

    let handle_impl_call = quote! { handle_impl(params.clone()).await };

    let expanded = quote! {
        impl ToolFn for #struct_name {
            type Output = #output_type;
            type Params = #params_type;

            fn call(&self, context: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
                Box::pin(async move {
                    let params: Self::Params = crate::tool::extract_parameter_values(&context)?;
                    let result = #handle_impl_call;
                    Ok(ToolResult {
                        result,
                        params: Some(params),
                    })
                })
            }
        }
    };

    Ok(expanded)
}
