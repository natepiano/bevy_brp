use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Lit, Result, parse2};

/// Derive macro for implementing the ToolFn trait
///
/// This macro generates the standard ToolFn implementation pattern that is
/// repeated across all tools in the codebase. It handles parameter extraction,
/// calling the handle_impl function, and wrapping the result.
///
/// # Usage
///
/// ```rust
/// #[derive(ToolFn)]
/// #[tool_fn(params = "MyParams", output = "MyOutput")]
/// pub struct MyTool;
/// ```
///
/// Or with context passing:
/// ```rust
/// #[derive(ToolFn)]
/// #[tool_fn(params = "MyParams", output = "MyOutput", with_context)]
/// pub struct MyTool;
/// ```
///
/// The macro expects:
/// - A `params` attribute specifying the parameter type
/// - An `output` attribute specifying the output type
/// - An optional `with_context` flag to pass HandlerContext to handle_impl
/// - A `handle_impl` function in scope with signature:
///   - Without context: `async fn handle_impl(params: MyParams) -> Result<MyOutput>`
///   - With context: `async fn handle_impl(ctx: HandlerContext, params: MyParams) ->
///     Result<MyOutput>`
pub fn derive_tool_fn(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;

    // Extract the struct name
    let struct_name = &input.ident;

    // Find the tool_fn attribute to get params and output types
    let tool_fn_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("tool_fn"))
        .ok_or_else(|| {
            Error::new_spanned(
                &input,
                "ToolFn derive requires #[tool_fn(params = \"...\", output = \"...\")] attribute",
            )
        })?;

    let mut params_type = None;
    let mut output_type = None;
    let mut with_context = false;

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
        } else if meta.path.is_ident("with_context") {
            with_context = true;
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

    // Generate the implementation based on whether context is needed
    let handle_impl_call = if with_context {
        quote! { handle_impl(ctx.clone(), params.clone()).await }
    } else {
        quote! { handle_impl(params.clone()).await }
    };

    let expanded = quote! {
        impl ToolFn for #struct_name {
            type Output = #output_type;
            type Params = #params_type;

            fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
                Box::pin(async move {
                    let params: Self::Params = ctx.extract_parameter_values()?;
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
