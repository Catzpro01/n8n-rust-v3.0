---
name: tdd-auto-loop
description: Strict Test-Driven Development autonomous loop. Enforces Red-Green-Refactor verification cycles before any code is marked done.
---

# TDD Auto Loop (Red-Green-Refactor Engine)

## Purpose
Ensures zero hallucinations and 100% functional correctness for every code change by enforcing the Red-Green-Refactor discipline.

## Execution Cycle:
1. **RED**: Write a minimal failing test defining the expected behavior.
2. **GREEN**: Write the minimal implementation required to make the test pass.
3. **REFACTOR**: Clean up code structure, type safety, and formatting while keeping tests green.

## Rules:
- Never declare a feature or bugfix complete without an automated test verifying the exact edge-case.
