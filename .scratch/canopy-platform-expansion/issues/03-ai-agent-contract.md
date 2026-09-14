# 03: Decide the AI Agent Node and Agent Engine contract

Type: wayfinder-decision
Status: resolved
Blocked by: None
GitHub issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/11
Resolved: 2026-09-14
ADR: `docs/adr/0061-make-agent-turns-durable-and-capability-bound.md`
Specs: `docs/spec/ai/agent-node.md`, `docs/spec/ai/agent-turn.md`

## Question

What is the production contract for the AI Agent Node and its six typed subports: Agent Engine, Memory, Skill, MCP/Tool, Policy, and Output?

Settle the turn/session protocol, built-in and external adapter lifecycle, Model Route and ordered fallback, Memory Stack reads/writes/provenance, Skill/MCP capability grants, hard budgets, Output Contract validation and bounded repair, typed suspension/failure/uncertain outcomes, redacted trace, deterministic/replay boundaries, and remote-first versus local-opt-in execution. Include Agent Blueprint scope and lock semantics without making the default Rust installation depend on heavy runtimes.

## Resolution

The Owner selected the following HITL baseline:

- Durable Activation owns each bounded turn/session and supports explicit
  checkpoint, resume, suspension, cancellation, and recovery.
- A Model Route has one locked primary and an ordered fallback list; automatic
  fallback is permitted only for a classified failure before an external side
  effect. Unknown outcomes do not silently retry or switch engines.
- Engine, Memory, Skill, MCP/Tool, Policy, and Output remain independent typed
  subports. Capabilities and credentials require explicit Capability Grants
  and Secret Leases; memory remains query-first and bounded.
- A side-effect-free Output Contract is validated before downstream delivery;
  hard budgets and bounded policy-approved repair apply; terminal outcomes are
  typed `Succeeded`, `Suspended`, `Failed`, `Uncertain`, or `Cancelled`.
- Published Agent Blueprints lock engine/route/memory/skill/tool/policy/output
  identities and effective scope. Heavy and non-Rust engines remain
  remote-first/local-opt-in and absent from the default Rust installation.

ADR 0061 and the two AI specs record the accepted contract. No production
runtime was added in this decision step; the first implementation must be a
vertical contract/fixture slice with production code, tests, documentation,
and release evidence, and must respect the approved phase order.
