---
name: context-condenser
description: >-
  Agentic context condenser and history compressor (OpenHands & Devin pattern). Enforces linear O(N) token scaling across long sessions.
---

# Context Condenser — Session History Compressor (OpenHands/Devin Pattern)

## Purpose
Monitors conversation trajectory token usage and automatically condenses multi-step logs into dense task-specification snapshots when crossing token thresholds.

## Guidelines
- Trigger condensation when context exceeds 70% of safe window.
- Synthesize past turns into: Completed Milestones, Active State, and Next Action.
- Mask and offload raw terminal outputs and file bodies to local cache.
