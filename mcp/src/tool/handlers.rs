use crate::response::{self, ResponseFormatterFactory, ResponseSpecification};

/// Build a formatter factory from a tool definition's response specification
pub fn build_formatter_factory_from_spec(
    formatter_spec: &ResponseSpecification,
) -> ResponseFormatterFactory {
    // Create the formatter factory for structured responses
    let mut formatter_builder = ResponseFormatterFactory::standard();

    // Set the template if provided
    let template = formatter_spec.message_template;
    if !template.is_empty() {
        formatter_builder = formatter_builder.with_template(template);
    }

    // Add response fields
    for field in &formatter_spec.response_fields {
        let (extractor, placement) = response::convert_response_field(field);
        formatter_builder =
            formatter_builder.with_response_field_placed(field.name(), extractor, placement);
    }

    formatter_builder.build()
}
