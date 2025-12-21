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
- Unit events trigger without payload
- Payload events capture data correctly
- Unknown events return clear error
- Invalid payloads return clear error
