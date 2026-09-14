---
status: accepted
---
# Revise Edit Fields to v1alpha2 for bounded integer label conversion

## Context

The first Ticket 08 implementation reached the pinned Rust CI compiler and
exposed an inconsistency between the approved Eco transformation and the
approved expression subset. Generate Items intentionally emits a numeric
`index`, while the frozen label is `"eco-" + $json.index`. ADR 0057's first
subset allowed string concatenation only when both operands were strings, so
the exact Eco label was correctly rejected by the VM.

The Owner selected the bounded safe-conversion option in Arena on 2026-09-13.
This is a narrow follow-up to ADR 0057, not permission to evaluate JavaScript
or to add general coercion.

## Decision

Publish the active Edit Fields contract as
`canopy.native/edit-fields/v1alpha2` and leave the v1alpha1 contract bytes
available as an immutable historical compatibility artifact. The editor and
native compiler catalog use v1alpha2.

For v1alpha2 only, the `+` operator accepts exactly these additional operands:

- `string + JSON integer` converts the right integer to its canonical base-10
  decimal spelling and concatenates it;
- signed and unsigned JSON integers are supported within the existing JSON and
  expression resource bounds;
- decimal numbers, booleans, null, arrays, objects, and missing values are not
  converted;
- `number + string` is rejected; conversion is deliberately right-operand
  only, so this is not JavaScript-style bidirectional coercion.

The conversion is pure, deterministic, locale-independent, bounded, and has no
clock, random, environment, method, host, or mutation access. The v1alpha2
contract records the rule in its namespaced expression-profile extension and
bumps the contract identity API version and release filename. Existing
unsupported-JavaScript diagnostics remain unchanged.

## Consequences

The exact frozen Eco label can run over the existing numeric Generate Items
`index` without changing Ticket 07's item schema, provenance, stream digest, or
checkpoint format. The contract digest and editor catalog lock change because
this is a deliberate contract revision. Release bundles carry both immutable
Edit Fields contract revisions, while only v1alpha2 is offered for new editor
nodes.

The VM must keep focused tests for canonical signed/unsigned integer spelling,
rejection of decimals and reversed operands, and immutable input behavior.
Future coercions or conversion functions require another decision record.
