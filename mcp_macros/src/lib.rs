//! Procedural macros for bevy_brp_mcp

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

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
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the path from the attribute
    let path = extract_path(&input.attrs);

    // Ensure we're working with an enum
    let Data::Enum(data_enum) = &input.data else {
        panic!("ToolDescription can only be derived for enums");
    };

    // Generate match arms for each variant
    let match_arms = data_enum.variants.iter().map(|variant| {
        // Ensure the variant has no fields
        if !matches!(variant.fields, Fields::Unit) {
            panic!("ToolDescription can only be derived for enums with unit variants");
        }

        let variant_name = &variant.ident;
        let snake_case_name = variant_name.to_string().to_snake_case();
        let file_path = format!("{path}/{snake_case_name}.txt");

        quote! {
            Self::#variant_name => include_str!(#file_path)
        }
    });

    let enum_name = &input.ident;

    // Generate the implementation
    let expanded = quote! {
        impl #enum_name {
            /// Returns the description text for this tool.
            pub const fn description(&self) -> &'static str {
                match self {
                    #(#match_arms,)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Extract the path from tool_description attributes
fn extract_path(attrs: &[syn::Attribute]) -> String {
    for attr in attrs {
        if attr.path().is_ident("tool_description") {
            let mut path = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("path") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    path = Some(s.value());
                    Ok(())
                } else {
                    Err(meta.error("unsupported tool_description attribute"))
                }
            })
            .expect("failed to parse tool_description attribute");

            if let Some(path) = path {
                return path;
            }
        }
    }

    panic!("tool_description attribute with path is required");
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
    let input = parse_macro_input!(input as DeriveInput);

    // Ensure we're working with an enum
    let Data::Enum(data_enum) = &input.data else {
        panic!("BrpTools can only be derived for enums");
    };

    let mut tool_impls = Vec::new();

    // Process each variant
    for variant in &data_enum.variants {
        // Ensure the variant has no fields
        if !matches!(variant.fields, Fields::Unit) {
            panic!("BrpTools can only be derived for enums with unit variants");
        }

        let variant_name = &variant.ident;

        // Extract brp_method and brp_tool attributes
        let method = extract_brp_method_attr(&variant.attrs);
        let tool_params = extract_brp_tool_attr(&variant.attrs);

        // Validate: brp_tool requires brp_method
        if tool_params.is_some() && method.is_none() {
            panic!("Variant {variant_name} has #[brp_tool] but no #[brp_method]");
        }

        // No longer generate constants - we use the BrpMethod enum instead

        // Generate tool implementation only if brp_tool is present
        if let Some(params) = tool_params {
            let method = method.expect("already validated");
            let params_ident = syn::Ident::new(&params, variant_name.span());

            tool_impls.push(quote! {
                pub struct #variant_name;

                impl crate::tool::ToolFn for #variant_name {
                    type Output = crate::brp_tools::handler::BrpMethodResult;
                    type CallInfoData = crate::response::BrpCallInfo;

                    fn call(
                        &self,
                        ctx: &crate::tool::HandlerContext,
                    ) -> crate::tool::HandlerResponse<(Self::CallInfoData, Self::Output)> {
                        let ctx_clone = ctx.clone();
                        Box::pin(async move {
                            let params = ctx_clone.extract_parameter_values::<crate::tool::brp_parameters::#params_ident>()?;
                            let port = <crate::tool::brp_parameters::#params_ident as crate::brp_tools::handler::HasPortField>::port(&params);
                            let result = crate::brp_tools::handler::execute_static_brp_call::<
                                #variant_name,
                                crate::tool::brp_parameters::#params_ident,
                            >(&ctx_clone)
                            .await?;

                            Ok((
                                crate::response::BrpCallInfo {
                                    method: <#variant_name as crate::brp_tools::handler::HasBrpMethod>::brp_method(),
                                    port,
                                },
                                result,
                            ))
                        })
                    }
                }

                impl crate::brp_tools::handler::HasBrpMethod for #variant_name {
                    fn brp_method() -> &'static str {
                        #method
                    }
                }

                impl crate::brp_tools::handler::HasPortField for crate::tool::brp_parameters::#params_ident {
                    fn port(&self) -> u16 {
                        self.port
                    }
                }
            });
        }
    }

    // Generate match arms for the enum's brp_method() function
    let mut method_match_arms = Vec::new();
    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        if let Some(method) = extract_brp_method_attr(&variant.attrs) {
            method_match_arms.push(quote! {
                Self::#variant_name => Some(#method)
            });
        } else {
            method_match_arms.push(quote! {
                Self::#variant_name => None
            });
        }
    }

    let enum_name = &input.ident;

    // Generate BrpMethod enum variants only for those with brp_method attribute
    let mut brp_method_variants = Vec::new();
    let mut to_brp_method_arms = Vec::new();
    let mut from_brp_method_arms = Vec::new();
    let mut brp_method_string_arms = Vec::new();

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        if let Some(method) = extract_brp_method_attr(&variant.attrs) {
            brp_method_variants.push(quote! {
                #variant_name
            });

            to_brp_method_arms.push(quote! {
                Self::#variant_name => Some(BrpMethod::#variant_name)
            });

            from_brp_method_arms.push(quote! {
                BrpMethod::#variant_name => Self::#variant_name
            });

            brp_method_string_arms.push(quote! {
                BrpMethod::#variant_name => #method
            });
        } else {
            to_brp_method_arms.push(quote! {
                Self::#variant_name => None
            });
        }
    }

    // Generate the complete output
    let expanded = quote! {
        // Tool implementations
        #(#tool_impls)*

        // Add brp_method() function to the enum
        impl #enum_name {
            /// Returns the BRP method string for this tool variant, if it has one.
            pub const fn brp_method(&self) -> Option<&'static str> {
                match self {
                    #(#method_match_arms,)*
                }
            }

            /// Converts to BrpMethod if this variant has a BRP method
            pub const fn to_brp_method(&self) -> Option<BrpMethod> {
                match self {
                    #(#to_brp_method_arms,)*
                }
            }
        }

        /// Enum containing only tool variants that have BRP methods
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum BrpMethod {
            #(#brp_method_variants,)*
        }

        impl BrpMethod {
            /// Returns the BRP method string (infallible)
            pub const fn as_str(&self) -> &'static str {
                match self {
                    #(#brp_method_string_arms,)*
                }
            }
        }

        impl From<BrpMethod> for #enum_name {
            fn from(brp_method: BrpMethod) -> Self {
                match brp_method {
                    #(#from_brp_method_arms,)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Extract method from brp_method attribute
fn extract_brp_method_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("brp_method") {
            // Parse the method string directly from the attribute
            if let Ok(method) = attr.parse_args::<syn::LitStr>() {
                return Some(method.value());
            }
        }
    }
    None
}

/// Extract params from brp_tool attribute
fn extract_brp_tool_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("brp_tool") {
            let mut params = None;

            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("params") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    params = Some(s.value());
                } else {
                    return Err(meta.error("unsupported brp_tool attribute"));
                }
                Ok(())
            });

            return params;
        }
    }
    None
}
