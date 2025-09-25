//! Standalone enum path builder - no PathBuilder dependency

use std::collections::HashMap;

use serde_json::{Value, json};

use super::mutation_path_builder::builders::enum_builder::{
    build_enum_examples, extract_and_group_variants,
};
use super::mutation_path_builder::{
    EnumContext, ExampleGroup, MutationPathDescriptor, MutationPathInternal, MutationStatus,
    PathAction, PathKind, RecursionContext, TypeKind, VariantName, VariantPath,
};
use crate::brp_tools::brp_type_guide::constants::RecursionDepth;
use crate::error::Result;

/// Standalone enum path builder - no PathBuilder dependency
pub struct EnumPathBuilder;

impl EnumPathBuilder {
    /// Process enum type directly, bypassing PathBuilder trait
    /// Uses the same shared functions as EnumMutationBuilder for identical output
    pub fn process_enum(
        &self,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Vec<MutationPathInternal>> {
        tracing::debug!("EnumPathBuilder processing type: {}", ctx.type_name());

        // Use shared function to get variant information - same as EnumMutationBuilder
        let variant_groups = extract_and_group_variants(ctx)?;

        // Process children and collect BOTH examples AND child paths
        let (child_examples, child_paths) = self.process_children(&variant_groups, ctx, depth)?;

        // Use shared function to build examples - same as EnumMutationBuilder
        let assembled_value = build_enum_examples(&variant_groups, child_examples, ctx)?;

        // Create result paths including both root AND child paths
        self.create_result_paths(ctx, assembled_value, child_paths)
    }

    /// Process child paths - simplified version of MutationPathBuilder's child processing
    fn process_children(
        &self,
        variant_groups: &HashMap<
            super::mutation_path_builder::types::VariantSignature,
            Vec<super::mutation_path_builder::builders::enum_builder::EnumVariantInfo>,
        >,
        ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<(
        HashMap<MutationPathDescriptor, Value>,
        Vec<MutationPathInternal>,
    )> {
        let mut child_examples = HashMap::new();
        let mut all_child_paths = Vec::new();

        // Process each variant group (same logic as EnumMutationBuilder::collect_children)
        for (signature, variants_in_group) in variant_groups {
            let applicable_variants: Vec<VariantName> = variants_in_group
                .iter()
                .map(|v| v.variant_name().clone())
                .collect();

            // Create paths for this signature group
            let paths = self.create_paths_for_signature(signature, &applicable_variants, ctx);

            // Process each path
            for path_kind in paths {
                if let Some(path) = path_kind {
                    let mut child_ctx =
                        ctx.create_recursion_context(path.clone(), PathAction::Create);

                    // Set up enum context for children
                    if let Some(representative_variant) = applicable_variants.first() {
                        child_ctx.variant_chain.push(VariantPath {
                            full_mutation_path: ctx.full_mutation_path.clone(),
                            variant:            representative_variant.clone(),
                            instructions:       String::new(),
                            variant_example:    json!(null),
                        });
                    }
                    child_ctx.enum_context = Some(EnumContext::Child);

                    // Recursively process child and collect paths
                    let child_descriptor = path.to_mutation_path_descriptor();
                    let child_schema = child_ctx.require_registry_schema()?;
                    let child_type_kind =
                        TypeKind::from_schema(child_schema, child_ctx.type_name());

                    // Use the same recursion function as MutationPathBuilder
                    use super::mutation_path_builder::builder::recurse_mutation_paths;
                    let child_paths =
                        recurse_mutation_paths(child_type_kind, &child_ctx, depth.increment())?;

                    // Extract example from first path (same logic as MutationPathBuilder)
                    let child_example = child_paths.first().map_or(json!(null), |p| {
                        p.enum_root_example_for_parent
                            .as_ref()
                            .map_or_else(|| p.example.clone(), Clone::clone)
                    });

                    child_examples.insert(child_descriptor, child_example);

                    // Collect ALL child paths for the final result
                    all_child_paths.extend(child_paths);
                }
            }
        }

        Ok((child_examples, all_child_paths))
    }

    /// Create PathKind objects for a signature - mirrors EnumMutationBuilder logic
    fn create_paths_for_signature(
        &self,
        signature: &super::mutation_path_builder::types::VariantSignature,
        _applicable_variants: &[VariantName],
        ctx: &RecursionContext,
    ) -> Vec<Option<PathKind>> {
        use super::mutation_path_builder::types::VariantSignature;

        match signature {
            VariantSignature::Unit => {
                vec![None] // Unit variants have no paths
            }
            VariantSignature::Tuple(types) => types
                .iter()
                .enumerate()
                .map(|(index, type_name)| {
                    Some(PathKind::IndexedElement {
                        index,
                        type_name: type_name.clone(),
                        parent_type: ctx.type_name().clone(),
                    })
                })
                .collect(),
            VariantSignature::Struct(fields) => fields
                .iter()
                .map(|(field_name, type_name)| {
                    Some(PathKind::StructField {
                        field_name:  field_name.clone(),
                        type_name:   type_name.clone(),
                        parent_type: ctx.type_name().clone(),
                    })
                })
                .collect(),
        }
    }

    /// Process a single child - simplified version of MutationPathBuilder::process_child
    fn process_single_child(
        &self,
        child_ctx: &RecursionContext,
        depth: RecursionDepth,
    ) -> Result<Value> {
        // Get child's type information
        let child_schema = child_ctx.require_registry_schema()?;
        let child_type_kind = TypeKind::from_schema(child_schema, child_ctx.type_name());

        // Use the same recursion function as MutationPathBuilder
        use super::mutation_path_builder::builder::recurse_mutation_paths;
        let child_paths = recurse_mutation_paths(child_type_kind, child_ctx, depth.increment())?;

        // Extract example from first path (same logic as MutationPathBuilder)
        let child_example = child_paths.first().map_or(json!(null), |p| {
            p.enum_root_example_for_parent
                .as_ref()
                .map_or_else(|| p.example.clone(), Clone::clone)
        });

        Ok(child_example)
    }

    /// Create final result paths - includes both root and child paths
    fn create_result_paths(
        &self,
        ctx: &RecursionContext,
        assembled_value: Value,
        child_paths: Vec<MutationPathInternal>,
    ) -> Result<Vec<MutationPathInternal>> {
        // Process assembled value for enum context (same logic as MutationPathBuilder)
        let (parent_example, enum_root_examples, enum_root_example_for_parent) =
            self.process_enum_context(ctx, assembled_value);

        // Create the main mutation path for this enum root
        let root_mutation_path = MutationPathInternal {
            full_mutation_path: ctx.full_mutation_path.clone(),
            example: parent_example,
            enum_root_examples,
            enum_root_example_for_parent,
            type_name: ctx.type_name().display_name(),
            path_kind: ctx.path_kind.clone(),
            mutation_status: MutationStatus::Mutable, // Simplified for now
            mutation_status_reason: None,
            enum_instructions: None, // Simplified for now
            enum_variant_path: ctx.variant_chain.clone(),
        };

        // Return root path plus all child paths (like MutationPathBuilder does)
        let mut result = vec![root_mutation_path];
        result.extend(child_paths);
        Ok(result)
    }

    /// Process enum context - same logic as MutationPathBuilder::process_enum_context
    fn process_enum_context(
        &self,
        ctx: &RecursionContext,
        assembled_example: Value,
    ) -> (Value, Option<Vec<ExampleGroup>>, Option<Value>) {
        match &ctx.enum_context {
            Some(EnumContext::Root) => assembled_example
                .get("enum_root_data")
                .cloned()
                .map_or_else(
                    || (assembled_example, None, None),
                    |enum_data| {
                        let default_example = enum_data
                            .get("enum_root_example_for_parent")
                            .cloned()
                            .unwrap_or(json!(null));
                        let examples_json = enum_data
                            .get("enum_root_examples")
                            .cloned()
                            .unwrap_or(json!([]));
                        let examples: Vec<ExampleGroup> =
                            serde_json::from_value(examples_json).unwrap_or_default();

                        (json!(null), Some(examples), Some(default_example))
                    },
                ),
            Some(EnumContext::Child) => (assembled_example, None, None),
            None => (assembled_example, None, None),
        }
    }
}
