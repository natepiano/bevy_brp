#!/usr/bin/env bash
# Mouse input integration test script
# Usage: bash .claude/scripts/integration_tests/mouse_input_test.sh <PORT>
#
# Validates comprehensive mouse input simulation on both primary and secondary windows.
# Communicates with a running mouse_test Bevy example via BRP JSON-RPC over HTTP.
#
# Exit code: 0 = all passed, 1 = any failures

set -euo pipefail

# ---------------------------------------------------------------------------
# Argument parsing & constants
# ---------------------------------------------------------------------------

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <PORT>"
    exit 1
fi

PORT="$1"
BRP_URL="http://127.0.0.1:${PORT}"
RESOURCE_TYPE="mouse_test::MouseStateTracker"

# Counters
PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

# Entity IDs discovered in scenario 1
PRIMARY_WINDOW=""
SECONDARY_WINDOW=""

# Last fetched tracker JSON (updated by get_tracker)
TRACKER=""

# Last raw BRP response (updated by brp_call)
BRP_RESPONSE=""

# JSON-RPC request ID counter
RPC_ID=1

# Float comparison tolerance
TOLERANCE=1.0

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------

brp_call() {
    local method="$1"
    local params="${2:-null}"

    local body
    if [[ "$params" == "null" ]]; then
        body="{\"jsonrpc\":\"2.0\",\"method\":\"${method}\",\"id\":${RPC_ID}}"
    else
        body="{\"jsonrpc\":\"2.0\",\"method\":\"${method}\",\"params\":${params},\"id\":${RPC_ID}}"
    fi
    RPC_ID=$((RPC_ID + 1))

    BRP_RESPONSE=$(curl -sf "${BRP_URL}" \
        -H "Content-Type: application/json" \
        -d "${body}" 2>/dev/null) || {
        echo "    [ERROR] curl failed for method=${method}"
        return 1
    }

    # Check for JSON-RPC error
    if echo "${BRP_RESPONSE}" | jq -e '.error' >/dev/null 2>&1; then
        local err_msg
        err_msg=$(echo "${BRP_RESPONSE}" | jq -r '.error.message // "unknown error"')
        echo "    [ERROR] BRP error for ${method}: ${err_msg}"
        return 1
    fi

    return 0
}

get_tracker() {
    brp_call "world.get_resources" "{\"resource\":\"${RESOURCE_TYPE}\"}" || return 1
    TRACKER=$(echo "${BRP_RESPONSE}" | jq -r '.result.value')
    return 0
}

move_mouse_abs() {
    local x="$1" y="$2" window="${3:-}"
    local params
    if [[ -n "$window" ]]; then
        params="{\"position\":[${x},${y}],\"window\":${window}}"
    else
        params="{\"position\":[${x},${y}]}"
    fi
    brp_call "brp_extras/move_mouse" "$params"
}

move_mouse_delta() {
    local dx="$1" dy="$2" window="${3:-}"
    local params
    if [[ -n "$window" ]]; then
        params="{\"delta\":[${dx},${dy}],\"window\":${window}}"
    else
        params="{\"delta\":[${dx},${dy}]}"
    fi
    brp_call "brp_extras/move_mouse" "$params"
}

click_mouse() {
    local button="$1" window="${2:-}"
    local params
    if [[ -n "$window" ]]; then
        params="{\"button\":\"${button}\",\"window\":${window}}"
    else
        params="{\"button\":\"${button}\"}"
    fi
    brp_call "brp_extras/click_mouse" "$params"
}

double_click() {
    local button="$1" delay_ms="${2:-}" window="${3:-}"
    local params="{\"button\":\"${button}\""
    if [[ -n "$delay_ms" ]]; then
        params="${params},\"delay_ms\":${delay_ms}"
    fi
    if [[ -n "$window" ]]; then
        params="${params},\"window\":${window}"
    fi
    params="${params}}"
    brp_call "brp_extras/double_click_mouse" "$params"
}

send_button() {
    local button="$1" duration_ms="${2:-}" window="${3:-}"
    local params="{\"button\":\"${button}\""
    if [[ -n "$duration_ms" ]]; then
        params="${params},\"duration_ms\":${duration_ms}"
    fi
    if [[ -n "$window" ]]; then
        params="${params},\"window\":${window}"
    fi
    params="${params}}"
    brp_call "brp_extras/send_mouse_button" "$params"
}

