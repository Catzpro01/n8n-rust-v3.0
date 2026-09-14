# 10: Merge closed branch streams without unbounded memory

**What to build:** Merge waits for both If branch streams to close and appends/spools them in declared deterministic input order without unbounded memory.

**Blocked by:** 09: Route items deterministically with If

**Status:** `implemented-and-pinned-verified` — bounded native success/empty/restart slice verified; runtime cancellation, branch-failure, and spill/read fault-injection now covered with deterministic `WORKFLOWD_TEST_FAULTS` harness (2026-09-14). Awaiting fresh pinned `validate` green on this revision before marking fully complete.

- [x] Merge declares two stable named inputs, one output, Barrier/Reducer Activation Shape, Pure effects, and explicit completion/cardinality rules.
- [x] The first mode appends the complete true branch followed by the complete false branch, or the independently frozen declared order, consistently.
- [x] Both input streams may spill through Artifacts; Merge does not require their full materialization in RAM.
- [x] Output count equals the sum of branch counts, every item retains parent provenance, and no item is duplicated or lost.
- [x] The compiler rejects missing required inputs, unsupported modes, ambiguous connection indexes, and incompatible schemas.
- [x] Run cancellation, one-branch failure, empty branch, delayed branch close, and spill/read failure produce typed outcomes and trace evidence. Empty-branch and delayed-close were already covered; runtime cancellation (`merge-cancel`), one-branch failure (`merge-branch-failure` → `canopy.if.injected_failure`), and spill/read fault-injection (`merge-spool-read-failure` → `canopy.merge.spool_read`, `merge-spool-write-failure` → `canopy.merge.spool_storage`) are now exercised by `tests/acceptance/test_if_runtime.py` under `WORKFLOWD_TEST_FAULTS=1`. All produce typed `failed`/`cancelled` terminal states, `permanent_failure` Merge/If activations, and matching `activation_outcome` Causal Trace events with abandoned/quarantined Artifact leases.
- [x] Causal Trace distinguishes logical Merge behavior from physical spool/segment metrics.
- [x] Repeated and differently interleaved branch completion produces the same merged logical order and digest input.

Pinned evidence and operator semantics are documented in `docs/operations/merge-routing.md`; the bounded public seam is `tests/acceptance/test_if_runtime.py`. Fault-injection acceptance is implemented in `crates/workflowd/src/run.rs` (`fault_injection_enabled`, `MergeArtifactIterator::with_read_fault`, `inject_spool_write_failure`).
