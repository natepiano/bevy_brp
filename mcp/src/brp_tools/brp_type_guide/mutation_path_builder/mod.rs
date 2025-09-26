mod builder;
mod builders;
mod enum_path_builder;
mod mutation_knowledge;
mod not_mutable_reason;
mod path_builder;
mod path_kind;
mod recursion_context;
mod type_parser;
mod types;

pub use builder::recurse_mutation_paths;
pub use enum_path_builder::select_preferred_example;
pub use mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey, MutationKnowledge};
pub use not_mutable_reason::NotMutableReason;
pub use path_kind::{MutationPathDescriptor, PathKind};
pub use recursion_context::RecursionContext;
pub use types::{MutationPath, MutationPathInternal, MutationStatus};

/// Internal result type for mutation path building using structured control flow.
///
/// This type alias enables clean error handling where `NotMutableReason` represents expected
/// "not mutable" conditions rather than actual errors. Internal builders return this type,
/// and `NotMutableReason` values get converted to user-facing output at the choke point in
/// `recurse_mutation_paths()` via `build_not_mutable_path()`.
///
/// This design separates genuine system errors (which bubble up as `Result<T, Error>`) from
/// expected mutation limitations that are part of normal type analysis.
pub(super) type MutationResult = std::result::Result<Vec<MutationPathInternal>, NotMutableReason>;