scroll() {
    local x="$1" y="$2" unit="$3" window="${4:-}"
    local params="{\"x\":${x},\"y\":${y},\"unit\":\"${unit}\""
    if [[ -n "$window" ]]; then
        params="${params},\"window\":${window}"
    fi
    params="${params}}"
    brp_call "brp_extras/scroll_mouse" "$params"
}

drag() {
    local button="$1" sx="$2" sy="$3" ex="$4" ey="$5" frames="$6" window="${7:-}"
    local params="{\"button\":\"${button}\",\"start\":[${sx},${sy}],\"end\":[${ex},${ey}],\"frames\":${frames}"
    if [[ -n "$window" ]]; then
        params="${params},\"window\":${window}"
    fi
    params="${params}}"
    brp_call "brp_extras/drag_mouse" "$params"
}

pinch() {
    local delta="$1"
    brp_call "brp_extras/pinch_gesture" "{\"delta\":${delta}}"
}

rotate() {
    local delta="$1"
    brp_call "brp_extras/rotation_gesture" "{\"delta\":${delta}}"
}

double_tap() {
    brp_call "brp_extras/double_tap_gesture" "null"
}

# --- Assertion helpers ---

pass_() {
    local msg="$1"
    echo "  [PASS] ${msg}"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail_() {
    local msg="$1"
    echo "  [FAIL] ${msg}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

skip_() {
    local msg="$1"
    echo "  [SKIP] ${msg}"
    SKIP_COUNT=$((SKIP_COUNT + 1))
}

# Compare two floats within TOLERANCE using awk
assert_float_eq() {
    local label="$1" actual="$2" expected="$3" tol="${4:-$TOLERANCE}"
    local ok
    ok=$(awk "BEGIN { print (($actual - $expected) >= -$tol && ($actual - $expected) <= $tol) ? 1 : 0 }")
    if [[ "$ok" == "1" ]]; then
        pass_ "${label} = ${actual} (expected ${expected} +/-${tol})"
    else
        fail_ "${label} = ${actual} (expected ${expected} +/-${tol})"
    fi
}

# Check a Vec2 field (serialized as [x, y]) from TRACKER
assert_vec2() {
    local label="$1" field="$2" exp_x="$3" exp_y="$4" tol="${5:-$TOLERANCE}"
    local ax ay
    ax=$(echo "${TRACKER}" | jq -r ".${field}[0]")
    ay=$(echo "${TRACKER}" | jq -r ".${field}[1]")
    assert_float_eq "${label}.x" "$ax" "$exp_x" "$tol"
    assert_float_eq "${label}.y" "$ay" "$exp_y" "$tol"
}

assert_bool() {
    local label="$1" field="$2" expected="$3"
    local actual
    actual=$(echo "${TRACKER}" | jq -r ".${field}")
    if [[ "$actual" == "$expected" ]]; then
        pass_ "${label} = ${actual}"
    else
        fail_ "${label} = ${actual} (expected ${expected})"
    fi
}

assert_gt() {
    local label="$1" actual="$2" threshold="$3"
    local ok
    ok=$(awk "BEGIN { print ($actual > $threshold) ? 1 : 0 }")
    if [[ "$ok" == "1" ]]; then
        pass_ "${label} = ${actual} (> ${threshold})"
    else
        fail_ "${label} = ${actual} (expected > ${threshold})"
    fi
}

assert_int_eq() {
    local label="$1" actual="$2" expected="$3"
    if [[ "$actual" == "$expected" ]]; then
        pass_ "${label} = ${actual}"
    else
        fail_ "${label} = ${actual} (expected ${expected})"
    fi
}

assert_string_eq() {
    local label="$1" actual="$2" expected="$3"
    if [[ "$actual" == "$expected" ]]; then
        pass_ "${label} = ${actual}"
    else
        fail_ "${label} = ${actual} (expected ${expected})"
    fi
}

# ---------------------------------------------------------------------------
# Scenario 1: Window Discovery
# ---------------------------------------------------------------------------

scenario_1_window_discovery() {
    echo ""
    echo "--- Scenario 1: Window Discovery ---"

    # Query for primary window
    brp_call "world.query" '{"data":{},"filter":{"with":["bevy_window::window::PrimaryWindow"]}}' || {
        fail_ "Failed to query primary window"
        return
    }
    PRIMARY_WINDOW=$(echo "${BRP_RESPONSE}" | jq -r '.result[0].entity')

    # Query for secondary window
    brp_call "world.query" '{"data":{},"filter":{"with":["mouse_test::SecondaryWindow"]}}' || {
        fail_ "Failed to query secondary window"
        return
    }
    SECONDARY_WINDOW=$(echo "${BRP_RESPONSE}" | jq -r '.result[0].entity')

    # Validate
    if [[ -n "$PRIMARY_WINDOW" && "$PRIMARY_WINDOW" != "null" ]]; then
        pass_ "Primary window: ${PRIMARY_WINDOW}"
    else
        fail_ "Primary window not found"
    fi

    if [[ -n "$SECONDARY_WINDOW" && "$SECONDARY_WINDOW" != "null" ]]; then
        pass_ "Secondary window: ${SECONDARY_WINDOW}"
    else
        fail_ "Secondary window not found"
    fi

    if [[ "$PRIMARY_WINDOW" != "$SECONDARY_WINDOW" ]]; then
        pass_ "Windows have distinct entity IDs"
    else
        fail_ "Primary and secondary windows have same entity ID"
    fi
}

# ---------------------------------------------------------------------------
# Scenario 2: Cursor Movement
# ---------------------------------------------------------------------------

scenario_2_cursor_movement() {
    echo ""
    echo "--- Scenario 2: Cursor Movement ---"

    # Move to absolute positions on both windows, then apply deltas
    move_mouse_abs 200 150 "$PRIMARY_WINDOW" || { fail_ "move_mouse_abs primary"; return; }
    move_mouse_abs 100 100 "$SECONDARY_WINDOW" || { fail_ "move_mouse_abs secondary"; return; }
    move_mouse_delta 50 30 "$PRIMARY_WINDOW" || { fail_ "move_mouse_delta primary"; return; }
    move_mouse_delta 20 10 "$SECONDARY_WINDOW" || { fail_ "move_mouse_delta secondary"; return; }

    sleep 0.2
    get_tracker || { fail_ "Failed to get tracker"; return; }

    assert_vec2 "primary_window_position" "primary_window_position" 250 180
    assert_vec2 "secondary_window_position" "secondary_window_position" 120 110
}

# ---------------------------------------------------------------------------
# Scenario 3: Mouse Buttons
# ---------------------------------------------------------------------------

scenario_3_mouse_buttons() {
    echo ""
    echo "--- Scenario 3: Mouse Buttons ---"

    # Move to primary, press all 5 buttons
    move_mouse_abs 150 150 "$PRIMARY_WINDOW" || { fail_ "move_mouse_abs primary"; return; }
    send_button "Left" 100 || { fail_ "send_button Left"; return; }
    send_button "Right" 100 || { fail_ "send_button Right"; return; }
    send_button "Middle" 100 || { fail_ "send_button Middle"; return; }
    send_button "Back" 100 || { fail_ "send_button Back"; return; }
    send_button "Forward" 100 || { fail_ "send_button Forward"; return; }

    sleep 0.2

    get_tracker || { fail_ "Failed to get tracker"; return; }

    # All button timestamps should be > 0
    local left_ts right_ts mid_ts back_ts fwd_ts
    left_ts=$(echo "${TRACKER}" | jq -r '.left_timestamp')
    right_ts=$(echo "${TRACKER}" | jq -r '.right_timestamp')
    mid_ts=$(echo "${TRACKER}" | jq -r '.middle_timestamp')
    back_ts=$(echo "${TRACKER}" | jq -r '.back_timestamp')
    fwd_ts=$(echo "${TRACKER}" | jq -r '.forward_timestamp')

    assert_gt "left_timestamp" "$left_ts" 0
    assert_gt "right_timestamp" "$right_ts" 0
    assert_gt "middle_timestamp" "$mid_ts" 0
    assert_gt "back_timestamp" "$back_ts" 0
    assert_gt "forward_timestamp" "$fwd_ts" 0

    # Wait for all buttons to release
    sleep 0.2

    get_tracker || { fail_ "Failed to get tracker after release"; return; }

    # All buttons should be released
    assert_bool "left_pressed" "left_pressed" "false"
    assert_bool "right_pressed" "right_pressed" "false"
    assert_bool "middle_pressed" "middle_pressed" "false"
    assert_bool "back_pressed" "back_pressed" "false"
    assert_bool "forward_pressed" "forward_pressed" "false"
}

# ---------------------------------------------------------------------------
# Scenario 4: Scroll Operations
# ---------------------------------------------------------------------------

scenario_4_scroll() {
    echo ""
    echo "--- Scenario 4: Scroll Operations ---"

    # Primary: scroll Y by 5 lines, then X by 3 lines
    move_mouse_abs 300 200 "$PRIMARY_WINDOW" || return
    scroll 0.0 5.0 "Line" "$PRIMARY_WINDOW" || return
    scroll 3.0 0.0 "Line" "$PRIMARY_WINDOW" || return

    # Secondary: scroll Y by 4 lines, X by -2 lines
    move_mouse_abs 300 200 "$SECONDARY_WINDOW" || return
    scroll 0.0 4.0 "Line" "$SECONDARY_WINDOW" || return
    scroll -2.0 0.0 "Line" "$SECONDARY_WINDOW" || return

    # Primary: scroll Y by 100 pixels
    move_mouse_abs 300 200 "$PRIMARY_WINDOW" || return
    scroll 0.0 100.0 "Pixel" "$PRIMARY_WINDOW" || return

    # Secondary: scroll Y by 50 pixels
    move_mouse_abs 300 200 "$SECONDARY_WINDOW" || return
    scroll 0.0 50.0 "Pixel" "$SECONDARY_WINDOW" || return

    sleep 0.2
    get_tracker || { fail_ "Failed to get tracker"; return; }

    local psx psy psu ssx ssy ssu
    psx=$(echo "${TRACKER}" | jq -r '.primary_scroll_x_total')
    psy=$(echo "${TRACKER}" | jq -r '.primary_scroll_y_total')
    psu=$(echo "${TRACKER}" | jq -r '.primary_scroll_unit')
    ssx=$(echo "${TRACKER}" | jq -r '.secondary_scroll_x_total')
    ssy=$(echo "${TRACKER}" | jq -r '.secondary_scroll_y_total')
    ssu=$(echo "${TRACKER}" | jq -r '.secondary_scroll_unit')

    assert_float_eq "primary_scroll_x_total" "$psx" 3.0 0.1
    assert_float_eq "primary_scroll_y_total" "$psy" 105.0 0.1
    assert_string_eq "primary_scroll_unit" "$psu" "Pixel"
    assert_float_eq "secondary_scroll_x_total" "$ssx" -2.0 0.1
    assert_float_eq "secondary_scroll_y_total" "$ssy" 54.0 0.1
    assert_string_eq "secondary_scroll_unit" "$ssu" "Pixel"
}

# ---------------------------------------------------------------------------
# Scenario 5: Gestures (macOS only)
# ---------------------------------------------------------------------------

scenario_5_gestures() {
    echo ""
    echo "--- Scenario 5: Gestures ---"

    if [[ "$(uname)" != "Darwin" ]]; then
        skip_ "Gesture tests skipped (not macOS)"
        return
    fi

    # Primary window gestures
    move_mouse_abs 300 200 "$PRIMARY_WINDOW" || return
    pinch 2.5 || return
    pinch -1.0 || return
    rotate 0.5 || return
    double_tap || return

    # Secondary window gestures
    move_mouse_abs 300 200 "$SECONDARY_WINDOW" || return
    pinch 3.0 || return
    pinch 0.5 || return
    rotate 0.3 || return
    double_tap || return

    sleep 0.2
    get_tracker || { fail_ "Failed to get tracker"; return; }

    local pp pr pdt sp sr sdt
    pp=$(echo "${TRACKER}" | jq -r '.primary_pinch_total')
    pr=$(echo "${TRACKER}" | jq -r '.primary_rotation_total')
    pdt=$(echo "${TRACKER}" | jq -r '.primary_double_tap_timestamp')
    sp=$(echo "${TRACKER}" | jq -r '.secondary_pinch_total')
    sr=$(echo "${TRACKER}" | jq -r '.secondary_rotation_total')
    sdt=$(echo "${TRACKER}" | jq -r '.secondary_double_tap_timestamp')

    assert_float_eq "primary_pinch_total" "$pp" 1.5 0.1
    assert_float_eq "primary_rotation_total" "$pr" 0.5 0.1
    assert_gt "primary_double_tap_timestamp" "$pdt" 0
    assert_float_eq "secondary_pinch_total" "$sp" 3.5 0.1
    assert_float_eq "secondary_rotation_total" "$sr" 0.3 0.1
    assert_gt "secondary_double_tap_timestamp" "$sdt" 0
}

# ---------------------------------------------------------------------------
# Scenario 6: Click Operations
# ---------------------------------------------------------------------------

scenario_6_clicks() {
    echo ""
    echo "--- Scenario 6: Click Operations ---"

    # Primary window: single click, then double click
    move_mouse_abs 200 200 "$PRIMARY_WINDOW" || return
    click_mouse "Left" "$PRIMARY_WINDOW" || return
    sleep 0.6  # age out double-click window

    move_mouse_abs 250 175 "$PRIMARY_WINDOW" || return
    double_click "Left" 100 "$PRIMARY_WINDOW" || return
    sleep 0.3

    # Secondary window: single click, then double click
    move_mouse_abs 100 100 "$SECONDARY_WINDOW" || return
    click_mouse "Left" "$SECONDARY_WINDOW" || return
    sleep 0.6

    move_mouse_abs 150 120 "$SECONDARY_WINDOW" || return
    double_click "Left" 100 "$SECONDARY_WINDOW" || return
    sleep 0.3

    get_tracker || { fail_ "Failed to get tracker"; return; }

    assert_vec2 "primary_click_position" "primary_click_position" 250 175
    assert_vec2 "primary_doubleclick_position" "primary_doubleclick_position" 250 175

    assert_vec2 "secondary_click_position" "secondary_click_position" 150 120
    assert_vec2 "secondary_doubleclick_position" "secondary_doubleclick_position" 150 120

    local pcts pdcts scts sdcts
    pcts=$(echo "${TRACKER}" | jq -r '.primary_click_timestamp')
    pdcts=$(echo "${TRACKER}" | jq -r '.primary_doubleclick_timestamp')
    scts=$(echo "${TRACKER}" | jq -r '.secondary_click_timestamp')
    sdcts=$(echo "${TRACKER}" | jq -r '.secondary_doubleclick_timestamp')

    assert_gt "primary_click_timestamp" "$pcts" 0
    assert_gt "primary_doubleclick_timestamp" "$pdcts" 0
    assert_gt "secondary_click_timestamp" "$scts" 0
    assert_gt "secondary_doubleclick_timestamp" "$sdcts" 0
}

# ---------------------------------------------------------------------------
# Scenario 7: Drag Operations
# ---------------------------------------------------------------------------

scenario_7_drags() {
    echo ""
    echo "--- Scenario 7: Drag Operations ---"

    drag "Left" 100 100 300 200 20 "$PRIMARY_WINDOW" || { fail_ "drag primary"; return; }
    drag "Left" 50 50 150 150 20 "$SECONDARY_WINDOW" || { fail_ "drag secondary"; return; }

    # Wait for drags to complete (20 frames at debug fps ~15-30fps ≈ 0.7-1.3s)
    sleep 1.5

    get_tracker || { fail_ "Failed to get tracker"; return; }

    assert_vec2 "primary_window_position (post-drag)" "primary_window_position" 300 200
    assert_vec2 "secondary_window_position (post-drag)" "secondary_window_position" 150 150
}

# ---------------------------------------------------------------------------
# Scenario 8: Picking Validation
# ---------------------------------------------------------------------------

scenario_8_picking() {
    echo ""
    echo "--- Scenario 8: Picking Validation ---"

    # Click primary cuboid (center of 600x400 window)
    move_mouse_abs 300 200 "$PRIMARY_WINDOW" || return
    click_mouse "Left" "$PRIMARY_WINDOW" || return
    sleep 0.2

    get_tracker || { fail_ "Failed to get tracker"; return; }
    local pcc
    pcc=$(echo "${TRACKER}" | jq -r '.primary_picking_click_count')
    assert_gt "primary_picking_click_count" "$pcc" 0
    assert_bool "primary_picking_gizmo_active" "primary_picking_gizmo_active" "true"

    # Wait, then double-click primary cuboid
    sleep 0.6
    move_mouse_abs 300 200 "$PRIMARY_WINDOW" || return
    double_click "Left" 100 "$PRIMARY_WINDOW" || return
    sleep 0.3

    get_tracker || { fail_ "Failed to get tracker"; return; }
    local pdcc
    pdcc=$(echo "${TRACKER}" | jq -r '.primary_picking_doubleclick_count')
    assert_gt "primary_picking_doubleclick_count" "$pdcc" 0

    # Click away from cuboid on primary (deselect)
    move_mouse_abs 50 50 "$PRIMARY_WINDOW" || return
    click_mouse "Left" "$PRIMARY_WINDOW" || return
    sleep 0.2

    get_tracker || { fail_ "Failed to get tracker"; return; }
    assert_bool "primary_picking_gizmo_active (deselected)" "primary_picking_gizmo_active" "false"

    # Click secondary cuboid
    move_mouse_abs 300 200 "$SECONDARY_WINDOW" || return
    click_mouse "Left" "$SECONDARY_WINDOW" || return
    sleep 0.2

    get_tracker || { fail_ "Failed to get tracker"; return; }
    local scc
    scc=$(echo "${TRACKER}" | jq -r '.secondary_picking_click_count')
    assert_gt "secondary_picking_click_count" "$scc" 0
    assert_bool "secondary_picking_gizmo_active" "secondary_picking_gizmo_active" "true"

    # Wait, then double-click secondary cuboid
    sleep 0.6
    move_mouse_abs 300 200 "$SECONDARY_WINDOW" || return
    double_click "Left" 100 "$SECONDARY_WINDOW" || return
    sleep 0.3

    get_tracker || { fail_ "Failed to get tracker"; return; }
    local sdcc
    sdcc=$(echo "${TRACKER}" | jq -r '.secondary_picking_doubleclick_count')
    assert_gt "secondary_picking_doubleclick_count" "$sdcc" 0

    # Click away from cuboid on secondary (deselect)
    move_mouse_abs 50 50 "$SECONDARY_WINDOW" || return
    click_mouse "Left" "$SECONDARY_WINDOW" || return
    sleep 0.2

    get_tracker || { fail_ "Failed to get tracker"; return; }
    assert_bool "secondary_picking_gizmo_active (deselected)" "secondary_picking_gizmo_active" "false"
}

# ---------------------------------------------------------------------------
# Scenario 9: Final Verification
# ---------------------------------------------------------------------------

scenario_9_final_verification() {
    echo ""
    echo "--- Scenario 9: Final Verification ---"

    get_tracker || { fail_ "Failed to get tracker"; return; }

    # All buttons released
    assert_bool "left_pressed (final)" "left_pressed" "false"
    assert_bool "right_pressed (final)" "right_pressed" "false"
    assert_bool "middle_pressed (final)" "middle_pressed" "false"
    assert_bool "back_pressed (final)" "back_pressed" "false"
    assert_bool "forward_pressed (final)" "forward_pressed" "false"

    # Picking counts > 0 on both windows
    local pcc pdcc scc sdcc
    pcc=$(echo "${TRACKER}" | jq -r '.primary_picking_click_count')
    pdcc=$(echo "${TRACKER}" | jq -r '.primary_picking_doubleclick_count')
    scc=$(echo "${TRACKER}" | jq -r '.secondary_picking_click_count')
    sdcc=$(echo "${TRACKER}" | jq -r '.secondary_picking_doubleclick_count')

    assert_gt "primary_picking_click_count (final)" "$pcc" 0
    assert_gt "primary_picking_doubleclick_count (final)" "$pdcc" 0
    assert_gt "secondary_picking_click_count (final)" "$scc" 0
    assert_gt "secondary_picking_doubleclick_count (final)" "$sdcc" 0

    # Both gizmos deselected
    assert_bool "primary_picking_gizmo_active (final)" "primary_picking_gizmo_active" "false"
    assert_bool "secondary_picking_gizmo_active (final)" "secondary_picking_gizmo_active" "false"

    # Scroll totals reflect test values
    local psy ssy
    psy=$(echo "${TRACKER}" | jq -r '.primary_scroll_y_total')
    ssy=$(echo "${TRACKER}" | jq -r '.secondary_scroll_y_total')
    assert_float_eq "primary_scroll_y_total (final)" "$psy" 105.0 0.1
    assert_float_eq "secondary_scroll_y_total (final)" "$ssy" 54.0 0.1
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

echo "================================================================"
echo "  Mouse Input Integration Test — Port: ${PORT}"
echo "================================================================"

scenario_1_window_discovery
scenario_2_cursor_movement
scenario_3_mouse_buttons
scenario_4_scroll
scenario_5_gestures
scenario_6_clicks
scenario_7_drags
scenario_8_picking
scenario_9_final_verification

echo ""
echo "================================================================"
TOTAL=$((PASS_COUNT + FAIL_COUNT + SKIP_COUNT))
echo "  RESULTS: ${PASS_COUNT} passed, ${FAIL_COUNT} failed, ${SKIP_COUNT} skipped (${TOTAL} total)"
if [[ "$FAIL_COUNT" -gt 0 ]]; then
    echo "  STATUS: FAILED"
    echo "================================================================"
    exit 1
else
    echo "  STATUS: PASSED"
    echo "================================================================"
    exit 0
fi
