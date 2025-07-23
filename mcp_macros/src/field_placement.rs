//! FieldPlacement derive macro implementation

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

/// Information about a computed field
struct ComputedField {
    field_name: syn::Ident,
    from_field: String,
    operation:  String,
}

/// Implementation of the FieldPlacement derive macro
pub fn derive_field_placement_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Ensure we're working with a struct
    let Data::Struct(data_struct) = &input.data else {
        panic!("FieldPlacement can only be derived for structs");
    };

    // Collect field information
    let mut field_placements = Vec::new();
    let mut field_accessors = Vec::new();
    let mut response_data_fields = Vec::new();
    let mut call_info_fields = Vec::new();
    let mut computed_fields = Vec::new();
    let mut regular_fields = Vec::new();
    let mut has_format_corrections = false;

    for field in &data_struct.fields {
        let field_name = field
            .ident
            .as_ref()
            .expect("FieldPlacement only works with named fields");
        let field_type = &field.ty;

        // Check for our placement attributes
        let mut placement = None;
        let mut source_path = None;
        let mut field_type_override = None;
        let mut skip_if_none = false;
        let mut is_computed = false;
        let mut computed_from = None;
        let mut computed_operation = None;

        for attr in &field.attrs {
            if attr.path().is_ident("to_metadata") {
                placement = Some(quote! { crate::response::FieldPlacement::Metadata });
                parse_placement_attr(
                    attr,
                    &mut source_path,
                    &mut field_type_override,
                    &mut skip_if_none,
                    &mut computed_from,
                    &mut computed_operation,
                );
            } else if attr.path().is_ident("to_result") {
                placement = Some(quote! { crate::response::FieldPlacement::Result });
                parse_placement_attr(
                    attr,
                    &mut source_path,
                    &mut field_type_override,
                    &mut skip_if_none,
                    &mut computed_from,
                    &mut computed_operation,
                );
            } else if attr.path().is_ident("to_call_info") {
                call_info_fields.push(field_name.clone());
                continue; // Skip adding to other collections
            } else if attr.path().is_ident("computed") {
                is_computed = true;
                parse_computed_attr(attr, &mut computed_from, &mut computed_operation);
            }
        }

        // If we found computed_from and computed_operation in placement attrs, mark as computed
        if computed_from.is_some() && computed_operation.is_some() {
            is_computed = true;
        }

        // Check if this is a format corrections field
        if field_name == "format_corrections" || field_name == "format_corrected" {
            has_format_corrections = true;
        }

        // Handle computed fields
        if is_computed {
            if let (Some(from), Some(operation)) = (computed_from, computed_operation) {
                computed_fields.push(ComputedField {
                    field_name: field_name.clone(),
                    from_field: from,
                    operation,
                });
            }
        } else {
            regular_fields.push((field_name.clone(), field_type.clone()));
        }

        // Only add placement info if there's a placement attribute
        if let Some(placement) = &placement {
            let field_name_str = field_name.to_string();
            let field_type_token = field_type_override
                .map(|t| match t.as_str() {
                    "String" => quote! { crate::response::ResponseFieldType::String },
                    "Number" => quote! { crate::response::ResponseFieldType::Number },
                    "Boolean" => quote! { crate::response::ResponseFieldType::Boolean },
                    "StringArray" => quote! { crate::response::ResponseFieldType::StringArray },
                    "NumberArray" => quote! { crate::response::ResponseFieldType::NumberArray },
                    "Any" => quote! { crate::response::ResponseFieldType::Any },
                    "Count" => quote! { crate::response::ResponseFieldType::Count },
                    "LineSplit" => quote! { crate::response::ResponseFieldType::LineSplit },
                    "QueryComponentCount" => {
                        quote! { crate::response::ResponseFieldType::QueryComponentCount }
                    }
                    _ => panic!("Unknown field type: {}", t),
                })
                .unwrap_or_else(|| infer_field_type(field_type));

            let source_path_token = source_path
                .as_ref()
                .map(|s| quote! { Some(#s) })
                .unwrap_or_else(|| quote! { None });

            field_placements.push(quote! {
                crate::response::FieldPlacementInfo {
                    field_name: #field_name_str,
                    placement: #placement,
                    source_path: #source_path_token,
                    field_type: #field_type_token,
                    skip_if_none: #skip_if_none,
                }
            });

            field_accessors.push(generate_field_accessor(field_name, field_type));
            response_data_fields.push(generate_response_data_field(
                field_name,
                field_type,
                &placement,
                skip_if_none,
            ));
        }
    }

    // Generate CallInfoProvider if needed
    let call_info_impl = generate_call_info_provider(struct_name, &call_info_fields);

    // Generate from_brp_value method only if needed (for result structs)
    let from_brp_value_impl = if !computed_fields.is_empty()
        || has_format_corrections
        || regular_fields.iter().any(|(name, _)| name == "result")
    {
        generate_from_brp_value(
            struct_name,
            &regular_fields,
            &computed_fields,
            has_format_corrections,
        )
    } else {
        quote! {}
    };

    // Generate the trait implementations
    let expanded = quote! {
        impl crate::response::HasFieldPlacement for #struct_name {
            fn field_placements() -> Vec<crate::response::FieldPlacementInfo> {
                vec![
                    #(#field_placements,)*
                ]
            }
        }

        impl crate::response::FieldAccessor for #struct_name {
            fn get_field(&self, name: &str) -> Option<crate::response::ExtractedValue> {
                match name {
                    #(#field_accessors,)*
                    _ => None,
                }
            }
        }

        impl crate::response::ResponseData for #struct_name {
            fn add_response_fields(&self, builder: crate::response::ResponseBuilder) -> crate::error::Result<crate::response::ResponseBuilder> {
                let mut builder = builder;
                #(#response_data_fields)*
                Ok(builder)
            }
        }

        #from_brp_value_impl

        #call_info_impl
    };

    TokenStream::from(expanded)
}

