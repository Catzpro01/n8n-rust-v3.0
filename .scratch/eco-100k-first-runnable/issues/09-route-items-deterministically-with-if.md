# 09: Route items deterministically with If

**What to build:** If routes every transformed item to exactly one named true/false output with stable ordering, counts, provenance, diagnostics, and Causal Trace.

**Blocked by:** 08: Transform items with Edit Fields and the safe expression VM

**Status:** in-progress (2026-09-14)

- [x] If declares one typed/dynamic-item input, two stable named outputs, Per-Item Stream Activation Shape, Pure effects, and no capabilities.
- [x] The editor catalog exposes the If contract and its bounded declarative condition configuration.
- [x] Every input item reaches exactly one output; no item is duplicated, dropped, or sent to both branches.
- [x] True/false item order is deterministic across repeated runs, restarts, and resource profiles.
- [x] Missing/null/type/coercion boundary cases and AND/OR composition used by the first contract are covered by conformance tests.
- [x] Invalid Port Schema, predicate, or output configuration blocks publication with structured diagnostics.
- [x] Branch output, count, item-link provenance, and logical route decision are inspectable in Causal Trace without retaining all payloads in browser memory.
- [x] Cancellation, queue pressure, and downstream backpressure do not violate exactly-one-branch behavior.

## Implementation progress — 2026-09-14

- [x] Added `canopy.native/if` v1alpha1 with stable `true` and `false`
      dynamic-item outputs, pure deterministic effects, bounded resources, and
      no capabilities.
- [x] Added the Rust bounded condition compiler/evaluator with `all`/`any`
      composition, strict boolean results, missing/null/type diagnostics, and
      deterministic route results.
- [x] Added the contract to the Rust compiler, Draft catalog/validation, and
      browser catalog build.
- [x] Wire batched If routing through the durable Run scheduler, checkpoints,
      branch output persistence, item-link trace, and cancellation/backpressure
      handling.
- [x] Add replay-stable route-chain evidence, durable true/false progress, and
      transformed logical-item coverage.
- [ ] Complete the public/editor acceptance journey and attach release evidence.

The native runtime implementation is in `crates/workflowd/src/if_node.rs` and
`crates/workflowd/src/run.rs`; its operational contract is documented in
`docs/operations/if-routing.md`. Focused Rust coverage and pinned repository
validation are green in workflow run `34777304858` (push, 2026-09-14). The
contract/catalog work remains committed as `71e8c6a` plus formatting fix
`292e3ca`; earlier push CI `34775814353` and pull-request CI `34775816647`
passed the editor build, Rust formatting/tests, and repository checks. Ticket 09
remains open until the public/editor acceptance and release-evidence gate is
closed.
