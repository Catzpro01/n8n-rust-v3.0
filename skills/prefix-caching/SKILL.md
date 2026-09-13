---
name: prefix-caching
description: >-
  Automatic Prefix Caching & KV-Cache State Management. Reorders system prompts and tool schemas to align with provider KV-caches, cutting input token costs by 50-90%.
---

# Prefix Caching & KV-Cache Management

## Guidelines
- Structure prompts hierarchically: Immutable system instructions at top, tool definitions in middle, volatile conversation at bottom.
- Maximize KV-cache reuse on providers supporting prefix caching (Anthropic, vLLM, DeepSeek).
- Cut latency and cost up to 90% on long multi-turn sessions.
