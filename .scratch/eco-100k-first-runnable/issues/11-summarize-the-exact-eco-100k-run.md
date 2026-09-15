# 11: Summarize the exact Eco 100K Run

**What to build:** Summarize consumes the merged stream, emits count/branch
counters and a frozen algorithm-tagged Output Digest, and completes the full
six-node Eco journey with exactly 100,000 logical Activations.

**Blocked by:** 10: Merge closed branch streams without unbounded memory

**Status:** complete — pinned validation run
[34917759414](https://github.com/Catzpro01/n8n-rust-v3.0/actions/runs/34917759414)
passed the full gate.

- [x] Summarize declares Barrier/Reducer Activation Shape, Pure deterministic effects, one output, bounded state/spill policy, and the Output Digest operation.
- [x] The fixture equation is enforced: 1 Manual Trigger + 1 Generate Items + 49,998 Edit Fields + 49,998 If + 1 Merge + 1 Summarize = 100,000 Activations. Compiler validation rejects any non-exact six-edge Eco topology.
- [x] The independent fixture definition freezes input range, transform, predicate, merge order, canonicalization, branch/total counts, digest algorithm, and expected digest.
- [x] Summarize verifies total and branch counters and emits a small typed summary without materializing the merged payload in browser memory.
- [x] The browser renders aggregate wall/CPU timing, durable checkpoint progress, branch counts, and Output Digest.
- [x] Causal Trace exposes directly locatable retained Merge segment references and the retained ordinal range.
- [x] Route provenance is typed at the If, Merge, and Summarize seams; malformed or cross-port provenance is rejected instead of being coerced from an untyped JSON field.
- [x] Compiler connection validation rejects incompatible declared port schemas with structured diagnostics.
- [x] Focused regression coverage covers exact Eco edge rejection, schema incompatibility, typed provenance, concurrent login lockout, timing/segment rendering, and the full-transform vector.
- [x] The full browser/API test asserts aggregate timing and directly locatable retained Merge evidence in addition to create, publish, run, inspect, and non-destructive rollback.
- [x] Rust/workspace, editor, browser, Eco API/browser, dependency-audit, and release-bundle gates passed on the pinned revision.

## Implementation evidence

- `contracts/summarize.v1alpha1.json` publishes the versioned reducer contract.
- `crates/workflowd/src/compiler.rs` enforces the exact six-node/six-edge topology and typed source/target schema compatibility during compile, preview, and publish.
- `crates/workflowd/src/if_node.rs`, `merge.rs`, and `summarize.rs` carry typed If route provenance through the Merge and Summary reducers.
- `crates/workflowd/src/run.rs` persists aggregate wall time, cgroup-v2 CPU deltas when readable, Merge segment references, and Causal Trace links.
- `editor/src/editing-types.ts` and `editor/src/main.tsx` render timing and retained segment links without loading the merged payload.
- `crates/workflowd/src/security.rs` serializes the login read/verify/update state machine and has a concurrent-failure regression test.
- `docs/operations/eco-100k-summarize.md` freezes the independent topology, transform, counters, activation equation, and digest vector.

## Pinned verification

Run
[34917759414](https://github.com/Catzpro01/n8n-rust-v3.0/actions/runs/34917759414)
completed successfully on 2026-09-15 at head
`d4601d47d9fd598789ef9c90341e5c97a55ed66c`. Its green jobs include:

- Rust formatting and `cargo test --workspace --locked`;
- editor typecheck and dependency-free unit tests;
- two-tab, publish/rollback, run-trace, and artifact-generation browser acceptance;
- public If/Merge runtime fault-injection acceptance;
- Eco API and browser acceptance for the exact 100,000-Activation journey;
- Cargo and npm dependency audits;
- release-bundle build, verification, and artifact upload.

This closes Ticket 11 and unblocks Ticket 12. No Ticket 12 production behavior
is included in this ticket.
