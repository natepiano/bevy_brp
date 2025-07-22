//! Template substitution utilities for response message formatting.
//!
//! This module provides clean template substitution with prioritized value lookup:
//! 1. Configured fields (`ResponseField` extractions) - highest priority
//! 2. Request arguments - medium priority
//! 3. Raw response data - fallback priority - for fields that aren't configured in the
//!    `ResponseDef` but still have a template placeholder defined

use serde_json::Value;

use super::components::ConfiguredField;
use crate::tool::HandlerContext;

/// Parse template to find placeholder names (e.g., "{entity}" -> "entity")
fn parse_template_placeholders(template: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut remaining = template;

    while let Some(start) = remaining.find('{') {
        if let Some(end) = remaining[start + 1..].find('}') {
            let placeholder = &remaining[start + 1..start + 1 + end];
            if !placeholder.is_empty() {
                placeholders.push(placeholder.to_string());
            }
            remaining = &remaining[start + 1 + end + 1..];
        } else {
            break;
        }
    }

    placeholders
}

/// Substitute template with priority search: `configured_fields` -> `request_args` -> `raw_data`
pub fn substitute_template_with_priority(
    template: &str,
    configured_fields: &[ConfiguredField],
    handler_context: &HandlerContext,
    clean_data: &Value,
) -> String {
    let placeholders = parse_template_placeholders(template);
    let mut result = template.to_string();

    for placeholder in placeholders {
        let replacement_value =
            find_placeholder_value(&placeholder, configured_fields, handler_context, clean_data);

        if let Some(value) = replacement_value {
            let placeholder_str = format!("{{{placeholder}}}");
            let replacement = value_to_string(&value);
            result = result.replace(&placeholder_str, &replacement);
        }
    }

    result
}

/// Find value for placeholder in priority order: `configured_fields` -> `request_args` ->
/// `raw_data`
fn find_placeholder_value(
    placeholder: &str,
    configured_fields: &[ConfiguredField],
    handler_context: &HandlerContext,
    clean_data: &Value,
) -> Option<Value> {
    // 1. Check configured fields first (highest priority)
    for field in configured_fields {
        if field.name == placeholder {
            return Some(field.value.clone());
        }
    }

    // 2. Check request arguments
    if let Some(args) = &handler_context.request.arguments {
        if let Some(value) = args.get(placeholder) {
            return Some(value.clone());
        }
    }

    // 3. Check raw response data (fallback)
    if let Value::Object(data_map) = clean_data {
        if let Some(value) = data_map.get(placeholder) {
            return Some(value.clone());
        }
    }

    None
}

/// Convert Value to string for template substitution
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}
