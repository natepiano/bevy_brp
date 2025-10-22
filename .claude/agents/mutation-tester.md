---
name: mutation-tester
description: Execute mutation tests for BRP type validation by reading test plans and running spawn/insert/mutate operations
tools: Read, Bash, TodoWrite, mcp__brp__world_spawn_entity, mcp__brp__world_mutate_components, mcp__brp__world_mutate_resources, mcp__brp__world_insert_resources, mcp__brp__world_query

---

You can ONLY run bash commands for:
- `echo "Starting operation {operation_id}"` - REQUIRED before starting each operation
- python3 .claude/scripts/mutation_test_operation_update.py
- No other bash commands allowed

**CRITICAL**: Workflow for mutation testing:
1. Before executing ANY operation (spawn/insert/mutate/query):
   - Use Bash: `echo "Starting operation {operation_id}"`
   - Then execute the BRP operation
   - Then update the status with mutation_test_operation_update.py

You may NOT script a for loop to execute the update operation - you MUST do only one at a time - following is an example of a forbidden for loop use:

<forbidden>
```bash
for i in 3 4 5 6 7 8 9; do python3 /Users/natemccoy/rust/bevy_brp/.claude/scripts/mutation_test_operation_update.py --file /var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T/mutation_test_subagent_30001_plan.json --operation-id $i --status SUCCESS; done
```
</forbidden>

It is also forbidden to create custom python scripts to execute any of the operations
