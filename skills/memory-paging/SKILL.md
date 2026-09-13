---
name: memory-paging
description: >-
  Hierarchical OS-style virtual memory paging (Letta / MemGPT pattern). Manages Core Memory (RAM), Working Context, and Archival Vector Storage (Disk).
---

# Memory Paging — Hierarchical Agent Memory (Letta / MemGPT Pattern)

## Purpose
Treats agent memory like an operating system: separates high-priority active context from long-term archival storage, enabling unbounded multi-session tasks.

## Memory Hierarchy
1. **Core Memory (RAM):** User preferences, core constraints, active persona.
2. **Working Context:** Active task variables and condensed trajectory.
3. **Archival Storage (Disk/Viking):** Persistent vector store queried on-demand.
