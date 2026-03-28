//! `BrpTools` derive macro implementation

use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::parse_macro_input;

/// Attributes extracted from #[tool(...)]
struct ToolAttrs {
    params:     Option<String>,
    result:     Option<String>,
    brp_method: String, // Make required (not Option)
}

/// Implementation of the `BrpTools` derive macro
pub fn derive_brp_tools_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Ensure we're working with an enum
    let Data::Enum(data_enum) = &input.data else {
        panic!("BrpTools can only be derived for enums");
    };

    let (marker_structs, tool_impls) = generate_tool_impls(data_enum);
    let method_match_arms = generate_method_match_arms(data_enum);
    let brp_method_parts = generate_brp_method_parts(data_enum);

    let enum_name = &input.ident;
    let expanded = assemble_output(
        enum_name,
        &marker_structs,
        &tool_impls,
        &method_match_arms,
        &brp_method_parts,
    );

    TokenStream::from(expanded)
}

/// Collected token streams for the `BrpMethod` enum and its conversions.
struct BrpMethodParts {
    variants:           Vec<proc_macro2::TokenStream>,
    to_brp_method_arms: Vec<proc_macro2::TokenStream>,
    from_brp_method:    Vec<proc_macro2::TokenStream>,
    string_arms:        Vec<proc_macro2::TokenStream>,
    from_str_arms:      Vec<proc_macro2::TokenStream>,
}

/// Generate marker structs and `ToolFn` implementations for each variant.
fn generate_tool_impls(
    data_enum: &syn::DataEnum,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    let mut tool_impls = Vec::new();
    let mut marker_structs = Vec::new();

    for variant in &data_enum.variants {
        assert!(
            matches!(variant.fields, Fields::Unit),
            "BrpTools can only be derived for enums with unit variants"
        );

        let variant_name = &variant.ident;
        let tool_attrs = extract_tool_attr(&variant.attrs);

        let method = if tool_attrs.brp_method.is_empty() {
            None
        } else {
            Some(tool_attrs.brp_method.clone())
        };
        let tool_params = tool_attrs.params;
        let tool_result = tool_attrs.result;

        if tool_params.is_some() && method.is_some() {
            marker_structs.push(quote! {
                pub struct #variant_name;
            });
        }

        if let Some(params) = tool_params
            && method.is_some()
        {
            let params_ident = syn::Ident::new(&params, variant_name.span());
            let result_str = tool_result
                .as_ref()
                .expect("BRP tools must specify a result type");
            let result_ident = syn::Ident::new(result_str, variant_name.span());
            let result_type = quote! { #result_ident };

            tool_impls.push(generate_tool_fn_impl(
                variant_name,
                &params_ident,
                &result_type,
            ));
        }
    }

    (marker_structs, tool_impls)
}

/// Generate a single `ToolFn` implementation for a BRP tool variant.
fn generate_tool_fn_impl(
    variant_name: &syn::Ident,
    params_ident: &syn::Ident,
    result_type: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        impl crate::tool::ToolFn for #variant_name {
            type Output = #result_type;
            type Params = #params_ident;

            fn call(
                &self,
                ctx: crate::tool::HandlerContext,
            ) -> crate::tool::HandlerResult<crate::tool::ToolResult<Self::Output, Self::Params>> {
                Box::pin(async move {
                    let params = ctx.extract_parameter_values::<#params_ident>()?;
                    let port = params.port;
                    let params_json = serde_json::to_value(&params).ok();

                    // Filter out transport-only metadata before sending BRP params.
                    let mut params_value = serde_json::to_value(&params)
                        .map_err(|e| crate::error::Error::InvalidArgument(format!(
                            "Failed to serialize parameters: {e}"
                        )))?;
                    let brp_params = if let serde_json::Value::Object(ref mut map) = params_value {
                        map.retain(|key, _value| key != &String::from(crate::tool::ParameterName::Port));
                        if map.is_empty() {
                            None
                        } else {
                            Some(params_value)
                        }
                    } else {
                        Some(params_value)
                    };
                    // Create BrpClient and execute
                    let client = crate::brp_tools::BrpClient::new(
                        crate::tool::BrpMethod::#variant_name,
                        port,
                        brp_params,
                    );
                    let result = match client.execute::<#result_type>().await {
                        Ok(r) => r,
                        Err(e) => {
                            let params = params_json
                                .and_then(|json| serde_json::from_value::<#params_ident>(json).ok());
                            return Ok(crate::tool::ToolResult {
                                result: Err(e),
                                params,
                            });
                        },
                    };

                    let params = params_json
                        .and_then(|json| serde_json::from_value::<#params_ident>(json).ok());

                    Ok(crate::tool::ToolResult {
                        result: Ok(result),
                        params,
                    })
                })
            }
        }
    }
}

