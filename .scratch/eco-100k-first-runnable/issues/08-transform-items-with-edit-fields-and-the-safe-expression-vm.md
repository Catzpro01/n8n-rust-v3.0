# 08: Transform items with Edit Fields and the safe expression VM

**What to build:** The Owner can configure Edit Fields with fixed values and supported expressions, preview diagnostics, and deterministically transform every generated item while preserving types and item linking.

**Blocked by:** 07: Stream Generate Items through bounded Envelopes and Artifacts

**Status:** approved-for-implementation

## Shared-understanding gate

The Owner asked the agent to continue after Ticket 07 on 2026-09-11. The four bounded decision recommendations below were explicitly approved in Arena on 2026-09-13. Production implementation is authorized within these boundaries; any expansion requires a new decision record.

## Frozen Owner decisions — round 1

Approval record: the Owner selected “Setujui 08-A sampai 08-D” in Arena on 2026-09-13. The following scope is authorized for implementation and must not be silently broadened.

### 08-A — Frozen Eco transformation

Frozen: retain the generated `{index,value,data}` item and add four fields: fixed `eco=true`; expression `parity = $json.value % 2 === 0 ? "even" : "odd"`; expression `doubled = $json.value * 2`; and expression `label = "eco-" + $json.index`. This proves fixed and expression values, typed boolean/number/string outputs, predictable future If routing, and one output per input without unnecessary fixture complexity.

### 08-B — Native configuration and assignment visibility

Frozen: use ordered assignments tagged as either `{kind:"fixed",value:...}` or `{kind:"expression",source:"..."}`, with paths represented as explicit JSON path segments. Support `merge` and `replace` output modes, freeze Eco to `merge`, reject duplicate/conflicting paths, and evaluate every expression against the original immutable input rather than partially modified output. A later compatibility adapter may translate documented external dot-notation behavior without making ambiguous strings the native contract.

### 08-C — First safe expression subset

Frozen: accept JSON-compatible literals; `$json` and `$itemIndex`; field and array-index access; parentheses; unary `!` and `-`; checked `+ - * / %`; strict `=== !==`; typed `< <= > >=`; `&& || ??`; and the ternary operator. String concatenation is allowed only when both operands are strings. Fixed assignments carry nested arrays/objects; arbitrary JavaScript, property mutation, prototypes, constructors, method calls, loops, regex, eval, imports, and host access are rejected before publication with compatibility classification.

### 08-D — Missing, time, and randomness

Frozen: preserve an internal Missing value distinct from JSON null. Missing path reads propagate Missing through `??` and otherwise produce a structured evaluation error; assigning a final Missing result omits that target path. Do not expose ambient or logical time/random variables in this first subset. Diagnose them as unsupported before publication; add logical-clock or seeded-random operators only in a future separately locked contract revision.

- [ ] Edit Fields declares one Per-Item Stream input/output, Pure deterministic effects, bounded expansion, and no capabilities.
- [ ] Its declarative form distinguishes fixed values from expressions and supports the exact Eco transformation fields.
- [ ] The Rust expression VM compiles the accepted common subset and produces deterministic typed results using logical context only.
- [ ] Tests preserve missing versus null, strings versus numbers/booleans, nested arrays/objects, Unicode, escaping, and item-link provenance.
- [ ] Unsupported JavaScript semantics are identified before publication and classified for future delegation rather than evaluated in the daemon.
- [ ] Syntax, type, missing-node/item, resource-limit, and runtime errors use stable original structured codes and safe evidence.
- [ ] Logical clock and seeded randomness are controlled; ambient time/random/environment access is unavailable.
- [ ] Backpressure, cancellation, output limits, and forced Artifact spill remain effective across the transform.
- [ ] Repeated and restart/replay executions produce byte-for-byte canonical logical outputs and the same trace facts.

## Implementation progress — 2026-09-13

- [x] Owner approval recorded in this ticket and ADR 0057.
- [x] `edit-fields.v1alpha1` declares a pure deterministic per-item contract
      with bounded resources and no capabilities.
- [x] The compiler/catalog/release paths recognize the Edit Fields contract.
- [x] The pure Rust safe-expression compiler/evaluator and immutable-input
      Edit Fields assignment module have focused unit tests.
- [ ] Wire transformed Envelopes into the durable Run scheduler while
      preserving Ticket 07's checkpoints, backpressure, cancellation, Artifact
      spill, item linking, and replay invariants.
- [ ] Add public-seam and browser configuration/diagnostic acceptance coverage.

The local sandbox has no Cargo toolchain, so the new Rust unit tests still need
fresh verification in the pinned GitHub/Rust-enabled environment. The
repository validation workflow is `.github/workflows/validate.yml`; the current
GitHub integration did not permit manual workflow dispatch.

## Verification finding — 2026-09-13

The pinned Rust/Node GitHub workflow reached compilation and ran 35
`workflowd` tests plus the node-contract tests. Formatting, editor build, and
compilation passed. Two focused tests fail for the same frozen Eco label:
`"eco-" + $itemIndex` in the current test fixture (the approved ticket text
uses `"eco-" + $json.index`). The generated `index` is a JSON number, while
08-C explicitly permits string concatenation only when both operands are
strings. The safe VM correctly rejects the mixed operation with
`canopy.expression.type`, so this is a frozen-contract inconsistency rather
than a runtime coercion bug.

Implementation is paused at this boundary pending Owner choice:

1. retain 08-C's strict typing and revise the Eco label expression/fixture to
   provide a string operand (no numeric-to-string coercion);
2. approve a separately recorded deterministic numeric-to-string conversion
   for the label; or
3. change Generate Items' public `index` type from number to string, which
   would require revisiting Ticket 07's existing item and digest tests.

No coercion or Generate Items schema change has been made silently.
