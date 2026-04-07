//! `ResultStruct` derive macro implementation
//!
//! This macro generates implementations for result structs used in MCP tools.
//! Result structs have private fields and require a #[`to_message`] attribute.

use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::parse_macro_input;

use crate::shared;
use crate::shared::ComputedField;

/// Attributes for #[`brp_result`(...)]
#[derive(Default)]
struct BrpResultAttrs {
    enhanced_errors: bool,
}

/// Parse #[`brp_result`(...)] attribute
fn parse_brp_result_attr(attrs: &[syn::Attribute]) -> Option<BrpResultAttrs> {
    for attr in attrs {
        if attr.path().is_ident("brp_result") {
            let mut result = BrpResultAttrs::default();

            // Parse attribute arguments if any
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("enhanced_errors") {
                    let value = meta.value()?;
                    let lit: syn::LitBool = value.parse()?;
                    result.enhanced_errors = lit.value();
                }
                Ok(())
            });

            return Some(result);
        }
    }
    None
}

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

/// Implementation of the `ResultStruct` derive macro
pub(crate) fn derive_result_struct_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Parse #[brp_result] attribute
    let brp_attrs = parse_brp_result_attr(&input.attrs);

    // Ensure we're working with a struct
    let Data::Struct(data_struct) = input.data else {
        panic!("ResultStruct can only be derived for structs");
    };

    // Convert fields to a vec of references for the shared function
    let fields: Vec<_> = data_struct.fields.iter().collect();

    // Extract field information using shared function
    let extraction_result = shared::extract_field_data(&fields);

    // Validate that there's a #[to_message] attribute
    assert!(
        extraction_result.message_template_field.is_some(),
        "ResultStruct must have a field with #[to_message] attribute."
    );

    let field_placements = extraction_result.field_placements;
    let response_data_fields = extraction_result.response_data_fields;
    let regular_fields = extraction_result.regular_fields;
    let computed_fields = extraction_result.computed_fields;
    let message_template_field = extraction_result.message_template_field;

    let get_template_impl = generate_get_template_impl(&fields, message_template_field.as_ref());

    let message_template_impl = generate_message_template_provider(
        struct_name,
        message_template_field.as_ref(),
        &regular_fields,
        &computed_fields,
    );

    let brp_impls = generate_brp_trait_impls(
        struct_name,
        brp_attrs.as_ref(),
        &regular_fields,
        &computed_fields,
        message_template_field.as_ref(),
    );

    // Generate the trait implementations
    let expanded = quote! {
        impl crate::tool::HasFieldPlacement for #struct_name {
            fn field_placements() -> Vec<crate::tool::FieldPlacementInfo> {
                vec![
                    #(#field_placements,)*
                ]
            }
        }

        impl crate::tool::ResultStruct for #struct_name {
            fn add_response_fields(&self, builder: crate::tool::ResponseBuilder) -> crate::error::Result<crate::tool::ResponseBuilder> {
                let mut builder = builder;
                #(#response_data_fields)*
                Ok(builder)
            }

            fn get_message_template(&self) -> crate::error::Result<&str> {
                #get_template_impl
            }
        }

        #brp_impls

        #message_template_impl
    };

    TokenStream::from(expanded)
}

