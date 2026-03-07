mod enum_path_builder;
mod variant_kind;

use super::BuilderError;
use super::mutation_path_internal::MutationPathInternal;
use super::recursion_context::RecursionContext;
use super::types::Example;
use super::types::ExampleGroup;

pub(super) fn process_enum(
    ctx: &RecursionContext,
) -> std::result::Result<Vec<MutationPathInternal>, BuilderError> {
    enum_path_builder::process_enum(ctx)
}

pub(super) fn select_preferred_example(examples: &[ExampleGroup]) -> Option<Example> {
    enum_path_builder::select_preferred_example(examples)
}
