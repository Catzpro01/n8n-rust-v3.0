---
name: fuzz-and-harden-loop
description: Autonomous edge-case stress tester and vulnerability hardening loop. Generates extreme input boundary conditions to bulletproof production code.
---

# Fuzz & Harden Loop

## Purpose
Stress-tests finished features against malformed payloads, race conditions, edge cases, and unexpected inputs before production deployment.

## Capabilities:
1. **Boundary & Malformed Input Generation**:
   - Tests null, undefined, empty strings, massive arrays, special UTF-8 characters, and max integer limits.
2. **Race Condition & Concurrency Checks**:
   - Simulates parallel requests and async state mutations.
3. **Hardening Remediation**:
   - Inserts schema validation guardrails (Zod/Pydantic) for any vulnerability uncovered.