/// Parse placement attribute arguments
fn parse_placement_attr(
    attr: &syn::Attribute,
    source_path: &mut Option<String>,
    field_type: &mut Option<String>,
    skip_if_none: &mut bool,
    computed_from: &mut Option<String>,
    computed_operation: &mut Option<String>,
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
        } else if meta.path.is_ident("computed_from") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *computed_from = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("computed_operation") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *computed_operation = Some(s.value());
            Ok(())
        } else {
            Err(meta.error("unsupported attribute"))
        }
    });
}

/// Parse computed attribute arguments
fn parse_computed_attr(
    attr: &syn::Attribute,
    from_field: &mut Option<String>,
    operation: &mut Option<String>,
) {
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("from") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *from_field = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("operation") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *operation = Some(s.value());
            Ok(())
        } else {
            Err(meta.error("unsupported computed attribute"))
        }
    });
}

/// Infer ResponseFieldType from Rust type
fn infer_field_type(ty: &syn::Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();

    if type_str.contains("String") {
        quote! { crate::response::ResponseFieldType::String }
    } else if type_str.contains("usize")
        || type_str.contains("u64")
        || type_str.contains("u32")
        || type_str.contains("u16")
    {
        quote! { crate::response::ResponseFieldType::Number }
    } else if type_str.contains("bool") {
        quote! { crate::response::ResponseFieldType::Boolean }
    } else if type_str.contains("Vec < String >") {
        quote! { crate::response::ResponseFieldType::StringArray }
    } else if type_str.contains("Vec < u") {
        quote! { crate::response::ResponseFieldType::NumberArray }
    } else {
        quote! { crate::response::ResponseFieldType::Any }
    }
}

