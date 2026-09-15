# 11: Summarize the exact Eco 100K Run

**What to build:** Summarize consumes the merged stream, emits count/branch counters and a frozen algorithm-tagged Output Digest, and completes the full six-node Eco journey with exactly 100,000 logical Activations.

**Blocked by:** 10: Merge closed branch streams without unbounded memory

**Status:** complete - the pinned validate gate went green on run 34917759414

Ticket 10's runtime fault-injection acceptance is now pinned-verified (see that
ticket's evidence section), so Ticket 10 is no longer the upstream barrier. The
remaining gap is narrower and specific: the `Validate Rust workflow platform`
step `Browser acceptance: artifact generation` has never concluded successfully
in CI — it was skipped in the two green validate runs `34864197586` and
`34864802615`, and OOM-killed (exit 137) in the runs where it executed. The
measured cause and its fix are recorded in `.scratch/session-handoff.md`. Do not
promote this ticket to complete until that step and the full pinned gate pass on
a revision that contains this working tree.

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
- [ ] Rust/workspace and full browser acceptance evidence for this revision is still pending the pinned CI environment; Ticket 10's fault-injection gate is now closed.

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

The remaining dependency is intentionally honest: Ticket 10's runtime
cancellation, branch-failure, and spill/read fault-injection acceptance is now
closed by pinned CI evidence, so what is left is the full Ticket 11
browser/API and Rust gate on a revision whose `Browser acceptance: artifact
generation` step actually executes instead of being skipped or killed.
