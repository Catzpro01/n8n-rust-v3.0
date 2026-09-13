# AI Agent Turn Contract

**Contract:** `canopy.agent-turn/v1alpha1`
**Status:** accepted by ADR 0061

This is the normalized boundary between the AI Agent Node and an Agent Engine
adapter. It is a semantic contract for the first vertical slice; the concrete
Rust trait and serialized transport may be introduced behind this versioned
shape.

## Request

A `TurnRequest` is bound to the immutable execution identities:

- `run_id`, `activation_id`, `node_instance_id`, `plan_digest`;
- `blueprint_digest`, `turn_id`, and `attempt`;
- bounded input/Artifact references;
- effective Agent Policy and Model Route entry;
- explicit Capability Grant and Secret Lease handles;
- deadline, cancellation token, and remaining hard budget.

The request contains references and bounded excerpts only. It has no ambient
credential, unrestricted workflow state, or implicit memory snapshot.

## Event categories

An adapter emits an ordered stream of normalized events. Each event is tied to
the request identities and has a monotonic sequence, adapter identity, and
redacted trace metadata.

| Category | Meaning | Durable requirement |
| --- | --- | --- |
| `started` | Adapter accepted the turn | durable Activation event |
| `memory` | Query, bounded references, or policy decision | query and provenance refs |
| `skill` | Locked Skill Set selected or denied | lock/capability evidence |
| `progress` | Text/tool-argument/model progress | non-authoritative; bounded if retained |
| `tool` | Requested, granted, started, completed, or denied tool work | checkpoint before/after side effect |
| `checkpoint` | Safe restart/reconciliation point | durable cursor/ref and budget |
| `suspended` | Waiting for approval, event, remote task, or budget decision | signed resume binding |
| `candidate` | Proposed output or repair candidate | not downstream-visible |
| `validation` | Output Contract validation or bounded repair result | validator and schema evidence |
| `terminal` | `Succeeded`, `Failed`, `Uncertain`, or `Cancelled` | immutable terminal outcome |

Provider-specific event names, private reasoning, and raw credentials do not
cross this boundary. A remote task ID or provider cursor is opaque adapter
state referenced by the Activation; it cannot replace `activation_id`.

## Side-effect and retry rule

Before a tool or other external side effect, the engine must have an explicit
Capability Grant and an idempotency key where the target supports one. The
platform checkpoints the intended operation. Afterward it checkpoints the
observed result or records `Uncertain` if the result cannot be established.
Only a classified failure before this boundary can activate the next locked
Model Route entry.

## Terminal result

A terminal result includes one of:

- validated Output Contract references for `Succeeded`;
- a typed suspension reason and expiring resume binding for `Suspended`;
- a classified error and retryability for `Failed`;
- reconciliation key and unknown-operation evidence for `Uncertain`;
- cancellation request/confirmation evidence for `Cancelled`.

Budget use, cost, provider/model identity, lane, capability decisions, and
Artifact references are included in the redacted Causal Trace. No result is
published to downstream workflow nodes until Output Contract validation passes.

## Replay

A replay may start at a selected Activation from recorded references or supplied
fixtures. External writes are mocked by default. A live replay requires
approval, new idempotency/audit context, and a separate trace; it cannot mutate
the original Run's outcome.
