---
name: rolling-memory-compaction
description: >-
  Rolling Memory Compaction Middleware. Automatically summarizes older conversation history into dense vector/structural representations.
---

# Rolling Memory Compaction Middleware

## Guidelines
- Monitor active context token usage against safety thresholds.
- When threshold is crossed, summarize previous turns into state snapshots.
- Preserve key variables, file paths, and decisions while evicting raw transcripts.
