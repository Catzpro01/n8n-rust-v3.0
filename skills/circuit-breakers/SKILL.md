---
name: circuit-breakers
description: >-
  Infinite Loop & Token Burn Circuit Breakers. Passive monitor cutting execution loops and token burning spikes in real-time.
---

# Infinite Loop & Token Burn Circuit Breakers

## Rules & Triggers
- **Tool Repeat Limit:** Trip breaker if exact same tool + arguments is called 3x consecutively.
- **Error Loop Limit:** Trip breaker if same exception repeats 2x without code change.
- **Velocity Limit:** Trip breaker if token burn rate exceeds threshold per minute.
- Action: Pause execution, alert Fern/Matt, and request user guidance.
