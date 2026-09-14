---
status: accepted
owner_decision: 2026-09-14
---
# Make AI Agent turns durable and capability-bound

## Context

The recorded AI platform design gives an AI Agent six typed subports:
Agent Engine, Memory, Skill, MCP/Tool, Policy, and Output. Existing decisions
already require immutable plans, explicit Capability Grants and Secret Leases,
query-first Memory Stacks, hard budgets, side-effect-free output validation,
causal traces, and durable suspensions. Issue #11 must turn those principles
into one production boundary that works for a Rust engine, a model provider,
an A2A remote agent, or an isolated CLI/process worker without making any
heavy runtime a resident dependency.

The Owner selected the following options through HITL on 2026-09-14:

- durable Activation for the turn/session lifecycle;
- one locked primary engine with ordered fallback only for classified
  pre-side-effect failures;
- independent typed subports with explicit grants and leases;
- a validated Output Contract with hard budgets and typed outcomes.

## Decision

### 1. AI Agent Node and Agent Blueprint

An AI Agent Node has exactly these typed subports:

1. **Engine** — one primary Agent Engine plus an explicit ordered fallback
   policy;
2. **Memory** — zero or more ordered Memory Layers;
3. **Skill** — zero or more version-locked Skill Sets;
4. **MCP/Tool** — zero or more capability-governed tools;
5. **Policy** — exactly one Agent Policy, with a secure default when omitted;
6. **Output** — exactly one side-effect-free Output Contract.

A published Agent Blueprint freezes the identities and policies for all six
ports: engine adapter and worker identity, Model Route, Memory Stack, Skill
locks, MCP/tool contracts, Agent Policy, and Output Contract. It also freezes
the effective scope resolution and capability manifest. A Run references the
blueprint digest rather than a mutable editor object.

Global, Project, Workflow, and Node selections are visible in the Blueprint.
Effective node configuration is more local than workflow, project, or global
configuration, but a local override is valid only when it is represented by an
explicit lock/configuration change. Skill Package scope remains the separate
Global/Project/Workflow package lifecycle defined by ADR 0060; a Node-level
selection is a Blueprint edge, not an unreviewed package scope.

Mutable Drafts may edit a Blueprint composition. A Published Revision may only
reference immutable blueprint and dependency locks. Existing Runs, Recovery
Sets, and Artifacts keep the exact digest they started with.

### 2. Durable turn protocol

Each turn is a bounded durable Activation identified by the Run, node
instance, frozen plan, Blueprint digest, turn, and attempt identities. The
normalized request contains bounded input references, the effective Policy,
selected grants/leases, a deadline, and the remaining budget. It does not
contain ambient credentials or an unbounded prompt assembled from all memory.

The Agent Engine adapter consumes that request and emits normalized typed
lifecycle events. At minimum, the protocol represents:

- turn accepted/started and adapter identity;
- bounded Memory query/result references and Skill/tool selection;
- model or engine deltas as non-authoritative progress;
- tool request, grant decision, tool result, and Artifact references;
- checkpoint, budget, deadline, cancellation, and suspension events;
- candidate output, validation/repair attempts, and terminal outcome.

The daemon checkpoints before any external side effect and after its outcome.
A provider cursor or remote task identifier is opaque adapter state bound to the
Activation; it is not a replacement for the platform Activation identity.
Resume, cancellation, and timeout are explicit protocol operations. A worker
may be restarted or replaced only when the adapter contract says the cursor
and side-effect idempotency are safe to resume.

### 3. Engine adapters, lanes, and fallback

The common Agent Engine boundary admits built-in Rust execution, model-provider
adapters, local inference, A2A agents, and isolated CLI/process agents. MCP is
primarily the MCP/Tool subport; an MCP server is not treated as an agent
runtime merely because it exposes tools. An adapter may wrap an MCP-backed
agent only when it declares an Agent Engine contract and lifecycle.

The cheapest valid Execution Lane is selected by the plan and adapter
capabilities. Heavy Orchestrator and non-Rust runtimes are remote-first or
local-opt-in and absent/stopped in the default Rust installation. Every worker
is pinned by executable/image/version, capability manifest, resource budget,
trust evidence, and conformance fixture.

