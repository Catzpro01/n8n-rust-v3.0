---
name: time-travel-replay
description: >-
  StateGraph checkpointing and time-travel debugging (LangGraph pattern). Rewinds conversation trajectories to prior steps and branches into alternative execution paths without restart.
---

# Time-Travel Replay — StateGraph Rewind & Branching Engine (LangGraph Pattern)

## Purpose
Enables deterministic rewind and branching of agent reasoning trajectories. If a tool execution path fails or produces suboptimal results, the agent rewinds state to the last valid milestone and explores alternative branches.

## Protocols
1. **State Node Snapshotting:** Capture state graph nodes before and after each logical transition.
2. **Deterministic Rewind:** Revert execution state to step N-k when downstream errors prove unrecoverable.
3. **Branching Exploration:** Spawn an isolated alternative trajectory from the rewind anchor.
