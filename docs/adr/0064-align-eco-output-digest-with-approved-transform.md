---
status: accepted
owner_decision: 2026-09-14
---
# Align the Eco Output Digest with the approved Edit Fields transform

## Context

The Ticket 11 Eco vector was initially calculated from an item containing the
`parity` and `label` fields only. Ticket 08-A, however, explicitly freezes the
native Edit Fields transformation to retain `{index,value,data}` and add four
fields: `eco`, `parity`, `doubled`, and `label`. Ticket 11 defines the Output
Digest over the canonical transformed logical item, so omitting `eco` and
`doubled` would make the digest and byte count describe a different item than
the one executed, persisted, and linked by Causal Trace.

The discrepancy was identified during the 100K API acceptance run. The
full approved transform deterministically produces 5,135,908 transformed
logical bytes and the digest below; the earlier 3,791,517-byte vector and
`sha256:1caa...ae9fdd` described the incomplete two-field shape.

## Decision

Keep the full Ticket 08-A transform as the authoritative Eco input to Merge
and Summarize. The frozen Ticket 11 vector is revised to:

- generated and merged items: `49,998`;
- true and false branch counts: `24,999` each;
- transformed logical bytes: `5,135,908`;
- Output Digest algorithm: `sha256-jcs`;
- Output Digest:
  `sha256:56193dff07ac42baab1774fc9f25ce52dd527a5c14a59ac083b372b0ff402e55`;
- logical Activation count: `100,000`.

Summarize remains a generic bounded reducer. It consumes the complete
canonical transformed logical item from the Merge stream, retains only its
counters, ordinal-integrity bitmap, provenance links, and digest chain, and
does not add an Eco-specific projection or silently discard fields.

## Consequences

The independent Rust reducer vector, operation documentation, and API/browser
acceptance must assert the full four-field shape and revised digest. The
six-node topology, merge order, bounded-memory behavior, typed summary,
provenance, Causal Trace, rollback semantics, and 100,000-Activation equation
do not change. This decision avoids a hidden mismatch between execution output
and summary evidence and keeps replay and durable inspection semantically
aligned.