A Model Route locks one primary and an ordered fallback list. Each entry pins
provider/model or external engine identity, protocol version, capabilities,
data policy, determinism/seed behavior when supported, cost/quota rules, and
lane. Fallback is automatic only for a classified failure proven to occur
before a side effect. A timeout or lost connection after an external request,
an unknown remote outcome, a policy denial, or an output-side-effect boundary
never silently falls through; it becomes the corresponding typed outcome or
requires policy-approved reconciliation.

### 4. Memory, Skill, and MCP/Tool authority

Memory is query-first. A Memory Layer returns ranked references and bounded
excerpts under its scope, retrieval policy, sensitivity rules, and token
budget. Full content remains in Artifacts until selected. Memory writes retain
Run/Activation/tool/source provenance, pass redaction, are deduplicated, and
may be asynchronous only when the policy marks them safe.

Skill Sets are immutable locked inputs. The effective skill selection is
visible and follows the accepted scope/override rules. A skill's instructions
or resources do not grant authority by themselves.

Every skill, MCP server, model/provider credential, and tool call requires an
explicit Capability Grant. Selected credential fields arrive only through a
short-lived Secret Lease bound to the Activation. HTTP MCP authorization and
local STDIO credential handling remain separate. Grants declare operation,
resource, scope, expiry, and budget; a connected resource is never trusted by
mere attachment.

### 5. Budgets and terminal outcomes

Agent Policy enforces hard limits for wall time, tokens, money, model/tool
calls, recursion, memory retrieval, and output size. Usage and remaining
budget are traceable. Exhaustion stops the turn; it cannot be bypassed by an
adapter or hidden retry. A policy may permit a new approved budget or cheaper
route, but that change creates a visible new attempt/lock decision.

The public Agent Node outcome is one of:

- **Succeeded** — the Output Contract is valid and its references are durable;
- **Suspended** — a signed, expiring, one-use resume path is bound to the exact
  Run and Activation, such as approval, external event, or approved budget
  continuation;
- **Failed** — a classified terminal or retryable failure with no unresolved
  external side effect;
- **Uncertain** — an external outcome cannot be established; automatic retry or
  fallback is forbidden until reconciliation;
- **Cancelled** — cancellation is recorded, including whether the adapter
  confirmed it.

A `Suspended` turn consumes no worker slot. A `Failed` or `Uncertain` result
never masquerades as a valid Output Contract.

### 6. Output, trace, and replay

The Output Contract validates text, JSON Schema, multipart stream, Artifact,
or typed error before the result can enter the main workflow. Validation is
side-effect-free. Agent Policy may allow bounded repair using the remaining
budget; repair cannot silently deliver downstream side effects. Sending an
email, changing a repository, or writing an external database remains a
separate downstream node.

Every turn contributes a redacted Causal Trace containing event order,
Blueprint/plan/adapter identities, grants and Secret Lease identifiers,
policy decisions, budget usage, external call references, validation, and
error chains. Private model reasoning, credentials, and unrestricted prompts
are not required or persisted.

Replay begins at a selected Activation with recorded references or supplied
fixtures. External writes are mocked by default. An approved live replay uses
new idempotency and audit context and is never represented as the original
attempt. Provider token deltas are progress evidence, not a portable replay
contract; adapter-specific event names stay behind the normalized boundary.

## Consequences

- The first Agent Node slice can share the existing durable Run, Activation,
  Artifact, Capability Grant, Secret Lease, and Causal Trace seams.
- Built-in and external engines are interchangeable at the contract boundary,
  but their lifecycle, capabilities, lane, cost, and evidence remain visible.
- Unknown external outcomes are safer but require reconciliation UX and
  recovery work in the later upgrade/recovery phase.
- The default installation stays Rust-first and light; model, MCP, A2A, CLI,
  browser, and local/GPU runtimes remain optional workers.
- The next implementation must deliver the contract, a deterministic fake
  engine/fixture, one safe vertical slice, tests, documentation, and release
  evidence before adding broad provider or editor coverage.

## Not permitted

This decision does not authorize arbitrary n8n parity, silent provider
credential access, unreviewed third-party source import, persistence of private
chain-of-thought, mutable package/branch execution, implicit tool authority,
or a multi-agent council hidden inside one Node. Councils and ensembles remain
explicit strategies with their own Blueprint and budget.
