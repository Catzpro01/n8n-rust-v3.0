---
name: self-healing-debug-loop
description: Autonomous error convergence loop. Traps compiler and runtime errors, isolates root causes with strategic logging, patches minimally, and iterates until zero errors.
---

# Self-Healing Debug Loop

## Purpose
Prevents autonomous workflows from getting stuck in deadlocks by automatically diagnosing, patching, and re-verifying errors.

## Healing Protocol:
1. **Stacktrace & AST Root Cause Triage**:
   - Reads exact failure lines and compiler diagnostics without guesswork.
2. **Strategic Caveman Probing**:
   - Inserts targeted log probes if state flow is ambiguous.
3. **Minimal Surgical Patch**:
   - Applies the smallest possible diff to fix the error.
4. **Iterative Verification**:
   - Re-runs the test suite until error count reaches strictly **0**.
