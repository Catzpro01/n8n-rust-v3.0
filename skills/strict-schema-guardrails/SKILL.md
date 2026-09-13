---
name: strict-schema-guardrails
description: >-
  Strict structural output validation preventing downstream system breaks (Instructor / Pydantic).
---

# Strict Schema & Guardrails Enforcement

## Guidelines
- Coerce LLM outputs into strict Pydantic models.
- Auto-retry with validation errors upon schema violation.
- Guarantee 100% type-safe JSON payloads for external APIs.
