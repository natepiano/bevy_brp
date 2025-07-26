//! Shared utilities for field placement macros

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Field, Ident, Type};

/// Information about a computed field
pub struct ComputedField {
    pub field_name: Ident,
    pub from_field: String,
    pub operation: String,
}


/// Parse placement attribute arguments
pub fn parse_placement_attr(
    attr: &Attribute,
    source_path: &mut Option<String>,
    field_type: &mut Option<String>,
    skip_if_none: &mut bool,
    result_operation: &mut Option<String>,
) {
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("from") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *source_path = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("field_type") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *field_type = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("skip_if_none") {
            *skip_if_none = true;
            Ok(())
        } else if meta.path.is_ident("result_operation") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *result_operation = Some(s.value());
            Ok(())
        } else {
            Err(meta.error("unsupported attribute"))
        }
    });
}

/// Parse computed attribute arguments
pub fn parse_computed_attr(attr: &Attribute, result_operation: &mut Option<String>) {
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("operation") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *result_operation = Some(s.value());
            Ok(())
        } else {
            Err(meta.error("unsupported computed attribute"))
        }
    });
}

/// Parse to_message attribute arguments
pub fn parse_to_message_attr(attr: &Attribute) -> Option<String> {
    let mut message_template = None;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("message_template") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            message_template = Some(s.value());
            Ok(())
        } else {
            Err(meta.error("unsupported to_message attribute"))
        }
    });

    message_template
}


/// Generate response data field addition
pub fn generate_response_data_field(
    field_name: &Ident,
    field_type: &Type,
    placement: &TokenStream,
    skip_if_none: bool,
) -> TokenStream {
    let field_name_str = field_name.to_string();
    let type_str = quote!(#field_type).to_string();

    // Handle Option types with skip_if_none
    if type_str.starts_with("Option <") && skip_if_none {
        quote! {
            if let Some(val) = &self.#field_name {
                builder = builder.add_field_to(#field_name_str, val, #placement)?;
            }
        }
    } else {
        quote! {
            builder = builder.add_field_to(#field_name_str, &self.#field_name, #placement)?;
        }
    }
}

/// Generate CallInfoProvider implementation if there are call_info fields
pub fn generate_call_info_provider(
    struct_name: &Ident,
    call_info_fields: &[Ident],
) -> TokenStream {
    if call_info_fields.is_empty() {
        return quote! {};
    }

    // For now, assume port is the main call_info field
    // This can be extended to handle other patterns
    let has_port = call_info_fields.iter().any(|f| f == "port");

    if has_port {
        quote! {
            impl crate::tool::CallInfoProvider for #struct_name {
                fn to_call_info(&self, tool_name: String) -> crate::tool::CallInfo {
                    use crate::tool::ToolName;
                    use std::str::FromStr;

                    if let Ok(tn) = ToolName::from_str(&tool_name) {
                        if let Some(brp_method) = tn.to_brp_method() {
                            crate::tool::CallInfo::brp(tool_name, brp_method.to_string(), self.port)
                        } else {
                            crate::tool::CallInfo::local_with_port(tool_name, self.port)
                        }
                    } else {
                        crate::tool::CallInfo::local(tool_name)
                    }
                }
            }
        }
    } else {
        quote! {}
    }
}

/// Extract field data from struct fields
pub fn extract_field_data(fields: &[&Field]) -> FieldExtractionResult {
    let mut field_placements = Vec::new();
    let mut response_data_fields = Vec::new();
    let mut call_info_fields = Vec::new();
    let mut computed_fields = Vec::new();
    let mut regular_fields = Vec::new();
    let mut has_format_corrections = false;
    let mut message_template_field: Option<(Ident, Option<String>)> = None;

    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .expect("Only works with named fields");
        let field_type = &field.ty;

        // Check for our placement attributes
        let mut placement = None;
        let mut source_path = None;
        let mut field_type_override = None;
        let mut skip_if_none = false;
        let mut is_computed = false;
        let mut result_operation = None;

        for attr in &field.attrs {
            if attr.path().is_ident("to_metadata") {
                placement = Some(quote! { crate::tool::FieldPlacement::Metadata });
                parse_placement_attr(
                    attr,
                    &mut source_path,
                    &mut field_type_override,
                    &mut skip_if_none,
                    &mut result_operation,
                );
            } else if attr.path().is_ident("to_result") {
                placement = Some(quote! { crate::tool::FieldPlacement::Result });
                parse_placement_attr(
                    attr,
                    &mut source_path,
                    &mut field_type_override,
                    &mut skip_if_none,
                    &mut result_operation,
                );
            } else if attr.path().is_ident("to_call_info") {
                call_info_fields.push(field_name.clone());
                continue; // Skip adding to other collections
            } else if attr.path().is_ident("computed") {
                is_computed = true;
                parse_computed_attr(attr, &mut result_operation);
            } else if attr.path().is_ident("to_message") {
                let template = parse_to_message_attr(attr);
                message_template_field = Some((field_name.clone(), template));
                continue; // Skip adding to other collections
            }
        }

        // If we found result_operation in placement attrs, mark as computed
        if result_operation.is_some() {
            is_computed = true;
        }

        // Check if this is a format corrections field
        if field_name == "format_corrections" || field_name == "format_corrected" {
            has_format_corrections = true;
        }

        // Handle computed fields
        if is_computed {
            if let Some(operation) = result_operation {
                computed_fields.push(ComputedField {
                    field_name: field_name.clone(),
                    from_field: "result".to_string(), // Always operate on result
                    operation,
                });
            }
        } else {
            regular_fields.push((field_name.clone(), field_type.clone()));
        }

        // Only add placement info if there's a placement attribute
        if let Some(placement) = &placement {
            let field_name_str = field_name.to_string();

            let source_path_token = source_path
                .as_ref()
                .map(|s| quote! { Some(#s) })
                .unwrap_or_else(|| quote! { None });

            field_placements.push(quote! {
                crate::tool::FieldPlacementInfo {
                    field_name: #field_name_str,
                    placement: #placement,
                    source_path: #source_path_token,
                    skip_if_none: #skip_if_none,
                }
            });

            response_data_fields.push(generate_response_data_field(
                field_name,
                field_type,
                placement,
                skip_if_none,
            ));
        }
    }

    FieldExtractionResult {
        field_placements,
        response_data_fields,
        call_info_fields,
        computed_fields,
        regular_fields,
        has_format_corrections,
        message_template_field,
    }
}

pub struct FieldExtractionResult {
    pub field_placements: Vec<TokenStream>,
    pub response_data_fields: Vec<TokenStream>,
    pub call_info_fields: Vec<Ident>,
    pub computed_fields: Vec<ComputedField>,
    pub regular_fields: Vec<(Ident, Type)>,
    pub has_format_corrections: bool,
    pub message_template_field: Option<(Ident, Option<String>)>,
}