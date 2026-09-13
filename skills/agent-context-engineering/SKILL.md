---
name: agent-context-engineering
description: >-
  Agent Skills for Context Engineering (muratcankoylan). Compaction, masking, and prefix caching patterns to mitigate context degradation.
---

# Context Engineering Patterns

## Core Techniques
- **Masking:** Hide verbose stdout and intermediate logs from context once processed.
- **Compaction:** Synthesize multi-step execution logs into high-level state representations.
- **Prefix Caching Alignment:** Keep system prompts and immutable instructions strictly at the top of context to leverage provider-level KV-cache hits.
