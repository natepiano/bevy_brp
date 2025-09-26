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

// Internal result type for mutation path building
// This type alias is used by all internal builders
pub(super) type MutationResult = std::result::Result<Vec<MutationPathInternal>, NotMutableReason>;
