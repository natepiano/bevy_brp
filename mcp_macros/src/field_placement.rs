//! FieldPlacement derive macro implementation
//!
//! This macro generates implementations for result structs used in MCP tools.
//! 
//! ## Important: Result Struct Construction
//! 
//! When a struct has a `#[to_message(message_template = "...")]` field, the macro:
//! - Makes all struct fields private
//! - Generates a `::new()` constructor with all fields except `message_template`
//! - Generates a `with_message_template()` method for overriding the default template
//! 
//! This ensures result structs can ONLY be constructed via `::new()`, preventing:
//! - Forgetting to set the message template
//! - Inconsistent construction patterns
//! - Direct field access from outside the module

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
    let mut message_template_field = None;

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

            field_accessors.push(generate_field_accessor(field_name, field_type));
            response_data_fields.push(generate_response_data_field(
                field_name,
                field_type,
                placement,
                skip_if_none,
            ));
        }
    }

    // Generate CallInfoProvider if needed
    let call_info_impl = generate_call_info_provider(struct_name, &call_info_fields);

    // Generate MessageTemplateProvider and constructor methods if needed
    let message_template_impl = generate_message_template_provider(
        struct_name,
        &message_template_field,
        &regular_fields,
        &computed_fields,
    );

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
            &message_template_field,
        )
    } else {
        quote! {}
    };

    // Generate the trait implementations
    let expanded = quote! {
        impl crate::tool::HasFieldPlacement for #struct_name {
            fn field_placements() -> Vec<crate::tool::FieldPlacementInfo> {
                vec![
                    #(#field_placements,)*
                ]
            }
        }

        impl crate::tool::ResponseData for #struct_name {
            fn add_response_fields(&self, builder: crate::tool::ResponseBuilder) -> crate::error::Result<crate::tool::ResponseBuilder> {
                let mut builder = builder;
                #(#response_data_fields)*
                Ok(builder)
            }
        }

        #from_brp_value_impl

        #call_info_impl

        #message_template_impl
    };

    TokenStream::from(expanded)
}

/// Parse placement attribute arguments
fn parse_placement_attr(
    attr: &syn::Attribute,
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
fn parse_computed_attr(attr: &syn::Attribute, result_operation: &mut Option<String>) {
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
fn parse_to_message_attr(attr: &syn::Attribute) -> String {
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

    message_template.expect("to_message attribute requires message_template parameter")
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

/// Generate MessageTemplateProvider implementation and constructor methods
fn generate_message_template_provider(
    struct_name: &syn::Ident,
    message_template_field: &Option<(syn::Ident, String)>,
    regular_fields: &[(syn::Ident, syn::Type)],
    computed_fields: &[ComputedField],
) -> proc_macro2::TokenStream {
    if let Some((field_name, default_template)) = message_template_field {
        // Create parameter list for constructor (excluding message_template field)
        let constructor_params: Vec<_> = regular_fields
            .iter()
            .filter(|(name, _)| name != field_name)
            .map(|(name, ty)| quote! { #name: #ty })
            .collect();

        // Create field initializers for constructor
        let mut field_initializers = Vec::new();

        // Handle regular fields
        for (name, _) in regular_fields {
            if name == field_name {
                field_initializers.push(quote! { #name: #default_template.to_string() });
            } else {
                field_initializers.push(quote! { #name });
            }
        }

        // Handle computed fields with default values
        for computed in computed_fields {
            let field_name = &computed.field_name;
            // Provide default values for computed fields
            let default_value = match computed.operation.as_str() {
                "count"
                | "count_object"
                | "count_components"
                | "count_methods"
                | "count_query_components" => quote! { 0 },
                "count_errors" => quote! { None }, // count_errors is always optional
                "extract_entity" => quote! { 0 },
                "extract_duration_ms" => quote! { 100 },
                "extract_keys_sent" => quote! { Vec::new() },
                "extract_debug_enabled" => quote! { false },
                "extract_message" => quote! { String::new() },
                _ => quote! { Default::default() },
            };
            field_initializers.push(quote! { #field_name: #default_value });
        }

        quote! {
            impl crate::tool::MessageTemplateProvider for #struct_name {
                fn get_message_template(&self) -> &str {
                    &self.#field_name
                }
            }

            impl #struct_name {
                /// Create a new instance with default message template
                #[allow(clippy::too_many_arguments)]
                pub fn new(#(#constructor_params),*) -> Self {
                    Self {
                        #(#field_initializers,)*
                    }
                }

                /// Override the message template for this result
                pub fn with_message_template(mut self, template: impl Into<String>) -> Self {
                    self.#field_name = template.into();
                    self
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
    message_template_field: &Option<(syn::Ident, String)>,
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
        } else if let Some((template_field_name, template_default)) = message_template_field {
            if field_name == template_field_name {
                field_initializers.push(quote! { #field_name: #template_default.to_string() });
            }
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
            let from_ident = syn::Ident::new(from_field, field_name.span());
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
            _ => panic!("Unknown computed operation: {operation}"),
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
