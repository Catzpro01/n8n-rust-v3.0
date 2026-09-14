# 11: Summarize the exact Eco 100K Run

**What to build:** Summarize consumes the merged stream, emits count/branch counters and a frozen algorithm-tagged Output Digest, and completes the full six-node Eco journey with exactly 100,000 logical Activations.

**Blocked by:** 10: Merge closed branch streams without unbounded memory

**Status:** implementation-complete-pending-ticket-10-acceptance

Ticket 11's production seams are implemented in the working tree, but this ticket
must not be marked fully complete while Ticket 10's dedicated runtime
fault-injection acceptance remains open. The compiler/publication gate is now the
owner of the exact Eco topology; the executor still retains Ticket 10 as the
upstream barrier for the full acceptance claim.

- [x] Summarize declares Barrier/Reducer Activation Shape, Pure deterministic effects, one output, bounded state/spill policy, and the Output Digest operation.
- [x] The fixture equation is enforced: 1 Manual Trigger + 1 Generate Items + 49,998 Edit Fields + 49,998 If + 1 Merge + 1 Summarize = 100,000 Activations. Compiler validation now rejects any non-exact six-edge Eco topology.
- [x] The independent fixture definition freezes input range, transform, predicate, merge order, canonicalization, branch/total counts, digest algorithm, and expected digest before completion is claimed.
- [x] Summarize verifies total and branch counters and emits a small typed summary without materializing the merged payload in browser memory.
- [x] The browser renders aggregate wall/CPU timing, durable checkpoint progress, branch counts, and Output Digest.
- [x] Causal Trace exposes directly locatable retained Merge segment references and the retained ordinal range.
- [x] Route provenance is typed at the If, Merge, and Summarize seams; malformed or cross-port provenance is rejected instead of being coerced from an untyped JSON field.
- [x] Compiler connection validation rejects incompatible declared port schemas with structured diagnostics.
- [x] Focused regression coverage was added for exact Eco edge rejection, schema incompatibility, typed provenance, concurrent login lockout, timing/segment rendering, and the existing full-transform vector.
- [x] The full browser/API test now asserts aggregate timing and directly locatable retained Merge evidence in addition to create, publish, run, inspect, and non-destructive rollback.
- [ ] Rust/workspace and full browser acceptance evidence for this revision is still pending the pinned CI environment and Ticket 10 runtime fault-injection gate.

## Implementation evidence

- `crates/workflowd/src/compiler.rs` enforces the exact six-node/six-edge topology and typed source/target schema compatibility during compile/preview/publish.
- `crates/workflowd/src/if_node.rs`, `merge.rs`, and `summarize.rs` carry typed If route provenance through the Merge and Summary reducers.
- `crates/workflowd/src/run.rs` persists aggregate wall time, cgroup-v2 CPU deltas when readable, Merge segment references, and Causal Trace links.
- `editor/src/editing-types.ts` and `editor/src/main.tsx` render timing and retained segment links without loading the merged payload.
- `crates/workflowd/src/security.rs` serializes the login read/verify/update state machine and has a concurrent-failure regression test.
- `.github/workflows/validate.yml` runs the workspace build, installs Chromium, and invokes the complete browser acceptance suite before Python tests.

## Verification recorded for this workspace

- `npm run typecheck` — passed.
- `npm run build` — passed.
- `node --check tests/eco-summarize.mjs` — passed.
- Changed contract JSON files parse successfully.
- Source-only `git diff --check` passes; the tracked recovered split-PDF artifact remains excluded from that diagnostic as documented in the handoff.
- Local `cargo`, `rustc`, and `rustfmt` are unavailable. Do not convert the pending Rust/CI gate into a completion claim until pinned CI runs this revision.

The remaining dependency is intentionally honest: close Ticket 10's runtime
cancellation, branch-failure, and spill/read fault-injection acceptance, then
run the full Ticket 11 browser/API and Rust gate before promoting this status to
complete.
