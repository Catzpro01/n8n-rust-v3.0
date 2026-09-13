---
name: prompt-injection-defense
description: >-
  Semantic firewalls (NeMo Guardrails) screening inputs for injection and outputs for sensitive data leakage.
---

# Security Guardrails & Prompt Injection Defense

## Architecture
- Input Rail: Detect jailbreaks, indirect injections, and prompt manipulation.
- Dialog Rail: Enforce canonical conversational boundaries.
- Output Rail: Screen for API key leakage, PII, and unauthorized commands.
