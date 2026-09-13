---
name: subagent-swarming
description: >-
  Hierarchical asynchronous sub-agent swarming pattern breaking massive tasks into parallel worker units.
---

# Sub-Agent Swarming

## Workflow
- Master Planner partitions large objectives into independent sub-tasks.
- Spawn lightweight, sandboxed sub-agents in parallel.
- Collate outputs through an asynchronous aggregator node.
