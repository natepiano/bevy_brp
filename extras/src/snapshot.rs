//! Snapshot handler for BRP extras
//!
//! Recursive YAML outline of a UI entity tree: entity id, a short
//! component-type label, and any text content.

use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::Node;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde_json::Value;
use serde_json::json;

use crate::constants::PARAM_ROOT;
use crate::constants::RESPONSE_YAML_FIELD;

/// Components that don't tell you anything about *what* a UI node is —
/// skipped when picking the one type name to label a snapshot row with.
const NOISE_COMPONENTS: &[&str] = &[
    "Node",
    "ComputedNode",
    "ComputedNodeTarget",
    "UiTransform",
    "UiGlobalTransform",
    "Transform",
    "GlobalTransform",
    "Visibility",
    "InheritedVisibility",
    "ViewVisibility",
    "ChildOf",
    "Children",
    "Text",
    "TextFont",
    "TextColor",
    "TextLayout",
    "EditableText",
    "AccessibilityNode",
    "ComputedUiTargetCamera",
    "ComputedUiRenderTargetInfo",
    "Interaction",
    "RelativeCursorPosition",
    "TabIndex",
    "FocusPolicy",
    "ScrollPosition",
    "BorderRadius",
    "BorderColor",
    "BackgroundColor",
];

fn entity_label(world: &World, entity: Entity) -> String {
    let Ok(infos) = world.inspect_entity(entity) else {
        return "Entity".to_string();
    };

    infos
        .map(|info| info.name().to_string())
        .find(|full| !NOISE_COMPONENTS.contains(&short_name(full)))
        .map(|full| short_name(&full).to_string())
        .unwrap_or_else(|| "Entity".to_string())
}

fn short_name(full: &str) -> &str {
    full.rsplit("::")
        .next()
        .unwrap_or(full)
        .trim_end_matches('>')
}

fn write_snapshot_node(world: &World, entity: Entity, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    out.push_str(&format!(
        "{pad}- entity: {}\n{pad}  type: {}\n",
        entity.to_bits(),
        entity_label(world, entity)
    ));

    if let Some(name) = world.get::<Name>(entity) {
        out.push_str(&format!("{pad}  name: \"#{}\"\n", name.as_str()));
    }

    if let Some(text) = world.get::<Text>(entity) {
        out.push_str(&format!("{pad}  text: {:?}\n", text.0));
    } else if world.get::<EditableText>(entity).is_some() {
        out.push_str(&format!("{pad}  text: <editable>\n"));
    }

    if let Some(children) = world.get::<Children>(entity)
        && !children.is_empty()
    {
        out.push_str(&format!("{pad}  children:\n"));
        for child in children.iter() {
            write_snapshot_node(world, child, indent + 2, out);
        }
    }
}

/// Handler for snapshot requests
///
/// Returns a recursive YAML outline of the UI entity tree, rooted at the
/// entity given by the optional `root` param, or every top-level `Node`
/// (an entity with `Node` but no `ChildOf`) if `root` is omitted.
pub(crate) fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let root_param = params
        .as_ref()
        .and_then(|value| value.get(PARAM_ROOT))
        .map(|value| {
            serde_json::from_value::<Entity>(value.clone()).map_err(|err| BrpError {
                code:    INVALID_PARAMS,
                message: format!("Invalid '{PARAM_ROOT}' parameter: {err}"),
                data:    None,
            })
        })
        .transpose()?;

    let roots: Vec<Entity> = match root_param {
        Some(entity) => vec![entity],
        None => world
            .query_filtered::<Entity, (With<Node>, Without<ChildOf>)>()
            .iter(world)
            .collect(),
    };

    let mut out = String::new();
    for root in roots {
        write_snapshot_node(world, root, 0, &mut out);
    }

    Ok(json!({ RESPONSE_YAML_FIELD: out }))
}
