---
name: addyosmani-agent-skills
description: >-
  Structured engineering workflow by Addy Osmani (Google Chrome team). 
  Enforces spec-first development with /spec, /plan, /build, /test stages
  to reduce agent hallucinations and ensure quality output.
---

# Addy Osmani Agent Engineering Skills

## Overview
This skill enforces a structured, stage-gated approach to software engineering tasks. Rather than jumping directly to code, the agent first clarifies requirements, writes a spec, plans the implementation, then executes — reducing errors and hallucinations.

## Workflow Stages

### Stage 1: /spec
Before writing any code, produce a detailed specification:
- Clarify ambiguous requirements with targeted questions
- Define inputs, outputs, edge cases, and constraints
- Identify dependencies and potential risks
- Output a Markdown spec document for user approval

### Stage 2: /plan
With an approved spec, produce an implementation plan:
- Break the work into discrete, testable units
- Identify files to create/modify
- Estimate complexity and risk for each unit
- List acceptance criteria for each unit

### Stage 3: /build
Execute the plan with discipline:
- Implement one unit at a time
- Never skip validation or error handling for brevity
- Write code that is readable and maintainable over clever
- Commit to a branch with descriptive commit messages

### Stage 4: /test
Verify correctness systematically:
- Write unit tests before or alongside implementation (TDD preferred)
- Cover edge cases identified in /spec
- Run linters, type-checkers, and formatters
- Only mark DONE when all checks pass

## Rules
- ALWAYS start with /spec for any non-trivial task
- NEVER produce code without a plan when task complexity is HIGH
- When uncertain, ask — do not assume or hallucinate
- Document decisions and trade-offs inline with code
