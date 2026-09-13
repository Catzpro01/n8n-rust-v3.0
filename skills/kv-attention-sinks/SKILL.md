---
name: kv-attention-sinks
description: >-
  KV-cache preservation and attention sink management (StreamingLLM & SnapKV). Ensures infinite multi-turn stability without GPU VRAM exhaustion.
---

# KV Attention Sinks — Cache Preservation (StreamingLLM / SnapKV)

## Purpose
Preserves initial attention sink tokens and clusters active attention windows to guarantee unbounded conversational stability without memory overflow or context drift.

## Guidelines
- Retain the first 4 system/prompt tokens (attention sinks) immutably at the top of context.
- Group dynamic messages in sliding attention clusters.
- Align prompt structure strictly with provider KV-cache boundaries.
