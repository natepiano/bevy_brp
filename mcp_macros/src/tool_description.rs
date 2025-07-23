//! ToolDescription derive macro implementation

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Implementation of the ToolDescription derive macro
pub fn derive_tool_description_impl(input: TokenStream) -> TokenStream {
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
