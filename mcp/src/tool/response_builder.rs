use serde::Serialize;
use serde_json::Value;

use super::HandlerContext;
use super::ParamStruct;
use super::ResultStruct;
use super::field_placement::FieldPlacement;
use super::json_response::ResponseStatus;
use super::json_response::ToolCallJsonResponse;
use super::tool_name::CallInfo;
use crate::error::Error;
use crate::error::Result;

/// High-level response creation API
///
/// This provides a cleaner, more ergonomic interface for creating responses
/// compared to using `ResponseBuilder` directly.
pub struct Response;

impl Response {
    /// Create a success response from a `ResultStruct`
    pub fn success<R: ResultStruct + ?Sized, P: ParamStruct>(
        result: &R,
        params: Option<P>,
        call_info: CallInfo,
        context: &HandlerContext,
    ) -> Result<ToolCallJsonResponse> {
        ResponseBuilder::success(call_info).build_with_result_struct(result, params, context)
    }

    /// Create an error response from a `ResultStruct`
    pub fn error<R: ResultStruct + ?Sized, P: ParamStruct>(
        error_result: &R,
        params: Option<P>,
        call_info: CallInfo,
        context: &HandlerContext,
    ) -> Result<ToolCallJsonResponse> {
        ResponseBuilder::error(call_info).build_with_result_struct(error_result, params, context)
    }

    /// Create a simple error response with just a message
    pub fn error_message(message: impl Into<String>, call_info: CallInfo) -> ToolCallJsonResponse {
        ResponseBuilder::error(call_info).message(message).build()
    }

    /// Create an error response with message and optional details
    pub fn error_with_details(
        message: impl Into<String>,
        details: Option<&Value>,
        call_info: CallInfo,
    ) -> ToolCallJsonResponse {
        ResponseBuilder::error(call_info)
            .message(message)
            .add_optional_details(details)
            .build()
    }
}

/// Builder for constructing JSON responses
#[derive(Clone)]
pub struct ResponseBuilder {
    status:                ResponseStatus,
    message:               String,
    call_info:             CallInfo,
    metadata:              Option<Value>,
    parameters:            Option<Value>,
    result:                Option<Value>,
    error_info:            Option<Value>,
    brp_extras_debug_info: Option<Value>,
}

impl ResponseBuilder {
    /// Create a success response with call info pre-populated
    pub const fn success(call_info: CallInfo) -> Self {
        Self {
            status: ResponseStatus::Success,
            message: String::new(),
            call_info,
            metadata: None,
            parameters: None,
            result: None,
            error_info: None,
            brp_extras_debug_info: None,
        }
    }

