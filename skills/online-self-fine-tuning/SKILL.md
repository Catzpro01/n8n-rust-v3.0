---
name: online-self-fine-tuning
description: >-
  On-the-fly local parameter adaptation (LoRA) based on historical task success and experience replay.
---

# Online Self-Fine-Tuning & Experience Replay

## Methodology
- Buffer successful trajectory pairs (input, execution trace, result).
- Periodically trigger localized LoRA adapter updates.
- Continuously align agent behavior with unique project conventions.
