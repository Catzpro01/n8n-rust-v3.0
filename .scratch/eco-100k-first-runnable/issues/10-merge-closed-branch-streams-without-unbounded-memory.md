# 10: Merge closed branch streams without unbounded memory

**What to build:** Merge waits for both If branch streams to close and appends/spools them in declared deterministic input order without unbounded memory.

**Blocked by:** 09: Route items deterministically with If

**Status:** complete; the bounded native success/empty/restart slice and the dedicated runtime cancellation, one-branch-failure, and spill/read fault-injection acceptance are both pinned-verified.

- [x] Merge declares two stable named inputs, one output, Barrier/Reducer Activation Shape, Pure effects, and explicit completion/cardinality rules.
- [x] The first mode appends the complete true branch followed by the complete false branch, or the independently frozen declared order, consistently.
- [x] Both input streams may spill through Artifacts; Merge does not require their full materialization in RAM.
- [x] Output count equals the sum of branch counts, every item retains parent provenance, and no item is duplicated or lost.
- [x] The compiler rejects missing required inputs, unsupported modes, ambiguous connection indexes, and incompatible schemas.
- [x] Run cancellation, one-branch failure, empty branch, delayed branch close, and spill/read failure produce typed outcomes and trace evidence. Covered by `test_merge_cancellation_is_terminal_and_traceable`, `test_merge_branch_failure_is_typed_and_traceable`, and `test_merge_spool_read_failure_is_typed_and_traceable` in `tests/acceptance/test_if_runtime.py`.
- [x] Causal Trace distinguishes logical Merge behavior from physical spool/segment metrics.
- [x] Repeated and differently interleaved branch completion produces the same merged logical order and digest input.

Pinned evidence and operator semantics are documented in `docs/operations/merge-routing.md`; the bounded public seam is `tests/acceptance/test_if_runtime.py`.

## Fault-injection acceptance evidence — 2026-09-14

The whole public If/Merge module, including the three dedicated fault-injection
cases, passed in the pinned self-hosted workflow:

- Workflow `If runtime acceptance`, run
  [34881819422](https://github.com/Catzpro01/n8n-rust-v3.0/actions/runs/34881819422),
  job `if-runtime`, conclusion `success`, 18:42:49Z–19:01:13Z, head
  `46f675bc79530af1ed9304215f7605818b8e129f`.
- The `Run public If acceptance` step executes
  `python3 -m unittest tests.acceptance.test_if_runtime -v` against a daemon
  started with `WORKFLOWD_TEST_FAULTS=1`
  (`tests/acceptance/test_if_runtime.py:29`), which is the gate
  `fault_injection_enabled` requires (`crates/workflowd/src/run.rs:3537`).
- The file at that head still contains the three cases at lines 507, 572, and
  621; confirmed by reading the blob at that ref, not from memory.
- Companion evidence: `Eco 100K Summarize acceptance` run
  [34871356127](https://github.com/Catzpro01/n8n-rust-v3.0/actions/runs/34871356127)
  passed every step, head `7cc727c93554158e0ab1853a0afcf3afc57fd066`.