    /// Create an error response with call info pre-populated
    pub const fn error(call_info: CallInfo) -> Self {
        Self {
            status: ResponseStatus::Error,
            message: String::new(),
            call_info,
            metadata: None,
            parameters: None,
            result: None,
            error_info: None,
            brp_extras_debug_info: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Add a field to the metadata object. Creates a new object if metadata is None.
    pub fn add_field(mut self, key: &str, value: impl Serialize) -> Result<Self> {
        use error_stack::ResultExt;

        let value_json = serde_json::to_value(value)
            .change_context(Error::General(format!("Failed to serialize field '{key}'")))?;

        // Skip fields marked for nullable skipping
        if let Value::String(s) = &value_json
            && s == "__SKIP_NULL_FIELD__"
        {
            return Ok(self);
        }

        if let Some(Value::Object(map)) = &mut self.metadata {
            map.insert(key.to_string(), value_json);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value_json);
            self.metadata = Some(Value::Object(map));
        }

        Ok(self)
    }

    /// Add multiple fields from an optional JSON object to metadata
    /// Useful for adding error details or other optional metadata
    pub fn add_optional_details(self, details: Option<&serde_json::Value>) -> Self {
        match details {
            Some(Value::Object(map)) => {
                map.iter()
                    .filter(|(_, v)| !v.is_null())
                    .fold(self, |builder, (key, value)| {
                        builder.clone().add_field(key, value).unwrap_or_else(|_| {
                            tracing::warn!("Failed to add detail field '{}'", key);
                            builder // Keep the original builder if add_field fails
                        })
                    })
            }
            _ => self,
        }
    }

    /// Add a field to the specified location (metadata or result object)
    pub fn add_field_to(
        mut self,
        key: &str,
        value: impl Serialize,
        placement: FieldPlacement,
    ) -> Result<Self> {
        use error_stack::ResultExt;

        let value_json = serde_json::to_value(value)
            .change_context(Error::General(format!("Failed to serialize field '{key}'")))?;

        // Skip fields marked for nullable skipping
        if let Value::String(s) = &value_json
            && s == "__SKIP_NULL_FIELD__"
        {
            return Ok(self);
        }

        match placement {
            FieldPlacement::Metadata => {
                // For metadata, use field name as key in object
                if let Some(Value::Object(map)) = &mut self.metadata {
                    map.insert(key.to_string(), value_json);
                } else {
                    let mut map = serde_json::Map::new();
                    map.insert(key.to_string(), value_json);
                    self.metadata = Some(Value::Object(map));
                }
            }
            FieldPlacement::Result => {
                // For result, set the entire result field to the value
                // Field name is ignored to match raw BRP behavior
                self.result = Some(value_json);
            }
            FieldPlacement::ErrorInfo => {
                // For error_info, use field name as key in object
                if let Some(Value::Object(map)) = &mut self.error_info {
                    map.insert(key.to_string(), value_json);
                } else {
                    let mut map = serde_json::Map::new();
                    map.insert(key.to_string(), value_json);
                    self.error_info = Some(Value::Object(map));
                }
            }
        }

        Ok(self)
    }

    pub fn build(self) -> ToolCallJsonResponse {
        ToolCallJsonResponse {
            status:                self.status,
            message:               self.message,
            call_info:             self.call_info,
            metadata:              self.metadata,
            parameters:            self.parameters,
            result:                self.result,
            error_info:            self.error_info,
            brp_extras_debug_info: self.brp_extras_debug_info,
        }
    }

    /// Get metadata for template substitution
    pub const fn metadata(&self) -> Option<&Value> {
        self.metadata.as_ref()
    }

    /// Get result for template substitution
    pub const fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    /// Set parameters with optional parameter tracking
    pub fn parameters(mut self, params: impl Serialize) -> Result<Self> {
        use error_stack::ResultExt;

        let mut params_value = serde_json::to_value(params)
            .change_context(Error::General("Failed to serialize parameters".to_string()))?;

        // Extract optional parameters that were not provided
        if let Value::Object(ref mut params_obj) = params_value {
            let mut optional_not_provided = Vec::new();

            // Collect keys that have null values (optional parameters not provided)
            let null_keys: Vec<String> = params_obj
                .iter()
                .filter_map(|(key, value)| {
                    if value.is_null() {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Remove null values from the main parameters object
            for key in &null_keys {
                params_obj.remove(key);
                optional_not_provided.push(key.clone());
            }

            // Add the optional_parameters_not_provided array if there are any
            if !optional_not_provided.is_empty() {
                params_obj.insert(
                    "optional_parameters_not_provided".to_string(),
                    Value::Array(
                        optional_not_provided
                            .into_iter()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
        }

        self.parameters = Some(params_value);
        Ok(self)
    }

    /// Get parameters for template substitution
    pub const fn parameters_ref(&self) -> Option<&Value> {
        self.parameters.as_ref()
    }

    /// Terminal operation: Build complete response from a `ResultStruct`, handling all formatting
    /// and template substitution
    pub fn build_with_result_struct<R: ResultStruct + ?Sized, P: ParamStruct>(
        mut self,
        result: &R,
        params: Option<P>,
        handler_context: &HandlerContext,
    ) -> Result<ToolCallJsonResponse> {
        // Add response fields
        self = result
            .add_response_fields(self)
            .map_err(|e| Error::failed_to("add response fields", e))?;

        // Add parameters if present
        if let Some(params) = params {
            self = self.parameters(params)?;
        }

        // Perform template substitution
        let template_str = result.get_message_template()?;
        tracing::debug!("Template before substitution: '{}'", template_str);
        let message = Self::substitute_dynamic_template(template_str, &self, handler_context);
        tracing::debug!("Template after substitution: '{}'", message);
        self = self.message(message);

        Ok(self.build())
    }

    /// Substitute template placeholders with values from the builder using dynamic template string
    fn substitute_dynamic_template(
        template_str: &str,
        builder: &Self,
        handler_context: &HandlerContext,
    ) -> String {
        let mut result = template_str.to_string();

        // Extract placeholders from template
        let placeholders = Self::parse_template_placeholders(&result);

        for placeholder in placeholders {
            if let Some(replacement) =
                Self::find_placeholder_value(&placeholder, builder, handler_context)
            {
                let placeholder_str = format!("{{{{{placeholder}}}}}");
                result = result.replace(&placeholder_str, &replacement);
            }
        }

        result
    }

    /// Parse template to find placeholder names
    fn parse_template_placeholders(template: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut remaining = template;

        while let Some(start) = remaining.find("{{") {
            if let Some(end) = remaining[start + 2..].find("}}") {
                let placeholder = &remaining[start + 2..start + 2 + end];
                if !placeholder.is_empty() && !placeholder.contains('{') {
                    placeholders.push(placeholder.to_string());
                }
                remaining = &remaining[start + 2 + end + 2..];
            } else {
                break;
            }
        }

        placeholders
    }

    /// Find value for a placeholder
    fn find_placeholder_value(
        placeholder: &str,
        builder: &Self,
        handler_context: &HandlerContext,
    ) -> Option<String> {
        tracing::debug!("Looking for placeholder: '{}'", placeholder);
        // First check error_info (for error fields)
        if let Some(Value::Object(error_info)) = &builder.error_info {
            tracing::debug!(
                "Error info contains: {:?}",
                error_info.keys().collect::<Vec<_>>()
            );
            if let Some(value) = error_info.get(placeholder) {
                let result = Self::value_to_string(value);
                tracing::debug!("Found '{}' in error_info: '{}'", placeholder, result);
                return Some(result);
            }
        }

        // Then check metadata
        if let Some(Value::Object(metadata)) = builder.metadata()
            && let Some(value) = metadata.get(placeholder)
        {
            return Some(Self::value_to_string(value));
        }

        // Then check result if placeholder is "result"
        if placeholder == "result"
            && let Some(result_value) = builder.result()
        {
            return Some(Self::value_to_string(result_value));
        }

        // Check parameters added to the builder
        if let Some(Value::Object(params_obj)) = builder.parameters_ref() {
            // Special handling for entity_count from entities array
            if placeholder == "entity_count"
                && let Some(Value::Array(entities)) = params_obj.get("entities")
            {
                return Some(entities.len().to_string());
            }

            // Regular parameter lookup
            if let Some(value) = params_obj.get(placeholder) {
                return Some(Self::value_to_string(value));
            }
        }

        // Finally check request parameters
        if let Some(value) = handler_context.extract_optional_named_field(placeholder) {
            return Some(Self::value_to_string(value));
        }

        None
    }

    /// Convert value to string for template substitution
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => format!("{} items", arr.len()),
            _ => value.to_string(),
        }
    }
}
