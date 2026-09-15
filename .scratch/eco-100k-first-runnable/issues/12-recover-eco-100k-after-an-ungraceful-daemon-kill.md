# 12: Recover Eco 100K after an ungraceful daemon kill

**What to build:** A real process kill after speculative progress resumes the exact Eco Run from its latest Durable Checkpoint, performs bounded replay, avoids duplicate committed output, and converges on the uninterrupted digest.

**Blocked by:** 11: Summarize the exact Eco 100K Run

**Status:** complete

- [x] The acceptance harness observes speculative progress ahead of a known durable checkpoint and sends an ungraceful process kill, not graceful shutdown.
- [x] Systemd restarts the daemon and startup loads the same revision, pinned plan, checkpoint, Artifact references, digest state, and logical resume cursor.
- [x] Incomplete post-checkpoint work is discarded/replayed within the declared bound; no partial checkpoint rows become durable.
- [x] Committed outputs, counters, trace sequence, and Artifact references are not duplicated.
- [x] Recovered execution completes exactly 100,000 logical Activations with the same branch counts and digest as an uninterrupted control run.
- [x] UI/SSE visibly transitions through disconnected/recovering/replaying/running/terminal states and reports replay window.
- [x] Multiple injected kill points around checkpoint and Artifact boundaries are covered through installed-process tests.
- [x] SQLite WAL/FULL, commit count, restart time, replay work, trace volume, disk I/O, and correctness evidence are captured.
- [x] Recovery target under ordinary daemon crash is measured against the accepted under-one-minute objective.

## Completion evidence — 2026-09-16

The daemon now persists an atomic recovery projection containing the pinned
Revision and Plan digests, logical resume cursor, digest state, checkpoint
sequence, Artifact-reference prefix, and bounded replay window. Startup
reconciles the speculative suffix before admitting work, publishes
`recovering` and `replaying` trace/SSE states, resumes `running`, and retains a
terminal recovery record. Artifact reconciliation preserves the exact durable
reference prefix and releases only speculative suffix references. The editor
shows disconnected/recovering/replaying/running/terminal state and replay facts
when recovery evidence exists.

`IfRuntimeAcceptance.test_eco_100k_recovers_after_two_ungraceful_kills` observes
live generation ahead of a Durable Checkpoint, delivers two real `SIGKILL`s at
thresholds straddling checkpoint and Artifact boundaries, and restarts the same
Run from the same state directory. The passing acceptance proves:

- replay remains at or below the declared 1,024-outcome checkpoint bound;
- checkpoint sequences and Causal Trace event sequences are contiguous;
- each committed Artifact reference exists exactly once;
- SQLite reports `journal_mode=wal` and `synchronous=full`;
- restart-to-resumed-execution remains below 60,000 ms;
- evidence reports checkpoint commits, replay work, trace volume, and positive
  Artifact bytes written;
- terminal correctness is exactly 100,000 attempted/succeeded logical
  Activations, 49,998 output Items, and 24,999 true / 24,999 false branches;
- the recovered digest is exactly
  `sha256:56193dff07ac42baab1774fc9f25ce52dd527a5c14a59ac083b372b0ff402e55`,
  identical to uninterrupted execution.

The installed-process seam in `scripts/test-systemd-smoke.sh` sends `SIGKILL`
to the production unit's `MainPID`, requires systemd `Restart=on-failure` to
produce a different PID, and verifies readiness plus WAL/FULL state reuse.
The dedicated recovery, uninterrupted Eco, and public If acceptances passed at
source head `cfff3ed1f7115d78c0fe8f8e78443d3f4596afe3`; no runtime source changed
between that head and the final verification head. Final head
`838e1e53adf0fe8628734cefc52f2b4085d475b4` then passed the repository-owned
bare-metal webhook context `vps-baremetal/fast-ci` via `make check` in 19
seconds. Automatic GitHub Actions execution is retired for push/PR validation;
the VPS webhook is the pinned verification gate.

This closes Ticket 12. Ticket 13 is unblocked but is not started by this
closure.
