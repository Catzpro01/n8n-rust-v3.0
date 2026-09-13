# 08: Transform items with Edit Fields and the safe expression VM

**What to build:** The Owner can configure Edit Fields with fixed values and supported expressions, preview diagnostics, and deterministically transform every generated item while preserving types and item linking.

**Blocked by:** 07: Stream Generate Items through bounded Envelopes and Artifacts

**Status:** owner-decision-round-1

## Shared-understanding gate

The Owner asked the agent to continue after Ticket 07 on 2026-09-11. Ticket 08 is undergoing bounded `/grill-with-docs` decision rounds before production implementation. No production implementation is authorized until the Owner explicitly confirms the resulting decisions.

## Pending Owner decisions — round 1

### 08-A — Frozen Eco transformation

Recommended: retain the generated `{index,value,data}` item and add four fields: fixed `eco=true`; expression `parity = $json.value % 2 === 0 ? "even" : "odd"`; expression `doubled = $json.value * 2`; and expression `label = "eco-" + $json.index`. This proves fixed and expression values, typed boolean/number/string outputs, predictable future If routing, and one output per input without unnecessary fixture complexity.

### 08-B — Native configuration and assignment visibility

Recommended: use ordered assignments tagged as either `{kind:"fixed",value:...}` or `{kind:"expression",source:"..."}`, with paths represented as explicit JSON path segments. Support `merge` and `replace` output modes, freeze Eco to `merge`, reject duplicate/conflicting paths, and evaluate every expression against the original immutable input rather than partially modified output. A later compatibility adapter may translate documented external dot-notation behavior without making ambiguous strings the native contract.

### 08-C — First safe expression subset

Recommended: accept JSON-compatible literals; `$json` and `$itemIndex`; field and array-index access; parentheses; unary `!` and `-`; checked `+ - * / %`; strict `=== !==`; typed `< <= > >=`; `&& || ??`; and the ternary operator. String concatenation is allowed only when both operands are strings. Fixed assignments carry nested arrays/objects; arbitrary JavaScript, property mutation, prototypes, constructors, method calls, loops, regex, eval, imports, and host access are rejected before publication with compatibility classification.

### 08-D — Missing, time, and randomness

Recommended: preserve an internal Missing value distinct from JSON null. Missing path reads propagate Missing through `??` and otherwise produce a structured evaluation error; assigning a final Missing result omits that target path. Do not expose ambient or logical time/random variables in this first subset. Diagnose them as unsupported before publication; add logical-clock or seeded-random operators only in a future separately locked contract revision.

- [ ] Edit Fields declares one Per-Item Stream input/output, Pure deterministic effects, bounded expansion, and no capabilities.
- [ ] Its declarative form distinguishes fixed values from expressions and supports the exact Eco transformation fields.
- [ ] The Rust expression VM compiles the accepted common subset and produces deterministic typed results using logical context only.
- [ ] Tests preserve missing versus null, strings versus numbers/booleans, nested arrays/objects, Unicode, escaping, and item-link provenance.
- [ ] Unsupported JavaScript semantics are identified before publication and classified for future delegation rather than evaluated in the daemon.
- [ ] Syntax, type, missing-node/item, resource-limit, and runtime errors use stable original structured codes and safe evidence.
- [ ] Logical clock and seeded randomness are controlled; ambient time/random/environment access is unavailable.
- [ ] Backpressure, cancellation, output limits, and forced Artifact spill remain effective across the transform.
- [ ] Repeated and restart/replay executions produce byte-for-byte canonical logical outputs and the same trace facts.
