---
name: self-healing-recovery
description: >-
  Autonomous background process and subagent health monitor. Detects crashed, timed-out, or zombie tasks and recovers or restarts them seamlessly.
---

# Self-Healing Recovery — Autonomous Task Health Monitor

## Purpose
Monitors long-running background tasks, compiler processes, and subagent lifecycles, automatically recovering execution state if a crash, disconnect, or server restart occurs.

## Recovery Protocol
1. **Heartbeat Detection:** Check status of active background tasks before each execution step.
2. **Zombie Cleanup:** Terminate hung tasks exceeding timeout thresholds.
3. **State Restoration:** Load the most recent task checkpoint and resume execution from the last confirmed step.
4. **Idempotent Replay:** Re-run failed steps with verified cached outputs to avoid duplicate side effects.
