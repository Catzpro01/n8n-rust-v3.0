# AI Agent Node

**Contract:** `canopy.ai-agent-node/v1alpha1`
**Decision:** ADR 0061

An AI Agent Node executes one bounded agent turn under a durable Run
Activation. It is not an unbounded chat loop and it does not grant authority
because a model, skill, MCP server, or external engine is connected.

## Typed subports

- **Engine:** exactly one primary Agent Engine and an explicit ordered fallback
  policy. The adapter may be built-in Rust, a model provider, local inference,
  an A2A agent, or an isolated CLI/process runtime.
- **Memory:** zero or more ordered Memory Layers. Reads are query-first,
  bounded, scoped, ranked, and reference-based; writes are provenance-bearing,
  redacted, and policy-controlled.
- **Skill:** zero or more immutable, version-locked Skill Sets. Scope and
  shadowing are visible in the Blueprint and do not grant capabilities.
- **MCP/Tool:** zero or more tools governed by explicit Capability Grants and
  short-lived Secret Leases. MCP HTTP authorization and local STDIO credential
  handling are distinct.
- **Policy:** exactly one Agent Policy, with a secure default supplied when
  omitted. It sets hard budgets, permitted routes, tools, memory, recursion,
  and approval behavior.
- **Output:** exactly one side-effect-free Output Contract for text, JSON
  Schema, multipart stream, Artifact, or typed error.

## Agent Blueprint and locks

A Published Revision references a content-addressed Agent Blueprint containing:

- engine adapter/worker identity and capabilities;
- Model Route primary and ordered fallback identities;
- Memory Stack and layer policies;
- Skill Set and MCP/tool contract locks;
- Agent Policy and Output Contract;
- effective Global, Project, Workflow, and Node selections;
- lane, resource, cost, trust, and compatibility evidence.

Draft edits are not run as production. Published blueprints are immutable;
Runs, Recovery Sets, and Artifacts retain their exact Blueprint digest. A more
local Node or Workflow selection can shadow a Project or Global selection only
through an explicit visible lock/configuration change.

## Turn boundary

The normalized turn request is bound to `Run`, `Activation`, node instance,
plan, Blueprint digest, turn, and attempt identities. It carries bounded input
references, effective policy, selected grants/leases, deadline, and remaining
budget. It carries no ambient credentials or unbounded memory dump.

An adapter emits normalized lifecycle events for start, memory/skill/tool
selection, model/engine progress, tool request and result, checkpoints,
budget/deadline/cancellation, suspension, candidate output, validation, and
terminal outcome. Provider-specific event names and opaque remote cursors stay
inside the adapter. The daemon checkpoints before and after external side
effects.

## Fallback rule

A Model Route may select a different locked entry only after a classified
pre-side-effect failure. Timeouts or disconnects after an external request,
unknown remote outcomes, policy denials, and other uncertain states do not
fall through automatically. They surface as a typed outcome or require
reconciliation.

## Safety invariants

1. No engine, memory layer, skill, or tool receives a credential merely because
   it is connected. Capability Grants and Secret Leases are explicit.
2. MCP is a tool/resource lane by default, not an implicit Agent Engine.
3. Hard budgets cover tokens, money, wall time, tool calls, recursion, memory
   retrieval, and output size. Exhaustion stops the turn.
4. Output validation and bounded repair have no external side effect. External
   writes remain separate downstream nodes.
5. Every turn contributes a redacted Causal Trace; private model reasoning and
   credentials are not required or persisted.
6. Replay uses references or fixtures and mocks external writes by default.
   Approved live replay receives new idempotency and audit context.
7. Heavy Orchestrator and non-Rust engines are remote-first/local-opt-in and
   are not resident dependencies of the default Rust installation.

## Outcomes

- `Succeeded`: a valid Output Contract is available through durable references.
- `Suspended`: the turn can resume through a signed, expiring, one-use path
  bound to its Run and Activation.
- `Failed`: a classified terminal or retryable failure with no unresolved
  external side effect.
- `Uncertain`: an external outcome is not established; retry/fallback is
  blocked until reconciliation.
- `Cancelled`: cancellation is recorded, including adapter confirmation.
