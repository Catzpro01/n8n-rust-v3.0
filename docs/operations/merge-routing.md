# Deterministic Merge routing

**Status:** native runtime implementation and the bounded public acceptance lane
are pinned-verified for Ticket 10; broader compatibility remains out of scope.

The approved `canopy.native/merge@v1alpha1` slice is a pure, deterministic
Barrier/Reducer Activation after the bounded If path:

```text
manual-trigger -> generate-items -> [edit-fields] -> if -> merge
                                                        true  ─┐
                                                        false ─┴─> items
```

The bracketed Edit Fields stage is optional. Merge is not a general-purpose
workflow compatibility claim: the supported native topology has exactly one If
producer, two named inputs (`true` and `false`), and one `items` output.

## Contract and ordering

The locked contract is `contracts/merge.v1alpha1.json`:

- ABI: `canopy.merge/v1alpha1`;
- Activation Shape: `barrier_reducer`;
- effects: Pure, deterministic, idempotent, and retry-safe;
- required input ports: `true` and `false`;
- output port: `items`; and
- first supported mode: `true_then_false`.

The executor accepts and closes both If streams before it invokes the reducer.
It appends the complete true stream and then the complete false stream. The
reducer validates input-port phase, bounds ordinals with a fixed 50,000-bit
bitmap, rejects duplicate provenance, and emits each accepted record once. It
retains no item payload in the reducer itself; its output callback writes to a
separate bounded Artifact-backed spool.

The compiler rejects a missing branch input, a missing If-to-Merge connection,
unsupported Merge configuration, changed node identity, extra nodes/edges in
the native topology, incompatible declared source/target port schemas, and
invalid contract capabilities/effects. Each input record carries the typed
`canopy.if-route-provenance/v1alpha1` contract; the reducer rejects a missing,
malformed, or cross-port route rather than silently coercing an untyped JSON
field. The compiler and reducer preserve typed `canopy.merge.*` diagnostics.

## Artifact-backed streams and ownership

Each true, false, and output stream is written as newline-delimited canonical
JSON records through a bounded `ArtifactService` upload lease. A segment is
finalized only after its upload is complete and its reference is committed.
Merge reads one segment at a time through `ArtifactContentStream`; neither
branch nor output requires materializing the full stream in RAM.

Segment reference IDs are scoped to the run, Merge node, port, and segment
ordinal. Successful terminal progress retains the true, false, and output
references in `generation.merge`, so the Owner can verify the physical evidence
through the existing Artifact APIs. If cancellation, a spool/read error, or a
reduction error occurs, active leases are abandoned and already-finalized
runtime-only references are released and moved to the Artifact quarantine safety
window. Shared deduplicated objects are not quarantined while another Owner
reference protects them.

For an incomplete daemon restart, the executor replays the bounded source from
ordinal zero instead of treating a partially finalized branch as a complete
Merge input. This is intentional: branch-close state and the final Merge
checkpoint are one barrier, and an unpersisted suffix must not become a silent
loss. Startup Artifact reconciliation handles abandoned staging and unreferenced
objects before new Run admission.

## Durable progress and trace

`run_generation_progress.merge_json` stores the serialized `MergeProgress` view
with the node identity, mode, branch counts, output count, logical bytes, merged
stream digest, physical spool bytes, and all segment references. The terminal
checkpoint snapshot repeats the reducer evidence, while Causal Trace separates:

- the logical Merge activation and its input/output port semantics; from
- physical segment counts, Artifact references, and spool byte metrics.

A successful run requires the reducer output count to equal the generated count
and the two branch counts to sum to that output. The terminal correctness digest
includes Merge as the final logical outcome, after Generate Items, optional Edit
Fields, and If. Parent provenance is carried in every canonical Merge record;
the reducer never publishes a payload-less output.

## Bounded failure and cancellation behavior

Cancellation is observed between generation micro-batches. A cancellation
before the barrier prevents reduction, records a typed cancelled Merge outcome,
and cleans runtime-owned finalized segments. A branch evaluation failure prevents
reduction and records the typed upstream failure in the terminal trace. A Merge
configuration, duplicate-ordinal, provenance, Artifact read, or Artifact write
failure is a typed permanent failure and never publishes a successful output.

Empty true or false branches are valid: the reducer waits for both closed
streams and emits the non-empty side in the declared order. The same generated
items and route decisions therefore produce the same Merge logical digest even
when physical branch segment completion timing differs.

## Acceptance and release evidence

The public seam test `tests/acceptance/test_if_runtime.py` publishes the
five-node editor topology, executes twelve transformed items, verifies six true
and six false records, checks one-to-one Merge activation provenance and
Artifact-backed segment evidence, reads the terminal result after a daemon
restart, and verifies the persisted reducer digest and output count. Focused Rust
coverage remains in `crates/workflowd/src/merge.rs`, `artifact.rs`, and `run.rs`.

The bounded Ticket 10 code/test/documentation gate is recorded as follows on
2026-09-14 (Asia/Novosibirsk; the hosted run timestamps are UTC):

- `34779941760` — temporary pinned Rust diagnostic: compilation, 4 node-contract
  tests, 50 workflowd tests, doc tests, and the three public Merge acceptance
  cases passed; the diagnostic's final captured acceptance log is `OK`.
- `34780397609` — final public If/Merge acceptance passed on the evidence-clean
  branch tip, including empty-branch and restart assertions.
- `34780397639` — final repository validation passed after the retry-reference
  cleanup change, including editor build/typecheck, `cargo fmt --check`, all
  workspace tests, and dependency-free Python tests.
- `34779941770` — an earlier independent pinned rustfmt check passed; the
  temporary helper was removed after the final validation lane passed.

The temporary diagnostic and format-helper workflows and their scratch logs are
removed after this evidence capture. Release-bundle packaging and the broader
production upgrade/recovery gate remain later phase evidence; this document only
claims the bounded native Merge slice.
