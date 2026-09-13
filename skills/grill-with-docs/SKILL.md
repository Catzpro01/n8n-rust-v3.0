---
name: grill-with-docs
description: >-
  Stateful design interview by Matt Pocock. Conducts relentless grilling while
  compiling decisions into CONTEXT.md and Architectural Decision Records (ADRs).
---

# Grill-With-Docs — Stateful Architecture Interrogation (Matt Pocock)

## Purpose
The stateful sibling of `grill-me`. It conducts a rigorous design interview while actively persisting all resolved terms, definitions, and decisions directly into project files.

## Deliverables & Artifacts Generated
During and after the grilling session, the agent MUST maintain:
1. **`CONTEXT.md` (Ubiquitous Language / Glossary):**
   - Resolves ambiguous terms (e.g., what does "IDLE" actually mean? What is a "Leased Role"?).
   - Serves as the single source of truth for terminology between User and Agents.
2. **Architectural Decision Records (ADRs):**
   - Documents irreversible or high-stakes decisions:
     - Context & Problem Statement
     - Considered Options
     - Decision & Rationale
     - Consequences (Positive & Negative)

## Execution Rules
- Never write code during a `/grill-with-docs` session.
- Ask questions **one at a time** to keep focus sharp.
- After each key answer, update the in-memory glossary or `CONTEXT.md` immediately.
- Conclude with an explicit sign-off from the user before proceeding to implementation.
