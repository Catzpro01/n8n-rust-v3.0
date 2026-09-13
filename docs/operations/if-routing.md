# Deterministic If routing

**Status:** native runtime and public/editor acceptance complete for Ticket 09's bounded
If slice; broader compatibility remains out of scope.

The approved `canopy.native/if` v1alpha1 runtime is a pure, deterministic branch
stage for the bounded native generation path:

```text
manual-trigger -> generate-items -> [edit-fields] -> if
```

The bracketed Edit Fields stage is optional. This topology is deliberately
bounded; it is not a claim of unbounded workflow compatibility.

## Route semantics

The executor evaluates each `GeneratedEnvelope.logical_item` exactly once. Edit
Fields writes its transformed logical value back to that field before If runs,
while the physical `item` may still carry its encrypted Artifact reference.
The If evaluator produces exactly one stable output port, `true` or `false`, for
each item. Routing is staged: if a later item produces a diagnostic, no partial
route provenance is published for the batch.

Each routed envelope carries compact provenance rather than a retained payload
copy:

- `if_node_instance_id`;
- `if_input_digest`;
- `if_output_port` (`true` or `false`); and
- `if_condition_results`.

The ordered route chain is committed as `canopy.if-route-chain/v1alpha1`. Its
chained digest includes the prior route digest, input ordinal, selected output
port, condition results, and canonical logical item. This gives repeated runs
and restart suffixes a deterministic evidence boundary without retaining every
item in browser memory.

## Durable progress and trace

`run_generation_progress` stores the pinned If node identity, true/false counts,
and route-chain digest. A generation checkpoint is accepted only when the two
branch counts cover the generated cursor, counts do not regress, and the branch
identity is consistent with the pinned plan. The public `generation.branch`
view exposes the same compact facts.

A successful terminal run records the If activation after Generate Items and
optional Edit Fields:

- logical order 3 for `generate-items -> if`;
- logical order 4 for `generate-items -> edit-fields -> if`;
- named output ports `true` and `false`;
- true/false counts and route-chain digest;
- `exactly_one_output_per_item: true`; and
- `item_linking: one_to_one`.

The terminal correctness digest includes the branch outcome. Causal Trace stores
branch counts, output-port names, route-chain evidence, input/output digests,
and safe diagnostics, not the full item stream.

## Cancellation and pressure

Cancellation is observed between micro-batches. A batch is evaluated and
published as one bounded unit before the executor proceeds, so cooperative
cancellation cannot split a routed batch. The existing bounded envelope queue
and byte budget remain in force; blocked sends contribute backpressure metrics to
speculative progress and durable checkpoints. A terminal completion persists
any final batch facts even when the final batch did not cross a checkpoint
threshold.

Unsupported expressions, missing/null/type failures, invalid configuration, and
route-chain integrity failures become structured `canopy.if.*` diagnostics and
cannot publish a successful run.

## Evidence boundary

Focused Rust coverage exercises AND/OR and typed diagnostics in the If module,
atomic batch provenance, replay-stable route digests, transformed logical input,
cooperative cancellation, bounded queue permit release, and fault-injected batch
atomicity. The public seam test `tests/acceptance/test_if_runtime.py` publishes
the four-node editor topology and verifies both `all` and composed `any` logic,
terminal branch progress, 1,024-item envelope backpressure with exact counts,
pre-activation cancellation without an If activation, a follow-on successful
Run, restart persistence, and Causal Trace facts. The hardened acceptance lane
passed in pinned workflow `34782134925`; repository validation passed in
`34782134893`, covering formatting, workspace tests, editor build, and
dependency-free repository tests. The bounded Ticket 09 If slice is released as
verified evidence; downstream compatibility beyond this topology remains a later
phase.