/// Generate field accessor match arm
fn generate_field_accessor(
    field_name: &syn::Ident,
    field_type: &syn::Type,
) -> proc_macro2::TokenStream {
    let field_name_str = field_name.to_string();
    let type_str = quote!(#field_type).to_string();

    // Handle Option types
    if type_str.starts_with("Option <") {
        quote! {
            #field_name_str => self.#field_name.as_ref().map(|v| v.clone().into())
        }
    } else {
        quote! {
            #field_name_str => Some(self.#field_name.clone().into())
        }
    }
}

/// Generate response data field addition
fn generate_response_data_field(
    field_name: &syn::Ident,
    field_type: &syn::Type,
    placement: &proc_macro2::TokenStream,
    skip_if_none: bool,
) -> proc_macro2::TokenStream {
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
fn generate_call_info_provider(
    struct_name: &syn::Ident,
    call_info_fields: &[syn::Ident],
) -> proc_macro2::TokenStream {
    if call_info_fields.is_empty() {
        return quote! {};
    }

    // For now, assume port is the main call_info field
    // This can be extended to handle other patterns
    let has_port = call_info_fields.iter().any(|f| f == "port");

    if has_port {
        quote! {
            impl crate::response::CallInfoProvider for #struct_name {
                fn to_call_info(&self, tool_name: String) -> crate::response::CallInfo {
                    use crate::tool::ToolName;
                    use std::str::FromStr;

                    if let Ok(tn) = ToolName::from_str(&tool_name) {
                        if let Some(brp_method) = tn.to_brp_method() {
                            crate::response::CallInfo::brp(tool_name, brp_method.to_string(), self.port)
                        } else {
                            crate::response::CallInfo::local_with_port(tool_name, self.port)
                        }
                    } else {
                        crate::response::CallInfo::local(tool_name)
                    }
                }
            }
        }
    } else {
        quote! {}
    }
}

/// Generate from_brp_value method
fn generate_from_brp_value(
    struct_name: &syn::Ident,
    regular_fields: &[(syn::Ident, syn::Type)],
    computed_fields: &[ComputedField],
    has_format_corrections: bool,
) -> proc_macro2::TokenStream {
    let mut field_initializers = Vec::new();

    // Handle regular fields
    for (field_name, field_type) in regular_fields {
        let type_str = quote!(#field_type).to_string();

        if field_name == "result" && type_str.contains("Option < Value >") {
            field_initializers.push(quote! { result: value.clone() });
        } else if field_name == "format_corrections" {
            field_initializers.push(quote! {
                format_corrections: if format_corrections.as_ref().map_or(true, |v| v.is_empty()) {
                    None
                } else {
                    format_corrections
                }
            });
        } else if field_name == "format_corrected" {
            field_initializers.push(quote! {
                format_corrected: match format_corrected {
                    Some(crate::brp_tools::FormatCorrectionStatus::NotAttempted) | None => None,
                    other => other,
                }
            });
        }
        // Other regular fields would need to be passed as parameters or have defaults
    }

    // Handle computed fields
    for computed in computed_fields {
        let field_name = &computed.field_name;
        let from_field = &computed.from_field;
        let operation = &computed.operation;

        // Map field names to parameter names
        let source = if from_field == "result" {
            quote! { value }
        } else {
            let from_ident = syn::Ident::new(&from_field, field_name.span());
            quote! { #from_ident }
        };

        let computation = match operation.as_str() {
            "count" => {
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.len())
                        .unwrap_or(0)
                }
            }
            "count_object" => {
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.len())
                        .unwrap_or(0)
                }
            }
            "count_components" => {
                // For bevy/get result structure
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("components"))
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.len())
                        .unwrap_or(0)
                }
            }
            "count_errors" => {
                // For bevy/get result structure
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("errors"))
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.len())
                }
            }
            "count_query_components" => {
                // Total components across all entities in query result
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_array())
                        .map(|entities| {
                            entities
                                .iter()
                                .filter_map(|e| e.as_object())
                                .map(|obj| obj.len())
                                .sum()
                        })
                        .unwrap_or(0)
                }
            }
            "count_methods" => {
                // For rpc.discover
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("methods"))
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.len())
                        .unwrap_or(0)
                }
            }
            "extract_entity" => {
                // For spawn result - extracts entity ID from {entity: 123}
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("entity"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                }
            }
            "extract_keys_sent" => {
                // For send_keys result
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("keys_sent"))
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_else(Vec::new)
                }
            }
            "extract_duration_ms" => {
                // For send_keys result
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("duration_ms"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                        .unwrap_or(100)
                }
            }
            "extract_debug_enabled" => {
                // For set_debug_mode result
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("debug_enabled"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                }
            }
            "extract_message" => {
                // For extracting message field
                quote! {
                    #source.as_ref()
                        .and_then(|v| v.as_object())
                        .and_then(|obj| obj.get("message"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                }
            }
            _ => panic!("Unknown computed operation: {}", operation),
        };

        field_initializers.push(quote! { #field_name: #computation });
    }

    // Generate the method signature based on whether format corrections are present
    let params = if has_format_corrections {
        quote! {
            value: Option<serde_json::Value>,
            format_corrections: Option<Vec<serde_json::Value>>,
            format_corrected: Option<crate::brp_tools::FormatCorrectionStatus>,
        }
    } else {
        quote! {
            value: Option<serde_json::Value>,
        }
    };

    quote! {
        impl #struct_name {
            /// Create from BRP response value
            pub fn from_brp_value(#params) -> crate::error::Result<Self> {
                Ok(Self {
                    #(#field_initializers,)*
                })
            }
        }
    }
}
