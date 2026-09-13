---
status: accepted
---
# Approve Edit Fields and the bounded safe-expression VM

## Context

Ticket 08 extends the completed deterministic Generate Items path with a pure
Edit Fields transformation. The Owner explicitly approved the bounded 08-A
through 08-D decisions in Arena on 2026-09-13. The implementation must preserve
Ticket 07's Envelope, Artifact, backpressure, cancellation, checkpoint, replay,
and provenance invariants.

## Decision

### Transformation contract

The native Edit Fields contract keeps each input item's `{index, value, data}`
shape and adds the frozen Eco fields:

- fixed `eco=true`;
- `parity = $json.value % 2 === 0 ? "even" : "odd"`;
- `doubled = $json.value * 2`;
- `label = "eco-" + $json.index`.

It is a one-input, one-output, pure deterministic transformation with bounded
expansion and no capabilities.

### Assignments and output modes

Assignments are ordered and explicitly tagged as either fixed values or
expressions:

```json
{"kind":"fixed","value":...}
{"kind":"expression","source":"..."}
```

Paths are explicit JSON path segments, not ambiguous dot-notation strings. The
native contract supports `merge` and `replace`; the Eco workflow is frozen to
`merge`. Duplicate and conflicting paths are rejected. Every expression reads
the immutable original input, not a partially modified output. A future
compatibility adapter may translate documented external dot-notation behavior
without changing the native contract.

### Safe expression subset

The first compiler/VM accepts JSON-compatible literals, `$json`,
`$itemIndex`, field and array-index access, parentheses, unary `!` and `-`,
checked arithmetic `+ - * / %`, strict equality `=== !==`, typed comparisons
`< <= >= >`, boolean/coalescing operators `&& || ??`, and the ternary operator.
String concatenation is allowed only for two strings. Fixed assignments may hold
nested arrays and objects.

Arbitrary JavaScript, mutation, prototypes, constructors, method calls, loops,
regex, `eval`, imports, and host access are rejected before publication and
classified for a future compatibility lane rather than evaluated by the daemon.

### Missing, time, and randomness

Missing is an internal value distinct from JSON `null`. Missing path reads
propagate through `??`; otherwise use produces a structured evaluation error. A
final Missing assignment omits its target path. Ambient time and randomness are
not exposed in this contract. Logical-clock or seeded-random operators require
a later locked revision.

## Consequences

The compiler and VM can be tested as a pure deterministic module, separately
from SQLite and HTTP. The transformation can stream through bounded execution
and preserve the existing Artifact spill, backpressure, cancellation, and
checkpoint behavior. Compatibility behavior remains explicit instead of
silently accepting arbitrary JavaScript.

Production implementation is authorized only within this scope. Any expansion
of the expression language or side-effect surface requires a new decision
record and approval.
