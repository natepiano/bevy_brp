mod enum_path_builder;
mod variant_kind;

use super::BuilderError;
pub(super) use super::enum_path_info::EnumPathInfo;
pub(super) use super::example_group::ExampleGroup;
use super::mutation_path_internal::MutationPathInternal;
use super::path_example::Example;
use super::recursion_context::RecursionContext;

pub(super) fn process_enum(
    context: &RecursionContext,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    enum_path_builder::process_enum(context)
}

pub(super) fn select_preferred_example(examples: &[ExampleGroup]) -> Option<Example> {
    enum_path_builder::select_preferred_example(examples)
}
