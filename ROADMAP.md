# Canopy Workbench roadmap

Canopy Workbench is an independently authored, self-hosted workflow-automation
platform with a Rust execution core, a browser editor, immutable workflow
revisions, bounded durable execution, recovery, and clean-room compatibility
seams. The default production profile remains one lightweight Rust daemon;
heavy and non-Rust workers are optional and isolated.

This file is the repository-level roadmap shown on GitHub. GitHub milestones
are the delivery roll-up; GitHub issues are the collaboration surface; detailed
contracts, acceptance criteria, and retained evidence remain versioned beside
the code under `.scratch/`, `docs/spec/`, and `docs/adr/`.

## Current position

As of 2026-09-15, the recovered Rust/Preact baseline is the implementation to
continue. Eco implementation Tickets 01–11 are complete. Ticket 11,
**Summarize the exact Eco 100K Run**, is backed by the all-green pinned
[validation run 34917759414](https://github.com/Catzpro01/n8n-rust-v3.0/actions/runs/34917759414),
including the Rust workspace, browser suites, exact Eco acceptance, dependency
audits, and release bundle. Ticket 12, **Recover Eco 100K after an ungraceful
daemon kill**, is the next core frontier; this roadmap update does not begin
that ticket.

The Eco implementation ticket number is separate from GitHub issue numbering.
In particular, GitHub issue
[#11](https://github.com/Catzpro01/n8n-rust-v3.0/issues/11) is the AI Agent
contract decision, resolved by
[ADR 0061](docs/adr/0061-make-agent-turns-durable-and-capability-bound.md).

## Delivery milestones

| Milestone | Scope | Exit condition | Current state |
| --- | --- | --- | --- |
| [M1 — Core: Eco 100K First Runnable](https://github.com/Catzpro01/n8n-rust-v3.0/milestone/1) | Durable authoring, exact bounded execution, crash recovery, resource governance, evidence retention, signed packaging, and first compatibility proof. | Eco release candidate passes the complete deterministic, recovery, resource, editor, compatibility, packaging, and restore evidence gate. | In progress; Tickets 01–11 complete, Ticket 12 next. |
| [M2 — Extension Foundation](https://github.com/Catzpro01/n8n-rust-v3.0/milestone/2) | Separate Node Contract, Node Implementation, Node Form, and Execution Lane identities; add versioned External Process/WASM boundaries and promotion evidence. | At least one non-native binding proves the locked plan, capability, budget, outcome, and conformance seams without burdening the default daemon. | Decision accepted in [ADR 0059](docs/adr/0059-language-bindings-behind-contract-and-lane-boundary.md); implementation follows M1 dependencies. |
| [M3 — Workflow & Skill Hub](https://github.com/Catzpro01/n8n-rust-v3.0/milestone/3) | Content-addressed Workflow and Skill Packages, review, sandboxing, explicit scope, side-by-side updates, safe retirement, and immutable dependency locks. | A package can move from discovery through review and sandbox to a newly published revision, then update or retire without changing retained Runs or revisions. | Lifecycle accepted in [ADR 0060](docs/adr/0060-keep-hub-package-installation-reviewable-and-scoped.md). |
| [M4 — AI Agent Platform](https://github.com/Catzpro01/n8n-rust-v3.0/milestone/4) | Durable capability-bound Agent Turns, locked Agent Blueprints, Model Routes, bounded memory, Skill/MCP grants, validated output, and optional engine adapters. | A deterministic first Agent Node vertical slice proves suspension, budgets, typed outcomes, redacted trace, recovery, and the safe pre-side-effect fallback rule. | Contract accepted in [ADR 0061](docs/adr/0061-make-agent-turns-durable-and-capability-bound.md); production runtime remains future work. |
| [M5 — Production Upgrade & Recovery](https://github.com/Catzpro01/n8n-rust-v3.0/milestone/5) | Signed Current/Previous release slots, complete incremental Recovery Sets, migration traffic boundaries, quarantine, rollback, and isolated restore drills. | Core, Hub, and Agent state survive verified upgrade/rollback and a documented restore drill without silent lock or evidence loss. | Boundary accepted in [ADR 0062](docs/adr/0062-preserve-extension-state-across-verified-release-slots.md). |
| [M6 — Large Editor, Compatibility & Release](https://github.com/Catzpro01/n8n-rust-v3.0/milestone/6) | 100,000-Node-Instance editor proof, staged clean-room compatibility, integrated private-first acceptance, and release evidence. | The deterministic and external evidence lanes pass the integrated production gate, with compatibility claims limited to tested profiles. | Gate accepted in [ADR 0063](docs/adr/0063-use-a-private-first-dual-lane-expansion-gate.md). |

Milestones intentionally have no invented calendar dates. Due dates should be
added only when the Owner commits to a delivery window.

## Dependency order

```text
M1 Core
  -> M2 Extension Foundation
  -> M3 Workflow & Skill Hub
  -> M4 AI Agent Platform
  -> M5 Production Upgrade & Recovery
  -> M6 Large Editor, Compatibility & Release
```

Some decision work is already accepted ahead of implementation so later
contracts do not destabilize the core. Production implementation still follows
the dependency order, and no milestone is complete from design documents alone.

## Detailed maps

- [Eco 100K implementation map](.scratch/eco-100k-first-runnable/map.md)
- [Full platform expansion map](.scratch/canopy-platform-expansion/map.md)
- [First runnable product specification](.scratch/first-runnable-platform/spec.md)
- [Project status and verification frontier](docs/agents/PROJECT-STATUS.md)
- [Canonical language and retained decisions](CONTEXT.md)
- [Clean-room policy](docs/legal/clean-room-policy.md)

## GitHub Project layout

The intended GitHub Project is **Canopy Workbench Roadmap** with this short
description:

> Delivery board for the Rust-first Canopy Workbench platform, from the Eco
> 100K core through extension lanes, Hub packages, AI Agents, recovery, large
> editor proof, compatibility, and release evidence.

Recommended fields and views:

- **Status:** Backlog, Ready, In progress, In review, Blocked, Done;
- **Milestone/Phase:** the six milestones above;
- **Work type:** Decision, Implementation, Verification, Documentation,
  Infrastructure;
- **Evidence:** link to the pinned run, ADR, fixture, or release artifact;
- **Roadmap view:** grouped by Milestone/Phase;
- **Current view:** Ready, In progress, In review, and Blocked;
- **Evidence view:** items awaiting or carrying release evidence.

GitHub Projects v2 is account-owned rather than repository-owned. Creating the
board therefore requires Projects write permission on the `Catzpro01` account;
once that permission is available, this section is the board description and
configuration to publish.

## Guardrails

- Preserve the recovered Rust core and connected Preact editor; evolve them
  through explicit seams rather than restarting.
- Keep the implementation clean-room: do not copy n8n source, Enterprise code,
  tests, assets, product copy, icons, branding, or distinctive trade dress.
- Pin Workflow Revisions, Execution Plans, contracts, packages, blueprints,
  capabilities, budgets, and worker identities; mutable “latest” state cannot
  alter an active Run.
- Require production code, externally observable tests, documentation, and
  retained release/recovery evidence before completing a delivery milestone.
- Keep credentials, private keys, recovery material, and plaintext secrets out
  of GitHub issues, Projects, commits, and artifacts.
