# 10: Merge closed branch streams without unbounded memory

**What to build:** Merge waits for both If branch streams to close and appends/spools them in declared deterministic input order without unbounded memory.

**Blocked by:** 09: Route items deterministically with If

**Status:** implemented-and-pinned-verified for the bounded native success/empty/restart slice; cancellation and fault-injection coverage remain follow-up work.

- [x] Merge declares two stable named inputs, one output, Barrier/Reducer Activation Shape, Pure effects, and explicit completion/cardinality rules.
- [x] The first mode appends the complete true branch followed by the complete false branch, or the independently frozen declared order, consistently.
- [x] Both input streams may spill through Artifacts; Merge does not require their full materialization in RAM.
- [x] Output count equals the sum of branch counts, every item retains parent provenance, and no item is duplicated or lost.
- [x] The compiler rejects missing required inputs, unsupported modes, ambiguous connection indexes, and incompatible schemas.
- [ ] Run cancellation, one-branch failure, empty branch, delayed branch close, and spill/read failure produce typed outcomes and trace evidence. Empty-branch and delayed-close behavior are covered; dedicated runtime cancellation, one-branch-failure, and spill/read fault-injection acceptance remains.
- [x] Causal Trace distinguishes logical Merge behavior from physical spool/segment metrics.
- [x] Repeated and differently interleaved branch completion produces the same merged logical order and digest input.

Pinned evidence and operator semantics are documented in `docs/operations/merge-routing.md`; the bounded public seam is `tests/acceptance/test_if_runtime.py`.
