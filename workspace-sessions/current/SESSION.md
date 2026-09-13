# Current session handoff

**Started:** 2026-09-13
**Branch:** `arena/01a09a2a-n8n-rust-v3-0`
**Previous handoff:** `workspace-sessions/previous/SESSION.md`
**Active map:** `.scratch/canopy-platform-expansion/map.md`
**Active decision:** GitHub issue #10, “Decide the Workflow and Skill Package lifecycle” (issue #9 resolved as ADR 0059)

## Owner direction

- Implement all explicitly recorded workspace designs, not arbitrary full n8n
  parity.
- Follow the phase order: core completion; extension foundation;
  Workflow/Skill Hub; AI Agent platform; production upgrade/recovery; large
  editor and compatibility.
- A phase is complete only with production code, versioned contracts, UI where
  applicable, tests, documentation, and release evidence.
- Keep the recovered Rust/Preact baseline and do not silently broaden accepted
  ADRs or Ticket 08.

## Current state

- The full expansion Wayfinder map is published as GitHub issue #8.
- Issue #9 is resolved as ADR 0059: Contract, Implementation, Form, and Lane
  stay separate; Rust is the in-process promotion path; C++ and other languages
  use the common External Process/WASM boundary; Plans pin exact identities.
- Frontier decision tickets are #10 (Workflow/Skill Package lifecycle), #11
  (AI Agent/Agent Engine), and #14 (external adapter research). No production
  code for Hub, Skill Hub, or AI Agent has been started.

## Next action

Resolve the next HITL frontier ticket, issue #10, while keeping the core
implementation and language-neutral extension boundary intact. Create the
implementation spec/tickets only after the remaining decisions and integrated
release gate are accepted.
