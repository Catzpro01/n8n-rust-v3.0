---
name: latent-cot-envelopes
description: >-
  Latent Chain-of-Thought & Internal Reasoning Envelopes. Separates internal reasoning steps from tool execution payloads to minimize logic errors.
---

# Latent Chain-of-Thought & Reasoning Envelopes

## Guidelines
- Enforce rigid boundary between `<thinking>` and executable action tokens.
- Complete full reasoning trace before generating tool invocation parameters.
- Suppress premature actions caused by interleaved streaming.
