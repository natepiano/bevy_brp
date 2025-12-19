# Plan: Add `world_trigger_event` MCP Tool

## Overview

Add support for the new Bevy 0.18 `world.trigger_event` BRP method, which allows triggering events remotely. This enables AI agents to trigger game events like spawning enemies, changing game state, triggering cutscenes, etc.

## Background

Bevy 0.18 added `world.trigger_event` (commit `0af6fc1c7`). Events must be defined with:
```rust
#[derive(Event, Reflect)]
#[reflect(Event)]
pub struct MyEvent { ... }
```

BRP Parameters:
- `event`: Full type path (e.g., "my_game::SpawnEnemy")
- `value`: Optional JSON payload for the event data

## Implementation Steps

### Step 1: Add `Event` ToolCategory

**File:** `mcp/src/tool/annotations.rs`

Add new variant to `ToolCategory` enum:
```rust
#[strum(serialize = "Event")]
Event,
```

---

### Step 2: Create Tool Parameter/Result Structs

**File:** `mcp/src/brp_tools/tools/world_trigger_event.rs` (NEW)

```rust
//! `world.trigger_event` tool - Trigger events in the Bevy world

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.trigger_event` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct TriggerEventParams {
    /// The full type path of the event to trigger (e.g., "my_game::events::SpawnEnemy")
    pub event: String,

    /// The serialized value of the event payload, if any.
    /// For unit events (no data), omit this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.trigger_event` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(enhanced_errors = true)]
pub struct TriggerEventResult {
    /// The raw BRP response (null on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// The event type that was triggered (for metadata)
    #[to_metadata]
    pub event: String,

    /// Message template for formatting responses
    #[to_message(message_template = "Triggered event {event}")]
    pub message_template: String,
}
```

---

### Step 3: Register Module

**File:** `mcp/src/brp_tools/tools/mod.rs`

Add:
```rust
pub mod world_trigger_event;
```

---

### Step 4: Export Types from brp_tools

**File:** `mcp/src/brp_tools/mod.rs`

Add to the pub use statement for tools:
```rust
pub use tools::world_trigger_event::TriggerEventParams;
pub use tools::world_trigger_event::TriggerEventResult;
```

---

### Step 5: Add to ToolName Enum

**File:** `mcp/src/tool/tool_name.rs`

#### 5a. Add import

In the imports section, add `TriggerEventParams` and `TriggerEventResult`:
```rust
use crate::brp_tools::{
    // ... existing imports ...
    TriggerEventParams, TriggerEventResult,
};
```

#### 5b. Add enum variant

Add after `WorldSpawnEntity`:
```rust
/// `world_trigger_event` - Trigger events in the Bevy world
#[brp_tool(
    brp_method = "world.trigger_event",
    params = "TriggerEventParams",
    result = "TriggerEventResult"
)]
WorldTriggerEvent,
```

#### 5c. Add annotation in `get_annotations()`

Add case:
```rust
Self::WorldTriggerEvent => Annotation::new(
    "trigger event",
    ToolCategory::Event,
    EnvironmentImpact::AdditiveNonIdempotent,
),
```

#### 5d. Add to `get_parameters()`

Add case:
```rust
Self::WorldTriggerEvent => Some(parameters::build_parameters_from::<TriggerEventParams>),
```

#### 5e. Add to `create_handler()`

Add case in the BRP tools section:
```rust
Self::WorldTriggerEvent => Arc::new(WorldTriggerEvent),
```

---

### Step 6: Create Help Text

**File:** `mcp/help_text/world_trigger_event.md` (NEW)

```markdown
# world_trigger_event

Triggers an event in the Bevy world. This allows remote triggering of game events like spawning enemies, changing game state, or triggering cutscenes.

## Requirements

Events must be registered with reflection to be triggerable:

```rust
#[derive(Event, Reflect)]
#[reflect(Event)]
pub struct SpawnEnemy {
    pub enemy_type: String,
    pub position: Vec3,
}

// In your app setup:
app.register_type::<SpawnEnemy>();
```

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `event` | string | Yes | Full type path of the event (e.g., "my_game::SpawnEnemy") |
| `value` | object | No | Event payload as JSON. Omit for unit events (events with no data). |
| `port` | number | No | BRP port (default: 15702) |

## Examples

### Unit Event (no payload)

Trigger a simple event with no data:

```json
{
  "event": "my_game::events::PauseGame"
}
```

### Event with Payload

Trigger an event with structured data:

```json
{
  "event": "my_game::events::SpawnEnemy",
  "value": {
    "enemy_type": "goblin",
    "position": [10.0, 0.0, 5.0]
  }
}
```

### Event with Entity Reference

Events can reference entities:

```json
{
  "event": "my_game::events::DamageEntity",
  "value": {
    "target": 4294967299,
    "amount": 50
  }
}
```

## Error Handling

| Error | Cause |
|-------|-------|
| "Unknown event type" | Event type not registered in the type registry |
| "Event is not reflectable" | Event missing `#[reflect(Event)]` attribute |
| "is invalid" | Payload doesn't match expected event structure |

## Notes

- Events are triggered globally via `World::trigger()`, not targeted at specific entities
- The event must have `#[reflect(Event)]` to be discoverable and triggerable
- Use `brp_type_guide` or `registry_schema` to discover available event types
```

---

### Step 7: Update CHANGELOG

**File:** `CHANGELOG.md`

Add entry under new features:

```markdown
### New Features

- **`world_trigger_event` tool**: Trigger Bevy events remotely via the new `world.trigger_event` BRP method (Bevy 0.18+). Events must derive `Reflect` with `#[reflect(Event)]` to be triggerable.

  ```json
  {
    "event": "my_game::SpawnEnemy",
    "value": { "enemy_type": "goblin", "position": [10.0, 0.0, 5.0] }
  }
  ```
```

---

## File Summary

| File | Action |
|------|--------|
| `mcp/src/tool/annotations.rs` | Add `Event` variant to `ToolCategory` |
| `mcp/src/brp_tools/tools/world_trigger_event.rs` | **NEW** - Parameter/result structs |
| `mcp/src/brp_tools/tools/mod.rs` | Add module declaration |
| `mcp/src/brp_tools/mod.rs` | Export new types |
| `mcp/src/tool/tool_name.rs` | Add enum variant, imports, annotation, params, handler |
| `mcp/help_text/world_trigger_event.md` | **NEW** - Help documentation |
| `CHANGELOG.md` | Document new feature |

## Testing

1. Build: `cargo build -p bevy_brp_mcp`
2. Launch test app with an event registered
3. Test via MCP tool call:
   ```json
   {
     "event": "test_app::TestEvent",
     "value": { "message": "Hello from MCP!" }
   }
   ```

## Bevy Version Requirement

This feature requires Bevy 0.18+ (specifically the `world.trigger_event` BRP method added in 0.18.0-rc.1).
