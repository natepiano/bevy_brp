//! ResultStruct derive macro implementation
//!
//! This macro generates implementations for result structs used in MCP tools.
//! Result structs have private fields and require a #[to_message] attribute.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::shared::{ComputedField, extract_field_data};

/// Convert single-brace template placeholders to double-brace format
fn convert_template_braces(template: &str) -> String {
    // Replace {foo} with {{foo}}
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() != Some(&'{') {
            result.push_str("{{");
        } else if ch == '}' && chars.peek() != Some(&'}') {
            result.push_str("}}");
        } else {
            result.push(ch);
        }
    }

    result
}

/// Implementation of the ResultStruct derive macro
pub fn derive_result_struct_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Ensure we're working with a struct
    let Data::Struct(data_struct) = &input.data else {
        panic!("ResultStruct can only be derived for structs");
    };

    // Convert fields to a vec of references for the shared function
    let fields: Vec<_> = data_struct.fields.iter().collect();

    // Extract field information using shared function
    let extraction_result = extract_field_data(&fields);

    // Validate that there's a #[to_message] attribute
    if extraction_result.message_template_field.is_none() {
        panic!("ResultStruct must have a field with #[to_message] attribute.");
    }

    let field_placements = extraction_result.field_placements;
    let response_data_fields = extraction_result.response_data_fields;
    let regular_fields = extraction_result.regular_fields;
    let computed_fields = extraction_result.computed_fields;
    let message_template_field = extraction_result.message_template_field;
    let has_format_corrections = extraction_result.has_format_corrections;

    // Generate MessageTemplateProvider and constructor methods
    let message_template_impl = generate_message_template_provider(
        struct_name,
        &message_template_field,
        &regular_fields,
        &computed_fields,
    );

    // Generate from_brp_value method only if needed
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

        #message_template_impl
    };

    TokenStream::from(expanded)
}

