---
status: active
---
# Eco 100K Summary Fixture

This document freezes the independent fixture used by Ticket 11. It is a
contract test vector, not a browser payload fixture: the Run API returns only
bounded progress and summary data, while merged item records remain in the
Artifact-backed spool.

## Six-node topology

The published graph is exactly:

```text
Manual Trigger(invocation)
  -> Generate Items(input)
  -> Edit Fields(input)
  -> If(input)
     -> true  -> Merge(true)
     -> false -> Merge(false)
  Merge(items) -> Summarize(items)
```

There is one node instance for each named node. Merge order is the declared
`true_then_false` order, and Summarize has one `summary` output.

## Frozen input and transform

Generate Items receives one Manual Trigger invocation and emits the checked
integer range:

- `count = 49,998`;
- `start = 0`;
- `step = 1`;
- `data = null`;
- generated ordinals and values are `0..49,997`.

Edit Fields uses `mode = merge` and reads the immutable generated item. It
adds exactly:

```json
{"eco":true,
 "parity":"even" when value % 2 === 0, otherwise "odd",
 "doubled":value * 2,
 "label":"eco-" + index}
```

The label conversion is the bounded, right-operand JSON-integer conversion in
`docs/adr/0058-edit-fields-v1alpha2-bounded-integer-label-conversion.md`.
No other field, time source, random source, or host capability participates.

If routes even values to `true` and odd values to `false`. Thus both routes
are closed before Merge reduces them.

## Canonicalization and digest

The logical item is canonicalized with RFC 8785 JSON Canonicalization Scheme
(`jcs-rfc8785`). Summarize retains counters and a 50,000-bit ordinal set; it
never retains the merged payload collection. For each merged record it advances
this chain, using the canonical JSON bytes of the object:

```json
{
  "algorithm":"sha256-jcs",
  "input_port":"true" | "false",
  "item":<canonical transformed logical item>,
  "ordinal":<generated ordinal>,
  "previous":<previous digest or "genesis">,
  "schema":"canopy.output-digest/v1alpha1"
}
```

The digest operation is `sha256` over those JCS bytes and is serialized with
the `sha256:` tag. The frozen expected Output Digest is:

```text
sha256:56193dff07ac42baab1774fc9f25ce52dd527a5c14a59ac083b372b0ff402e55
```

The expected counters are `true = 24,999`, `false = 24,999`, and
`total = 49,998`. The expected transformed logical-byte sum is `5,135,908`
for the frozen JSON item shape, including `eco`, `parity`, `doubled`, and
`label`.

## Activation accounting

The exact logical Activation equation is:

```text
1 Manual Trigger
+ 1 Generate Items
+ 49,998 Edit Fields
+ 49,998 If
+ 1 Merge
+ 1 Summarize
= 100,000 Activations
```

Node-level Causal Trace records retain the bounded aggregate and causal range
facts needed to locate item work without returning the merged collection to the
browser. The summary record links back to the Merge output digest, branch
counts, ordinal range, and the retained per-item provenance in the spool.

Repeated runs must preserve these counts, ordering, canonicalization facts,
and Output Digest regardless of safe scheduler interleavings.