/// Generate the `get_message_template` implementation body.
fn generate_get_template_impl(
    fields: &[&syn::Field],
    message_template_field: Option<&(syn::Ident, Option<String>)>,
) -> proc_macro2::TokenStream {
    if let Some((field_name, _)) = message_template_field {
        let message_field_type = fields
            .iter()
            .find(|f| f.ident.as_ref() == Some(field_name))
            .map(|f| &f.ty);

        let is_option_type =
            message_field_type.is_some_and(|ty| quote!(#ty).to_string().contains("Option <"));

        if is_option_type {
            quote! {
                self.#field_name.as_ref()
                    .map(|s| s.as_str())
                    .ok_or_else(|| {
                        error_stack::Report::new(crate::error::Error::MissingMessageTemplate(
                            "Message template not set. Use .with_message_template() to provide a template.".to_string()
                        ))
                    })
            }
        } else {
            quote! {
                Ok(self.#field_name.as_str())
            }
        }
    } else {
        quote! {
            Err(error_stack::Report::new(crate::error::Error::MissingMessageTemplate(
                "No message template field defined".to_string()
            )))
        }
    }
}

/// Generate BRP-specific trait implementations (`BrpToolConfig`, `ResultStructBrpExt`,
/// `from_brp_client_response`).
fn generate_brp_trait_impls(
    struct_name: &syn::Ident,
    brp_attrs: Option<&BrpResultAttrs>,
    regular_fields: &[(syn::Ident, syn::Type)],
    computed_fields: &[ComputedField],
    message_template_field: Option<&(syn::Ident, Option<String>)>,
) -> proc_macro2::TokenStream {
    let from_brp_client_response_impl = if brp_attrs.is_some() {
        generate_from_brp_client_response(
            struct_name,
            regular_fields,
            computed_fields,
            message_template_field,
        )
    } else {
        quote! {}
    };

    let brp_tool_config_impl = if let Some(attrs) = brp_attrs {
        let enhanced_errors = attrs.enhanced_errors;
        quote! {
            impl crate::brp_tools::BrpToolConfig for #struct_name {
                const ADD_TYPE_GUIDE_TO_ERROR: bool = #enhanced_errors;
            }
        }
    } else {
        quote! {}
    };

    let result_struct_brp_ext_impl = if brp_attrs.is_some() {
        quote! {
            impl crate::brp_tools::ResultStructBrpExt for #struct_name {
                type Args = (
                    Option<serde_json::Value>,
                    Option<Vec<serde_json::Value>>,
                    Option<crate::brp_tools::FormatCorrectionStatus>,
                );

                fn from_brp_client_response(args: Self::Args) -> crate::error::Result<Self> {
                    Self::from_brp_client_response(args.0, args.1, args.2)
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #from_brp_client_response_impl
        #brp_tool_config_impl
        #result_struct_brp_ext_impl
    }
}

/// Generate `MessageTemplateProvider` implementation and constructor methods
fn generate_message_template_provider(
    struct_name: &syn::Ident,
    message_template_field: Option<&(syn::Ident, Option<String>)>,
    regular_fields: &[(syn::Ident, syn::Type)],
    computed_fields: &[ComputedField],
) -> proc_macro2::TokenStream {
    let Some((field_name, default_template)) = message_template_field else {
        return quote! {};
    };

    let constructor_params: Vec<_> = regular_fields
        .iter()
        .filter(|(name, _)| name != field_name)
        .map(|(name, ty)| quote! { #name: #ty })
        .collect();

    let field_initializers = build_constructor_initializers(
        field_name,
        default_template.as_deref(),
        regular_fields,
        computed_fields,
    );

    let is_option_type = is_option_message_field(field_name, regular_fields);

    let with_template_impl = if is_option_type {
        quote! { self.#field_name = Some(template.into()); }
    } else {
        quote! { self.#field_name = template.into(); }
    };

    if is_option_type && default_template.is_none() {
        generate_builder_pattern(
            struct_name,
            field_name,
            &constructor_params,
            &with_template_impl,
            regular_fields,
            computed_fields,
        )
    } else {
        generate_direct_constructor(
            struct_name,
            &constructor_params,
            &field_initializers,
            &with_template_impl,
        )
    }
}

/// Build field initializers for the constructor.
fn build_constructor_initializers(
    template_field_name: &syn::Ident,
    default_template: Option<&str>,
    regular_fields: &[(syn::Ident, syn::Type)],
    computed_fields: &[ComputedField],
) -> Vec<proc_macro2::TokenStream> {
    let mut initializers = Vec::new();

    for (name, ty) in regular_fields {
        if name == template_field_name {
            initializers.push(template_field_initializer(name, ty, default_template));
        } else {
            initializers.push(quote! { #name });
        }
    }

    for computed in computed_fields {
        let field_name = &computed.field_name;
        let default_value = computed_field_default(&computed.operation);
        initializers.push(quote! { #field_name: #default_value });
    }

    initializers
}

/// Generate the initializer for the message template field.
fn template_field_initializer(
    name: &syn::Ident,
    ty: &syn::Type,
    default_template: Option<&str>,
) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();
    let is_option = type_str.contains("Option <");

    if let Some(template) = default_template {
        let converted = convert_template_braces(template);
        if is_option {
            quote! { #name: Some(#converted.to_string()) }
        } else {
            quote! { #name: #converted.to_string() }
        }
    } else if is_option {
        quote! { #name: None }
    } else {
        panic!("Message template field must be Option<String> when no default template is provided")
    }
}

/// Check whether the message template field is `Option<String>`.
fn is_option_message_field(
    field_name: &syn::Ident,
    regular_fields: &[(syn::Ident, syn::Type)],
) -> bool {
    regular_fields
        .iter()
        .find(|(name, _)| name == field_name)
        .map(|(_, ty)| ty)
        .is_some_and(|ty| quote!(#ty).to_string().contains("Option <"))
}

/// Generate a builder-pattern constructor for `Option<String>` template fields without defaults.
fn generate_builder_pattern(
    struct_name: &syn::Ident,
    field_name: &syn::Ident,
    constructor_params: &[proc_macro2::TokenStream],
    with_template_impl: &proc_macro2::TokenStream,
    regular_fields: &[(syn::Ident, syn::Type)],
    computed_fields: &[ComputedField],
) -> proc_macro2::TokenStream {
    let builder_name = quote::format_ident!("{}Builder", struct_name);

    let field_names: Vec<_> = regular_fields
        .iter()
        .filter(|(name, _)| name != field_name)
        .map(|(name, _)| name.clone())
        .collect();

    let builder_fields: Vec<_> = regular_fields
        .iter()
        .filter(|(name, _)| name != field_name)
        .map(|(name, ty)| quote! { #name: #ty })
        .collect();

    let mut builder_to_struct_initializers = Vec::new();
    for (name, _) in regular_fields {
        if name != field_name {
            builder_to_struct_initializers.push(quote! { #name: self.#name });
        }
    }
    for computed in computed_fields {
        let cfield = &computed.field_name;
        let default_value = computed_field_default(&computed.operation);
        builder_to_struct_initializers.push(quote! { #cfield: #default_value });
    }

    quote! {
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
            #[must_use = "This returns a builder that must be completed with .with_message_template()"]
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
}

/// Generate a direct constructor (no builder) when a default template exists.
fn generate_direct_constructor(
    struct_name: &syn::Ident,
    constructor_params: &[proc_macro2::TokenStream],
    field_initializers: &[proc_macro2::TokenStream],
    with_template_impl: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
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

/// Generate `from_brp_client_response` method
fn generate_from_brp_client_response(
    struct_name: &syn::Ident,
    regular_fields: &[(syn::Ident, syn::Type)],
    computed_fields: &[ComputedField],
    message_template_field: Option<&(syn::Ident, Option<String>)>,
) -> proc_macro2::TokenStream {
    let mut field_initializers = Vec::new();

    for (field_name, field_type) in regular_fields {
        if let Some(init) =
            generate_regular_field_initializer(field_name, field_type, message_template_field)
        {
            field_initializers.push(init);
        }
    }

    for computed in computed_fields {
        field_initializers.push(generate_computed_field_initializer(computed));
    }

    let params = quote! {
        value: Option<serde_json::Value>,
        format_corrections: Option<Vec<serde_json::Value>>,
        format_corrected: Option<crate::brp_tools::FormatCorrectionStatus>,
    };

    quote! {
        impl #struct_name {
            /// Create from BRP response value
            pub fn from_brp_client_response(#params) -> crate::error::Result<Self> {
                Ok(Self {
                    #(#field_initializers,)*
                })
            }
        }

        // Note: ResultStructBrpExt implementation is now generated separately above
    }
}

/// Generate the initializer for a single regular field in `from_brp_client_response`.
fn generate_regular_field_initializer(
    field_name: &syn::Ident,
    field_type: &syn::Type,
    message_template_field: Option<&(syn::Ident, Option<String>)>,
) -> Option<proc_macro2::TokenStream> {
    let type_str = quote!(#field_type).to_string();

    if field_name == "result" && type_str.contains("Option < Value >") {
        Some(quote! { result: value.clone() })
    } else if field_name == "format_corrections" {
        Some(quote! {
            format_corrections: if format_corrections.as_ref().map_or(true, |v| v.is_empty()) {
                None
            } else {
                format_corrections.clone()
            }
        })
    } else if field_name == "format_corrected" {
        Some(quote! {
            format_corrected: match format_corrected {
                Some(crate::brp_tools::FormatCorrectionStatus::NotAttempted) | None => None,
                other => other,
            }
        })
    } else if field_name == "warning" && type_str.contains("Option < String >") {
        Some(quote! {
            warning: format_corrections.as_ref().and_then(|corrections| {
                if corrections.is_empty() {
                    None
                } else {
                    Some(format!(
                        "Operation succeeded with {} format correction(s) applied. See format_corrections field for details.",
                        corrections.len()
                    ))
                }
            })
        })
    } else if let Some((template_field_name, template_default)) = message_template_field
        && field_name == template_field_name
    {
        Some(template_field_initializer(
            field_name,
            field_type,
            template_default.as_deref(),
        ))
    } else {
        None
    }
}

/// Generate the initializer for a single computed field in `from_brp_client_response`.
fn generate_computed_field_initializer(computed: &ComputedField) -> proc_macro2::TokenStream {
    let field_name = &computed.field_name;
    let from_field = &computed.from_field;
    let operation = &computed.operation;

    let source = if from_field == "result" {
        quote! { value }
    } else {
        let from_ident = syn::Ident::new(from_field, field_name.span());
        quote! { #from_ident }
    };

    let computation = generate_computation(&source, operation);
    quote! { #field_name: #computation }
}

/// Generate the token stream for a computed field operation.
fn generate_computation(
    source: &proc_macro2::TokenStream,
    operation: &str,
) -> proc_macro2::TokenStream {
    if let Some(tokens) = generate_count_computation(source, operation) {
        return tokens;
    }
    if let Some(tokens) = generate_extract_computation(source, operation) {
        return tokens;
    }
    panic!("Unknown computed operation: {operation}")
}

/// Generate token streams for `count_*` operations.
///
/// Returns `None` if the operation is not a count variant.
fn generate_count_computation(
    source: &proc_macro2::TokenStream,
    operation: &str,
) -> Option<proc_macro2::TokenStream> {
    let tokens = match operation {
        "count" => quote! {
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
        },
        "count_type_guide" => quote! {
            #source.as_ref()
                .and_then(|v| v.get("type_guide"))
                .and_then(|v| v.as_object())
                .map(|obj| obj.len())
                .unwrap_or(0)
        },
        "count_components" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .map(|obj| {
                    if let Some(components) = obj.get("components").and_then(|v| v.as_object()) {
                        components.len()
                    } else {
                        obj.iter()
                            .filter(|(key, _)| key.as_str() != "errors")
                            .count()
                    }
                })
                .unwrap_or(0)
        },
        "count_errors" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("errors"))
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
        },
        "count_query_components" => quote! {
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
        },
        "count_methods" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("methods"))
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0)
        },
        "count_keys_sent" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("keys_sent"))
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0)
        },
        _ => return None,
    };
    Some(tokens)
}

/// Generate token streams for `extract_*` operations.
///
/// Returns `None` if the operation is not an extract variant.
fn generate_extract_computation(
    source: &proc_macro2::TokenStream,
    operation: &str,
) -> Option<proc_macro2::TokenStream> {
    let tokens = match operation {
        "extract_entity" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("entity"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        },
        "extract_keys_sent" => quote! {
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
        },
        "extract_duration_ms" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("duration_ms"))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(100)
        },
        "extract_debug_enabled" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("debug_enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        },
        "extract_message" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("message"))
                .and_then(|v| v.as_str())
                .map(String::from)
        },
        "extract_status" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("status"))
                .and_then(|v| v.as_str())
                .map_or_else(|| "unknown".to_string(), String::from)
        },
        "extract_old_title" | "extract_new_title" => {
            let json_key = operation
                .strip_prefix("extract_")
                .expect("has extract_ prefix");
            quote! {
                #source.as_ref()
                    .and_then(|v| v.as_object())
                    .and_then(|obj| obj.get(#json_key))
                    .and_then(|v| v.as_str())
                    .map_or_else(String::new, String::from)
            }
        },
        "extract_chars_queued" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("chars_queued"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(0)
        },
        "extract_skipped" => quote! {
            #source.as_ref()
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("skipped"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().and_then(|s| s.chars().next()))
                        .collect()
                })
                .unwrap_or_else(Vec::new)
        },
        _ => return None,
    };
    Some(tokens)
}

/// Default value for a computed field operation used in constructors and builders.
fn computed_field_default(operation: &str) -> proc_macro2::TokenStream {
    match operation {
        "count"
        | "count_type_info"
        | "count_components"
        | "count_methods"
        | "count_query_components"
        | "count_keys_sent"
        | "extract_chars_queued" => {
            quote! { 0 }
        },
        "extract_entity" => quote! { 0 },
        "extract_duration_ms" => quote! { 100 },
        "count_errors" => quote! { None },
        "extract_keys_sent" | "extract_skipped" => quote! { Vec::new() },
        "extract_debug_enabled" => quote! { false },
        "extract_message" | "extract_status" | "extract_old_title" | "extract_new_title" => {
            quote! { String::new() }
        },
        _ => quote! { Default::default() },
    }
}
