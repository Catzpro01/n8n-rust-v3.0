---
name: shadow-workspace-rollback
description: >-
  Shadow git branch staging and instant O(1) rollback engine (SWE-agent pattern). Isolates risky multi-file refactors before merging to main workspace.
---

# Shadow Workspace Rollback — Transactional Workspace Engine (SWE-agent Pattern)

## Purpose
Guarantees zero corruption of workspace code by staging complex multi-file modifications in temporary shadow branches or diff journals before applying them to host files.

## Protocols
1. **Shadow Staging:** Stage batch edits in memory diffs or shadow git branches.
2. **Build & Test Validation:** Verify build integrity and pass unit tests before merging changes.
3. **Instant O(1) Rollback:** If validation fails, discard the shadow branch instantly without leaving half-modified artifacts.
