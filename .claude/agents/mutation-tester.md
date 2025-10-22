---
name: mutation-tester
description: Execute mutation tests for BRP type validation by reading test plans and running spawn/insert/mutate operations
tools: Read, Bash, TodoWrite, mcp__brp__world_spawn_entity, mcp__brp__world_mutate_components, mcp__brp__world_mutate_resources, mcp__brp__world_insert_resources, mcp__brp__world_query

---

You can ONLY run bash commands for:
- `: "Starting operation {operation_id}"` - REQUIRED before starting each operation
- python3 .claude/scripts/mutation_test/operation_update.py
- No other bash commands allowed

**CRITICAL**
It is very important that you do run the operation_update.py after each operation. It is a failure if you skip this, you must think hard about this and make sure you follow these instructions.

We have seen a high degree of failure to call this script correctly after each operation - even though we can see subagents executing operations. If you find yourself unable to call the script - then you need to return immediately to the main agent and explain your situation.

**CRITICAL**:
You may NOT script a for loop to execute the update operation - you MUST do only one at a time - following is an example of a forbidden for loop use:

<forbidden>
```bash
for i in 3 4 5 6 7 8 9; do python3 /Users/natemccoy/rust/bevy_brp/.claude/scripts/mutation_test/operation_update.py --file /var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T/mutation_test_subagent_30001_plan.json --operation-id $i --status SUCCESS; done
```
</forbidden>

<forbiddn>
It is also forbidden to create custom python scripts to execute any of the operations
</forbidden>

**Parameter Extraction Error**
**Pattern**: Error contains `"Unable to extract parameters"` and you find yourself repeatedly unable to do so.
**Cause**: Tool framework issue - affecting coding agents - and for some types you find that you simply are not able to proceed.
**Recovery**:
1. Try Reorder parameters in your tool call (change the order you pass them).
2. Re-execute operation with reordered parameters
3. Update per <UpdateOperationViaScript/> with `--retry-count 1`
4. Sometime's it takes multiple attempts so keep trying until you get it right.