/// Generate MessageTemplateProvider implementation and constructor methods
fn generate_message_template_provider(
    struct_name: &syn::Ident,
    message_template_field: &Option<(syn::Ident, Option<String>)>,
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
        for (name, ty) in regular_fields {
            if name == field_name {
                // Check if the field type is Option<String> or String
                let type_str = quote!(#ty).to_string();
                let is_option = type_str.contains("Option <");

                if let Some(template) = default_template {
                    let converted_template = convert_template_braces(template);
                    if is_option {
                        field_initializers
                            .push(quote! { #name: Some(#converted_template.to_string()) });
                    } else {
                        field_initializers.push(quote! { #name: #converted_template.to_string() });
                    }
                } else {
                    // No default template
                    if is_option {
                        field_initializers.push(quote! { #name: None });
                    } else {
                        // This is an error case - String field with no default
                        panic!(
                            "Message template field must be Option<String> when no default template is provided"
                        );
                    }
                }
            } else {
                field_initializers.push(quote! { #name });
            }
        }

        // Handle computed fields with default values
        for computed in computed_fields {
            let field_name = &computed.field_name;
            // Provide default values for computed fields
            let default_value = match computed.operation.as_str() {
                "count" | "count_components" | "count_methods" | "count_query_components" => {
                    quote! { 0 }
                }
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

        // Determine if the field is Option<String> or String
        let message_field_type = regular_fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, ty)| ty);

        let is_option_type = message_field_type
            .map(|ty| quote!(#ty).to_string().contains("Option <"))
            .unwrap_or(false);

        let get_template_impl = if is_option_type {
            quote! {
                self.#field_name.as_ref()
                    .map(|s| s.as_str())
                    .ok_or_else(|| {
                        error_stack::Report::new(crate::error::Error::Configuration(
                            "Message template not set. Use .with_message_template() to provide a template.".to_string()
                        ))
                    })
            }
        } else {
            quote! {
                Ok(self.#field_name.as_str())
            }
        };

        let with_template_impl = if is_option_type {
            quote! {
                self.#field_name = Some(template.into());
            }
        } else {
            quote! {
                self.#field_name = template.into();
            }
        };

        // For Option<String> types without defaults, generate a builder
        if is_option_type && default_template.is_none() {
            let builder_name = quote::format_ident!("{}Builder", struct_name);

            // Get field names for the builder constructor
            let field_names: Vec<_> = regular_fields
                .iter()
                .filter(|(name, _)| name != field_name)
                .map(|(name, _)| name.clone())
                .collect();

            // Get field types for builder struct
            let builder_fields: Vec<_> = regular_fields
                .iter()
                .filter(|(name, _)| name != field_name)
                .map(|(name, ty)| quote! { #name: #ty })
                .collect();

            // Create initializers for building the final struct
            let mut builder_to_struct_initializers = Vec::new();
            for (name, _) in regular_fields {
                if name == field_name {
                    // Skip - we'll add this with the template parameter
                } else {
                    builder_to_struct_initializers.push(quote! { #name: self.#name });
                }
            }

            // Add computed field initializers
            for computed in computed_fields {
                let field_name = &computed.field_name;
                let default_value = match computed.operation.as_str() {
                    "count" | "count_components" | "count_methods" | "count_query_components" => {
                        quote! { 0 }
                    }
                    "count_errors" => quote! { None },
                    "extract_entity" => quote! { 0 },
                    "extract_duration_ms" => quote! { 100 },
                    "extract_keys_sent" => quote! { Vec::new() },
                    "extract_debug_enabled" => quote! { false },
                    "extract_message" => quote! { String::new() },
                    _ => quote! { Default::default() },
                };
                builder_to_struct_initializers.push(quote! { #field_name: #default_value });
            }

            quote! {
                impl crate::tool::MessageTemplateProvider for #struct_name {
                    fn get_message_template(&self) -> crate::error::Result<&str> {
                        #get_template_impl
                    }
                }

                pub struct #builder_name {
                    #(#builder_fields,)*
                }

                impl #builder_name {
                    /// Set the message template and build the final result
                    pub fn with_message_template(self, template: impl Into<String>) -> #struct_name {
                        #struct_name {
                            #(#builder_to_struct_initializers,)*
                            #field_name: Some(template.into()),
                        }
                    }
                }

                impl #struct_name {
                    /// Create a new instance - requires setting message template
                    #[allow(clippy::too_many_arguments)]
                    pub fn new(#(#constructor_params),*) -> #builder_name {
                        #builder_name {
                            #(#field_names,)*
                        }
                    }

                    /// Override the message template for this result
                    pub fn with_message_template(mut self, template: impl Into<String>) -> Self {
                        #with_template_impl
                        self
                    }
                }
            }
        } else {
            quote! {
                impl crate::tool::MessageTemplateProvider for #struct_name {
                    fn get_message_template(&self) -> crate::error::Result<&str> {
                        #get_template_impl
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
                        #with_template_impl
                        self
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
    message_template_field: &Option<(syn::Ident, Option<String>)>,
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
                // Check if the field type is Option<String> or String
                let type_str = quote!(#field_type).to_string();
                let is_option = type_str.contains("Option <");

                if let Some(template) = template_default {
                    let converted_template = convert_template_braces(template);
                    if is_option {
                        field_initializers
                            .push(quote! { #field_name: Some(#converted_template.to_string()) });
                    } else {
                        field_initializers
                            .push(quote! { #field_name: #converted_template.to_string() });
                    }
                } else {
                    // No default template
                    if is_option {
                        field_initializers.push(quote! { #field_name: None });
                    } else {
                        panic!(
                            "Message template field must be Option<String> when no default template is provided"
                        );
                    }
                }
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
                        .map(|v| {
                            if let Some(arr) = v.as_array() {
                                arr.len()
                            } else if let Some(obj) = v.as_object() {
                                obj.len()
                            } else {
                                0
                            }
                        })
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
