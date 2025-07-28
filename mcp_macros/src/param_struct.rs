//! ParamStruct derive macro implementation
//!
//! This macro generates implementations for parameter structs used in MCP tools.
//! Parameter structs are deserialized from JSON and have public fields.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::shared::extract_field_data;

/// Implementation of the ParamStruct derive macro
pub fn derive_param_struct_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Ensure we're working with a struct
    let Data::Struct(data_struct) = &input.data else {
        panic!("ParamStruct can only be derived for structs");
    };

    // Convert fields to a vec of references for the shared function
    let fields: Vec<_> = data_struct.fields.iter().collect();

    // Extract field information using shared function
    let extraction_result = extract_field_data(&fields);

    // Validate that there's no #[to_message] attribute
    if extraction_result.message_template_field.is_some() {
        panic!(
            "ParamStruct cannot have #[to_message] attributes. Use ResultStruct for result types."
        );
    }

    let field_placements = extraction_result.field_placements;

    // Generate the trait implementations
    let expanded = quote! {
        impl crate::tool::HasFieldPlacement for #struct_name {
            fn field_placements() -> Vec<crate::tool::FieldPlacementInfo> {
                vec![
                    #(#field_placements,)*
                ]
            }
        }

        impl crate::tool::ParamStruct for #struct_name {}
    };

    TokenStream::from(expanded)
}
