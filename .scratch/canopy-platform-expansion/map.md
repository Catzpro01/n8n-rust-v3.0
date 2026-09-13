# Wayfinder Map: Canopy Workbench full platform expansion

Type: decision-map
Status: ready-for-agent
Parent issue: https://github.com/Catzpro01/n8n-rust-v3.0/issues/8

## Destination

Implement every explicitly recorded product design in the current workspace as a staged, production-ready Canopy Workbench platform: complete the core Eco foundation, then add the Node Form/Execution Lane extension seams, Workflow Hub, Skill Hub, AI Agent/Agent Engine stack, upgrade/recovery path, large-editor proof, and compatibility surfaces. Each delivered slice must include its user-visible behavior, versioned contract, tests, documentation, and release evidence. This effort does not promise arbitrary full n8n parity or unrecorded features.

## Notes

- Preserve the recovered Rust-first + connected Preact baseline; do not restart the implementation.
- Existing accepted ADRs, `CONTEXT.md`, clean-room policy, and current Ticket 08 contract remain authoritative. Supersede them only with an explicit new ADR and Owner confirmation.
- Approved phase order: core completion; extension foundation; Workflow/Skill Hub; AI Agent platform; production upgrade/recovery; large editor and compatibility.
- Default production remains one lightweight Rust daemon; optional heavy/non-Rust agents are remote-first/local-opt-in and GPU is Off by default.
- Use `/grilling` plus `/domain-modeling` for HITL decisions, primary-source research for facts, and prototypes only for questions that need runnable evidence.
- Final acceptance means production code, versioned API/client seams, UI where user-visible, tests, documentation, and release/recovery evidence; no placeholder-only completion.

## Decisions so far

- Owner selected all explicitly recorded workspace designs, not unbounded full n8n parity.
- Owner approved the phase order above.
- Owner requires each phase to be implemented and verified end to end.
- Existing first-runnable map remains the foundation; its deferred AI/Hub scope is expanded here rather than silently rewritten.
- Existing accepted decisions include the Rust deterministic engine, immutable revisions/pinned plans, bounded Envelopes/Artifacts, clean-room compatibility, Agent Engine subports, package trust, and signed release slots.

## Current frontier

- [Decide the Node Form and Execution Lane extension contract](https://github.com/Catzpro01/n8n-rust-v3.0/issues/9) — unblocked; first decision.
- [Research external agent and package adapter boundaries](https://github.com/Catzpro01/n8n-rust-v3.0/issues/14) — unblocked; research may run in parallel.

## Later tickets

- [Decide the Workflow and Skill Package lifecycle](https://github.com/Catzpro01/n8n-rust-v3.0/issues/10) — blocked by #9.
- [Decide the AI Agent Node and Agent Engine contract](https://github.com/Catzpro01/n8n-rust-v3.0/issues/11) — blocked by #9.
- [Decide upgrade and recovery for Hub and Agent state](https://github.com/Catzpro01/n8n-rust-v3.0/issues/12) — blocked by #10 and #11.
- [Define the integrated expansion acceptance and release gate](https://github.com/Catzpro01/n8n-rust-v3.0/issues/13) — blocked by #10, #11, and #12.

## Core implementation dependency

The existing first-runnable implementation map remains the implementation dependency for the approved phase order:

- `.scratch/eco-100k-first-runnable/map.md`
- Tickets 09–21 complete the core Eco/large-editor/compatibility/release path after Ticket 08.

The expansion decision map must not claim those implementation tickets are complete; later expansion implementation begins only after its decision tickets and the required core blockers are resolved.

## Not yet specified

- Exact integration contract from current Native Nodes/Execution Plans to Node Forms, Execution Lanes, Agent Blueprints, and optional workers.
- Concrete Workflow Package and Skill Package lifecycle through catalog, verification, sandbox, Draft import, publication, update, rollback, and uninstall.
- AI Agent turn protocol, adapter boundary, Model Route, Memory Stack persistence/retrieval, Skill/MCP grants, Output Contract, trace, budgets, suspension, and recovery.
- Trust/capability policy for third-party packages, skills, MCP tools, agent engines, and local versus remote-first execution.
- Upgrade/migration contract for active Runs, pinned plans, Hub locks, Agent Blueprint/Model Route locks, Artifact/vault metadata, and Current/Previous slots.
- Integrated acceptance fixture and release gate for all extension slices.

## Out of scope

- Full arbitrary n8n node-catalog parity or official n8n compatibility.
- Copying n8n source, Enterprise code, tests, protected assets, branding, product copy, icons, or distinctive trade dress.
- Making Node.js, Python, browser, local-model, GPU, or external agent runtimes resident dependencies of the default installation.
- Building unrecorded features merely because another product has them.
- Silently replacing the current Rust/Preact implementation or changing the fixed Arena branch.
