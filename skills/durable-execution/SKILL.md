---
name: durable-execution
description: >-
  Fault-tolerant checkpointing systems (Temporal.io pattern) ensuring long-running workflows survive crashes.
---

# Durable Execution & State Persistence

## Guidelines
- Persist state machines to database at every boundary action.
- Guarantee idempotency across worker restarts.
- Resume long-running workflows seamlessly without state loss.
