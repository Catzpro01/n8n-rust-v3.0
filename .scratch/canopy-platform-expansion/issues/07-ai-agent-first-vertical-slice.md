# 07: Implement the first AI Agent vertical slice

Type: implementation
Status: planned
Phase: AI Agent platform
Decision prerequisite: ADR 0061 / issue #11
Recovery prerequisite: issue #12 decision before recovery/upgrade integration
Release prerequisite: issue #13 integrated expansion gate

## Goal

Deliver the first production AI Agent Node slice on the accepted durable,
capability-bound Agent Turn contract without adding a resident model, MCP, A2A,
CLI, browser, GPU, or other heavy runtime.

## Scope

1. Add versioned Rust domain/adapter seams for Agent Blueprint locks, Model
   Route entries, six typed subports, `TurnRequest`, normalized events,
   checkpoints, Capability Grants, Secret Leases, hard budgets, Output
   Contract validation, and typed terminal outcomes.
2. Connect one deterministic built-in/fake Agent Engine to the existing Run,
   Activation, Artifact, and Causal Trace seams. The fixture must exercise a
   memory reference, a safe tool call, bounded output validation/repair, a
   pre-side-effect fallback, suspension/resume, cancellation, and uncertain
   external outcome without using real credentials or a hosted provider.
3. Add the first browser editor surface for composing and inspecting the
   six-subport Blueprint locks and effective scope; Draft-only edits must not
   change a Published Revision.
4. Add persistence and replay evidence for bounded events, redacted traces,
   immutable Blueprint/Plan identities, idempotency/reconciliation keys, and
   external writes mocked by default.
5. Add unit, contract, integration, recovery, security, budget, and
   deterministic replay tests; document the fixture and produce release-gate
   evidence.

## Invariants

- A connected engine, Memory Layer, Skill Set, MCP server, or tool never gets
  implicit credentials or authority.
- Only classified failures before an external side effect may activate an
  ordered fallback; unknown outcomes become `Uncertain`.
- No candidate output reaches a downstream node before Output Contract
  validation.
- Budget exhaustion, cancellation, deadline, and suspension are durable typed
  outcomes and release worker capacity.
- The default Rust installation remains light and starts no optional runtime.

## Out of scope

Provider catalog breadth, real third-party agent adapters, arbitrary n8n
parity, multi-agent councils, private chain-of-thought persistence, broad
Skill/MCP package discovery, and the upgrade/migration implementation itself.
Those require their own contracts, evidence, and later phase work.
