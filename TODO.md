# Canopy Workbench (n8n-rust) Implementation Backlog

## Ralph Loop Active Sprint: Milestone 1 Completion

- [x] Ticket 01–12: Core Daemon, SQLite WAL, Durable Draft, Manual Trigger, Streaming, Expressions, If/Merge, Summarize, Crash Recovery (PR #19)
- [x] Ticket 13: Govern bounded work and scale across cgroup CPU profiles (PR #20)
- [x] Ticket 14: Prove the 100,000-Node-Instance editor seam (PR #20)
- [x] Ticket 15: Import the first n8n 2.39.0 compatibility subset (PR #20)
- [x] Ticket 16: Complete the critical journey without relying on Canvas or one browser (Keyboard & Screen-reader WCAG 2.2 AA)
- [x] Ticket 17: Retain, pin, compact, and expire evidence safely (Storage quotas, Pins, Compaction)
- [x] Ticket 18: Create verified Recovery Sets and Quarantine Mode
- [x] Ticket 19: Activate signed Release Slots and roll back failed upgrades
- [x] Ticket 20: Run the identical signed bundle with Podman and Docker
- [x] Ticket 21: Certify the Eco 100K first runnable release candidate

## Completed Milestones
- [x] Milestone 1: Eco 100K Core (Tickets 01–21 certified v0.1.0-rc1)
- [x] Milestone 2: Extension Foundation (ADR 0059)
  - [x] Node Contract vs Implementation vs Form vs Lane separation (`NodeForm`, `ExecutionLane`, `NodeImplementationLock`)
  - [x] Versioned Framed External Process Protocol (`canopy.lane-protocol/v1alpha1`, stdio IPC, capability grants, bounds)
  - [x] Sandboxed WASM Lane boundary (`wasm_lane.rs`, memory & instruction budgeting, host imports/exports)
  - [x] Compiler cheapest-valid-lane selection and plan locking (`compiler.rs`, isolation downgrade rejection)
  - [x] Editor Extension inspection UI card and TypeScript typing (`editing-types.ts`, `large-editor.tsx`)
  - [x] Conformance and acceptance verification suite (Editor 32/32 tests green, Python 5/5 tests green)
- [x] Milestone 3: Workflow & Skill Hub (ADR 0060)
  - [x] Content-addressed Workflow & Skill Package manifests (`hub.rs`)
  - [x] Sandbox exercise before installation (zero-credential, side-effect-free)
  - [x] Scoped dependencies (Workflow-local default, Project, Global)
  - [x] Side-by-side candidate diffs (graph, config, capability, cost)
  - [x] Safe package retirement without orphan data or breaking historical runs
- [x] Milestone 4: AI Agent Platform (ADR 0061)
  - [x] Six typed subports: Engine, Memory, Skill, MCP, Policy, Output (`agent.rs`)
  - [x] Locked Agent Blueprints & Model Routes
  - [x] Durable capability-bound Agent Turns with checkpoints
  - [x] Ordered pre-side-effect fallback rule & post-side-effect uncertain classification
  - [x] Bounded Output Contract validation before downstream emission
- [x] Milestone 5: Production Upgrade & Recovery (ADR 0062)
  - [x] Signed release slots (`release_slots.rs`) with extension & hub lock preservation
  - [x] Preflight verification & automatic pre-traffic rollback
  - [x] Verified recovery sets, restore drills, and quarantine mode
- [x] Milestone 6: Large Editor, Compatibility & Release (ADR 0063)
  - [x] 100,000-Node-Instance virtualized canvas proof (`large-editor.tsx`, 33MB packed binary topology)
  - [x] Integrated private-first dual-lane expansion gate (`test_dual_lane_expansion_gate.py`)
  - [x] Clean-room n8n 2.39.0 compatibility surface with secret redaction
  - [x] Complete verification suite (Node 37/37 green, Python 25/25 green)