/// Generate match arms for the `brp_method()` accessor on the tool enum.
fn generate_method_match_arms(data_enum: &syn::DataEnum) -> Vec<proc_macro2::TokenStream> {
    let mut method_match_arms = Vec::new();
    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let tool_attrs = extract_tool_attr(&variant.attrs);
        if tool_attrs.brp_method.is_empty() {
            method_match_arms.push(quote! {
                Self::#variant_name => None
            });
        } else {
            let method = &tool_attrs.brp_method;
            method_match_arms.push(quote! {
                Self::#variant_name => Some(#method)
            });
        }
    }
    method_match_arms
}

/// Generate `BrpMethod` enum variants and all associated conversion arms.
fn generate_brp_method_parts(data_enum: &syn::DataEnum) -> BrpMethodParts {
    let mut parts = BrpMethodParts {
        variants:           Vec::new(),
        to_brp_method_arms: Vec::new(),
        from_brp_method:    Vec::new(),
        string_arms:        Vec::new(),
        from_str_arms:      Vec::new(),
    };

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let tool_attrs = extract_tool_attr(&variant.attrs);
        if tool_attrs.brp_method.is_empty() {
            parts.to_brp_method_arms.push(quote! {
                Self::#variant_name => None
            });
        } else {
            let method = &tool_attrs.brp_method;
            parts.variants.push(quote! {
                #[serde(rename = #method)]
                #variant_name
            });
            parts.to_brp_method_arms.push(quote! {
                Self::#variant_name => Some(BrpMethod::#variant_name)
            });
            parts.from_brp_method.push(quote! {
                BrpMethod::#variant_name => Self::#variant_name
            });
            parts.string_arms.push(quote! {
                BrpMethod::#variant_name => #method
            });
            parts.from_str_arms.push(quote! {
                #method => Some(BrpMethod::#variant_name)
            });
        }
    }

    parts
}

/// Assemble the final output combining all generated parts.
fn assemble_output(
    enum_name: &syn::Ident,
    marker_structs: &[proc_macro2::TokenStream],
    tool_impls: &[proc_macro2::TokenStream],
    method_match_arms: &[proc_macro2::TokenStream],
    parts: &BrpMethodParts,
) -> proc_macro2::TokenStream {
    let brp_method_variants = &parts.variants;
    let to_brp_method_arms = &parts.to_brp_method_arms;
    let from_brp_method_arms = &parts.from_brp_method;
    let brp_method_string_arms = &parts.string_arms;
    let from_str_arms = &parts.from_str_arms;

    quote! {
        // Marker structs for all tools
        #(#marker_structs)*

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

            /// Converts to `BrpMethod` if this variant has a BRP method
            pub const fn to_brp_method(&self) -> Option<BrpMethod> {
                match self {
                    #(#to_brp_method_arms,)*
                }
            }
        }

        /// Enum containing only tool variants that have BRP methods
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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

            /// Parse a method string into a `BrpMethod` variant
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
    }
}

/// Extract unified tool attributes from #[tool(...)]
fn extract_tool_attr(attrs: &[syn::Attribute]) -> ToolAttrs {
    let mut tool_attrs = ToolAttrs {
        params:     None,
        result:     None,
        brp_method: String::new(), // Required field
    };

    let mut has_brp_tool = false;
    for attr in attrs {
        if attr.path().is_ident("brp_tool") {
            has_brp_tool = true;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("params") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    tool_attrs.params = Some(s.value());
                } else if meta.path.is_ident("result") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    tool_attrs.result = Some(s.value());
                } else if meta.path.is_ident("brp_method") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    tool_attrs.brp_method = s.value(); // Set required field
                } else {
                    return Err(meta.error("unsupported tool attribute"));
                }
                Ok(())
            });
            break;
        }
    }

    // Only validate if brp_tool attribute was present
    assert!(
        !(has_brp_tool && tool_attrs.brp_method.trim().is_empty()),
        "brp_tool attribute must include non-empty brp_method parameter"
    );

    tool_attrs
}
