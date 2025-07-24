//! BrpTools derive macro implementation

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Implementation of the BrpTools derive macro
pub fn derive_brp_tools_impl(input: TokenStream) -> TokenStream {
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
        let (tool_params, tool_result) = extract_brp_tool_attr(&variant.attrs);

        // Validate: brp_tool requires brp_method
        if tool_params.is_some() && method.is_none() {
            panic!("Variant {variant_name} has #[brp_tool] but no #[brp_method]");
        }

        // Generate tool implementation only if brp_tool is present
        if let Some(params) = tool_params {
            let _method = method.expect("already validated");
            let params_ident = syn::Ident::new(&params, variant_name.span());

            // Use specific result type if provided, otherwise use BrpMethodResult
            let result_type = if let Some(result) = &tool_result {
                let result_ident = syn::Ident::new(result, variant_name.span());
                quote! { #result_ident }
            } else {
                quote! { crate::brp_tools::handler::BrpMethodResult }
            };

            // Check if this method supports format discovery
            let supports_format_discovery = matches!(
                variant_name.to_string().as_str(),
                "BevySpawn"
                    | "BevyInsert"
                    | "BevyMutateComponent"
                    | "BevyInsertResource"
                    | "BevyMutateResource"
            );

            // Generate the conversion based on whether format discovery is supported
            let conversion = if tool_result.is_some() {
                if supports_format_discovery {
                    quote! {
                        let result = #result_type::from_brp_value(
                            brp_result.result,
                            brp_result.format_corrections,
                            brp_result.format_corrected,
                        )?;
                    }
                } else {
                    quote! {
                        let result = #result_type::from_brp_value(
                            brp_result.result,
                        )?;
                    }
                }
            } else {
                quote! {
                    let result = brp_result;
                }
            };

            tool_impls.push(quote! {
                pub struct #variant_name;

                impl crate::tool::ToolFn for #variant_name {
                    type Output = #result_type;
                    type CallInfoData = #params_ident;

                    fn call(
                        &self,
                        ctx: &crate::tool::HandlerContext,
                    ) -> crate::tool::HandlerResponse<(Self::CallInfoData, Self::Output)> {
                        let ctx_clone = ctx.clone();
                        Box::pin(async move {
                            let params = ctx_clone.extract_parameter_values::<#params_ident>()?;
                            let brp_result = crate::brp_tools::handler::execute_static_brp_call::<
                                #variant_name,
                                #params_ident,
                            >(&ctx_clone)
                            .await?;
                            // Convert BrpMethodResult to specific result type
                            #conversion

                            Ok((params, result))
                        })
                    }
                }

                impl crate::brp_tools::handler::HasBrpMethod for #variant_name {
                    fn brp_method() -> crate::tool::BrpMethod {
                        crate::tool::BrpMethod::#variant_name
                    }
                }

                impl crate::brp_tools::handler::HasPortField for #params_ident {
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
    let mut from_str_arms = Vec::new();

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

            from_str_arms.push(quote! {
                #method => Some(BrpMethod::#variant_name)
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

            /// Parse a method string into a BrpMethod variant
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    #(#from_str_arms,)*
                    _ => None
                }
            }
        }

        impl std::fmt::Display for BrpMethod {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
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

/// Extract params and result from brp_tool attribute
fn extract_brp_tool_attr(attrs: &[syn::Attribute]) -> (Option<String>, Option<String>) {
    for attr in attrs {
        if attr.path().is_ident("brp_tool") {
            let mut params = None;
            let mut result = None;

            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("params") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    params = Some(s.value());
                } else if meta.path.is_ident("result") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    result = Some(s.value());
                } else {
                    return Err(meta.error("unsupported brp_tool attribute"));
                }
                Ok(())
            });

            return (params, result);
        }
    }
    (None, None)
}
