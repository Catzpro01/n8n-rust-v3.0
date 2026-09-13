# 09: Route items deterministically with If

**What to build:** If routes every transformed item to exactly one named true/false output with stable ordering, counts, provenance, diagnostics, and Causal Trace.

**Blocked by:** 08: Transform items with Edit Fields and the safe expression VM

**Status:** in-progress (2026-09-14)

- [x] If declares one typed/dynamic-item input, two stable named outputs, Per-Item Stream Activation Shape, Pure effects, and no capabilities.
- [x] The editor catalog exposes the If contract and its bounded declarative condition configuration.
- [ ] Every input item reaches exactly one output; no item is duplicated, dropped, or sent to both branches.
- [ ] True/false item order is deterministic across repeated runs, restarts, and resource profiles.
- [ ] Missing/null/type/coercion boundary cases and AND/OR composition used by the first contract are covered by conformance tests.
- [ ] Invalid Port Schema, predicate, or output configuration blocks publication with structured diagnostics.
- [ ] Branch output, count, item-link provenance, and logical route decision are inspectable in Causal Trace without retaining all payloads in browser memory.
- [ ] Cancellation, queue pressure, and downstream backpressure do not violate exactly-one-branch behavior.

## Implementation progress — 2026-09-14

- [x] Added `canopy.native/if` v1alpha1 with stable `true` and `false`
      dynamic-item outputs, pure deterministic effects, bounded resources, and
      no capabilities.
- [x] Added the Rust bounded condition compiler/evaluator with `all`/`any`
      composition, strict boolean results, missing/null/type diagnostics, and
      deterministic route results.
- [x] Added the contract to the Rust compiler, Draft catalog/validation, and
      browser catalog build.
- [ ] Wire batched If routing through the durable Run scheduler, checkpoints,
      branch output persistence, item-link trace, and browser acceptance.

The runtime wiring remains the next implementation step; this ticket is not
complete until the exactly-one-branch, backpressure, cancellation, and release
evidence checks pass in pinned CI.
