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
///
/// Note: This follows the `DespawnEntityResult` pattern - the `{event}` placeholder
/// in the message template is resolved from `TriggerEventParams.event` at response-building
/// time, so we don't need an `event` field in this struct.
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct TriggerEventResult {
    /// The raw BRP response (null on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

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

**Note for implementer:** The `WorldTriggerEvent` struct doesn't need to be manually created. The `BrpTools` derive macro on the `ToolName` enum automatically generates marker structs (like `pub struct WorldTriggerEvent;`) and their `ToolFn` implementations for any variant with a `#[brp_tool(params = "...", result = "...")]` attribute.

---

### Step 6: Create Help Text

**File:** `mcp/help_text/world_trigger_event.txt` (NEW)

**Note:** Help text files use `.txt` extension (not `.md`) to match the `#[tool_description(path = "../../help_text")]` macro expectations.

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

**File:** `mcp/CHANGELOG.md`

Add entry under `[Unreleased]` section, in the `### Added` subsection (create if needed):

```markdown
### Added

- **`world_trigger_event` tool**: Trigger Bevy events remotely via the new `world.trigger_event` BRP method (Bevy 0.18+). Events must derive `Reflect` with `#[reflect(Event)]` to be triggerable.

  ```json
  {
    "event": "my_game::SpawnEnemy",
    "value": { "enemy_type": "goblin", "position": [10.0, 0.0, 5.0] }
  }
  ```
```

---

### Step 8: Create Integration Test

#### 8a. Create New Example

**File:** `test-app/examples/event_test.rs` (NEW)

```rust
//! Minimal BRP event test example
//!
//! Tests `world.trigger_event` BRP method with triggerable events.

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

/// Test event with no payload
#[derive(Event, Reflect, Clone)]
#[reflect(Event)]
struct TestUnitEvent;

/// Test event with payload
#[derive(Event, Reflect, Clone)]
#[reflect(Event)]
struct TestPayloadEvent {
    pub message: String,
    pub value: i32,
}

/// Resource to verify events were triggered
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct EventTriggerTracker {
    pub unit_event_count: u32,
    pub last_payload_message: String,
    pub last_payload_value: i32,
    pub payload_event_count: u32,
}

fn main() {
    let brp_plugin = BrpExtrasPlugin::new();
    let (port, _) = brp_plugin.get_effective_port();

    App::new()
        .add_plugins(DefaultPlugins.set(bevy::window::WindowPlugin {
            primary_window: Some(bevy::window::Window {
                title: format!("Event Test - Port {port}"),
                resolution: (400, 300).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(brp_plugin)
        // Register types with BRP for discovery and triggering
        .register_type::<TestUnitEvent>()
        .register_type::<TestPayloadEvent>()
        .register_type::<EventTriggerTracker>()
        .init_resource::<EventTriggerTracker>()
        .add_observer(on_unit_event)
        .add_observer(on_payload_event)
        .add_systems(Startup, minimize_window)
        .run();
}

fn on_unit_event(_trigger: Trigger<TestUnitEvent>, mut tracker: ResMut<EventTriggerTracker>) {
    tracker.unit_event_count += 1;
}

fn on_payload_event(trigger: Trigger<TestPayloadEvent>, mut tracker: ResMut<EventTriggerTracker>) {
    tracker.last_payload_message = trigger.event().message.clone();
    tracker.last_payload_value = trigger.event().value;
    tracker.payload_event_count += 1;
}

fn minimize_window(mut windows: Query<&mut Window>) {
    for mut window in &mut windows {
        window.set_minimized(true);
    }
}
```

#### 8b. Register Example in Cargo.toml

**File:** `test-app/Cargo.toml`

Add:
```toml
[[example]]
name = "event_test"
path = "examples/event_test.rs"
```

#### 8c. Create Integration Test File

**File:** `.claude/integration_tests/trigger_event.md` (NEW)

```markdown
# World Trigger Event Tests

## Objective
Validate the `world_trigger_event` tool for triggering Bevy events remotely.

**NOTE**: The event_test app is already running on the specified port.

## Test Steps

### 1. Verify Initial State
- Tool: `mcp__brp__world_get_resources`
- Resource: `event_test::EventTriggerTracker`
- Port: {{PORT}}
- Verify: all counters = 0, strings empty

### 2. Trigger Unit Event
- Tool: `mcp__brp__world_trigger_event`
- Params: `{"event": "event_test::TestUnitEvent", "port": {{PORT}}}`
- Verify: succeeds

### 3. Verify Unit Event Triggered
- Tool: `mcp__brp__world_get_resources`
- Resource: `event_test::EventTriggerTracker`
- Verify: `unit_event_count` = 1

### 4. Trigger Payload Event
- Tool: `mcp__brp__world_trigger_event`
- Params: `{"event": "event_test::TestPayloadEvent", "value": {"message": "Hello", "value": 42}, "port": {{PORT}}}`
- Verify: succeeds

### 5. Verify Payload Event Data
- Tool: `mcp__brp__world_get_resources`
- Resource: `event_test::EventTriggerTracker`
- Verify: `payload_event_count` = 1, `last_payload_message` = "Hello", `last_payload_value` = 42

### 6. Error Case - Unknown Event
- Tool: `mcp__brp__world_trigger_event`
- Params: `{"event": "event_test::NonExistentEvent", "port": {{PORT}}}`
- Verify: error about unknown event type

### 7. Error Case - Invalid Payload
- Tool: `mcp__brp__world_trigger_event`
- Params: `{"event": "event_test::TestPayloadEvent", "value": {"wrong": "fields"}, "port": {{PORT}}}`
- Verify: error about invalid payload

## Expected Results
- ✅ Unit events trigger without payload
- ✅ Payload events capture data correctly
- ✅ Unknown events return clear error
- ✅ Invalid payloads return clear error
```

#### 8d. Add Test Config Entry

**File:** `.claude/config/integration_tests.json`

Add entry:
```json
{
  "test_name": "trigger_event",
  "test_file": ".claude/integration_tests/trigger_event.md",
  "app_name": "event_test",
  "app_type": "example",
  "test_objective": "Test world_trigger_event for unit events, payload events, and error handling"
}
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
| `mcp/help_text/world_trigger_event.txt` | **NEW** - Help documentation |
| `mcp/CHANGELOG.md` | Document new feature under [Unreleased] |
| `test-app/examples/event_test.rs` | **NEW** - Minimal event test example |
| `test-app/Cargo.toml` | Add example entry |
| `.claude/integration_tests/trigger_event.md` | **NEW** - Test specification |
| `.claude/config/integration_tests.json` | Add test config entry |

## Testing

1. Build: `cargo build -p bevy_brp_mcp`
2. Build test example: `cargo build --example event_test`
3. Run integration test: `/integration_tests trigger_event`

## Bevy Version Requirement

This feature requires Bevy 0.18+ (specifically the `world.trigger_event` BRP method added in 0.18.0-rc.1).
