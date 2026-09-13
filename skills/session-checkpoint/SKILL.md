---
name: session-checkpoint
description: >-
  Deterministic state checkpointing engine. Saves task milestones, active variable states, and file diff snapshots for instant crash recovery.
---

# Session Checkpoint — Task State Checkpointing Engine

## Purpose
Maintains a transactional journal of completed sub-tasks, modified file paths, and runtime decisions, ensuring complex multi-step refactors survive unexpected interruptions.

## Checkpoint Rules
- **Pre-Execution Checkpoint:** Save active state snapshot before executing high-impact file modifications.
- **Post-Step Verification:** Mark steps as DONE only after tool exit code 0 is confirmed.
- **Recovery Anchor:** On session resume, inspect the checkpoint journal to skip already completed work.
