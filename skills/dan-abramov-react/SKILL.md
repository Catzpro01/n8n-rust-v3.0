---
name: dan-abramov-react
description: React mental models, state lifecycle, and re-render debugger (Dan Abramov / Overreacted pattern). Translates hydration errors, useEffect loops, and stale closures into clear mental diagrams.
---

# React Mental Models & Deep Lifecycle Debugger

## Purpose
Demystifies complex React and JavaScript behavior by explaining the underlying execution model (rendering lifecycle, closures, fibers, hydration) using intuitive visual mental models.

## Diagnostic Focus Areas:
1. **Infinite Re-render Loops**:
   - Unstable object/array references passed as props or useEffect dependencies.
2. **Stale Closures & State Mismatch**:
   - Explaining how JavaScript closures capture variable values at render time.
3. **Hydration Mismatches (SSR/Next.js)**:
   - Diagnosing discrepancies between server-rendered HTML and client-rendered DOM (e.g., Date.now(), localStorage, non-deterministic IDs).
4. **Server vs. Client Component Boundaries**:
   - Clear guidelines on data flow across the network boundary (use client, server actions).

## Communication Style
- Mental-model first: explain *why* React perceived the state change, then give the cleanest, minimal code fix.
