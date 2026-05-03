//! Shared utilities for field placement macros

use proc_macro2::TokenStream;
use quote::quote;
use syn::Attribute;
use syn::Field;
use syn::Ident;
use syn::LitStr;
use syn::Type;
use syn::meta::ParseNestedMeta;

/// Information about a computed field
pub(crate) struct ComputedField {
    pub field_name: Ident,
    pub from_field: String,
    pub operation:  String,
}

#[derive(Clone, Copy)]
enum PlacementAttrKey {
    FieldType,
    From,
    ResultOperation,
    SkipIfNone,
}

impl PlacementAttrKey {
    fn parse(meta: &ParseNestedMeta<'_>) -> syn::Result<Self> {
        [
            ("field_type", Self::FieldType),
            ("from", Self::From),
            ("result_operation", Self::ResultOperation),
            ("skip_if_none", Self::SkipIfNone),
        ]
        .into_iter()
        .find_map(|(ident, key)| meta.path.is_ident(ident).then_some(key))
        .ok_or_else(|| meta.error("unsupported attribute"))
    }
}

#[derive(Clone, Copy)]
enum PlacementKind {
    ErrorInfo,
    Metadata,
    Result,
}

impl PlacementKind {
    fn tokens(self) -> TokenStream {
        match self {
            Self::ErrorInfo => quote! { crate::tool::FieldPlacement::ErrorInfo },
            Self::Metadata => quote! { crate::tool::FieldPlacement::Metadata },
            Self::Result => quote! { crate::tool::FieldPlacement::Result },
        }
    }
}

#[derive(Clone, Copy)]
enum FieldAttributeKind {
    CallInfo,
    Computed,
    Message,
    Placement(PlacementKind),
}

impl FieldAttributeKind {
    fn parse(attr: &Attribute) -> Option<Self> {
        [
            ("computed", Self::Computed),
            ("to_call_info", Self::CallInfo),
            ("to_error_info", Self::Placement(PlacementKind::ErrorInfo)),
            ("to_message", Self::Message),
            ("to_metadata", Self::Placement(PlacementKind::Metadata)),
            ("to_result", Self::Placement(PlacementKind::Result)),
        ]
        .into_iter()
        .find_map(|(ident, kind)| attr.path().is_ident(ident).then_some(kind))
    }
}

/// Parse placement attribute arguments
pub(crate) fn parse_placement_attr(
    attr: &Attribute,
    source_path: &mut Option<String>,
    field_type: &mut Option<String>,
    skip_if_none: &mut bool,
    result_operation: &mut Option<String>,
) {
    let _ = attr.parse_nested_meta(|meta| match PlacementAttrKey::parse(&meta)? {
        PlacementAttrKey::From => {
            let value = meta.value()?;
            let string: LitStr = value.parse()?;
            *source_path = Some(string.value());
            Ok(())
        },
        PlacementAttrKey::FieldType => {
            let value = meta.value()?;
            let string: LitStr = value.parse()?;
            *field_type = Some(string.value());
            Ok(())
        },
        PlacementAttrKey::SkipIfNone => {
            *skip_if_none = true;
            Ok(())
        },
        PlacementAttrKey::ResultOperation => {
            let value = meta.value()?;
            let string: LitStr = value.parse()?;
            *result_operation = Some(string.value());
            Ok(())
        },
    });
}

/// Parse computed attribute arguments
pub(crate) fn parse_computed_attr(attr: &Attribute, result_operation: &mut Option<String>) {
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("operation") {
            let value = meta.value()?;
            let string: LitStr = value.parse()?;
            *result_operation = Some(string.value());
            Ok(())
        } else {
            Err(meta.error("unsupported computed attribute"))
        }
    });
}

/// Parse `to_message` attribute arguments
pub(crate) fn parse_to_message_attr(attr: &Attribute) -> Option<String> {
    let mut message_template = None;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("message_template") {
            let value = meta.value()?;
            let string: LitStr = value.parse()?;
            message_template = Some(string.value());
            Ok(())
        } else {
            Err(meta.error("unsupported to_message attribute"))
        }
    });

    message_template
}

/// Generate response data field addition
pub(crate) fn generate_response_data_field(
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

/// Extract field data from struct fields
pub(crate) fn extract_field_data(fields: &[&Field]) -> FieldExtractionResult {
    let mut field_placements = Vec::new();
    let mut response_data_fields = Vec::new();
    let mut computed_fields = Vec::new();
    let mut regular_fields = Vec::new();
    let mut message_template_field: Option<(Ident, Option<String>)> = None;

    for field in fields {
        let field_name = field.ident.as_ref().expect("Only works with named fields");
        let field_type = &field.ty;

        // Check for our placement attributes
        let mut placement = None;
        let mut source_path = None;
        let mut field_type_override = None;
        let mut skip_if_none = false;
        let mut is_computed = false;
        let mut result_operation = None;

        for attr in &field.attrs {
            let Some(attr_kind) = FieldAttributeKind::parse(attr) else {
                continue;
            };

            match attr_kind {
                FieldAttributeKind::Placement(placement_kind) => {
                    placement = Some(placement_kind.tokens());
                    parse_placement_attr(
                        attr,
                        &mut source_path,
                        &mut field_type_override,
                        &mut skip_if_none,
                        &mut result_operation,
                    );
                },
                FieldAttributeKind::CallInfo => {
                    // Skip fields marked with to_call_info as we no longer need them
                },
                FieldAttributeKind::Computed => {
                    is_computed = true;
                    parse_computed_attr(attr, &mut result_operation);
                },
                FieldAttributeKind::Message => {
                    let template = parse_to_message_attr(attr);
                    message_template_field = Some((field_name.clone(), template));
                    // Skip adding to other collections
                },
            }
        }

        // If we found result_operation in placement attrs, mark as computed
        if result_operation.is_some() {
            is_computed = true;
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
                .map_or_else(|| quote! { None }, |s| quote! { Some(#s) });

            field_placements.push(quote! {
                crate::tool::FieldPlacementInfo {
                    field_name: #field_name_str,
                    placement: #placement,
                    source_path: #source_path_token,
                    skip_if_none: crate::tool::SkipIfNone::from(#skip_if_none),
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
        computed_fields,
        regular_fields,
        message_template_field,
    }
}

pub(crate) struct FieldExtractionResult {
    pub field_placements:       Vec<TokenStream>,
    pub response_data_fields:   Vec<TokenStream>,
    pub computed_fields:        Vec<ComputedField>,
    pub regular_fields:         Vec<(Ident, Type)>,
    pub message_template_field: Option<(Ident, Option<String>)>,
}
