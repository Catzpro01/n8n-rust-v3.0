---
name: penpot
description: Prepare open design-system handoff artifacts, vector assets, tokens, components, and CSS-ready specifications.
mode: arena-workspace
auto_activate: true
triggers:
  - "penpot"
  - "design handoff"
  - "handoff desain"
  - "vector handoff"
source: https://github.com/penpot/penpot
source_license: MPL-2.0
audited_at: 2026-09-11
---

# penpot

## Use when
Use for UI/UX system planning, component libraries, reusable vector assets, design-token handoff, or preparation for Penpot import.

## Workflow
1. Inspect the product's existing visual language and framework before proposing a new one.
2. Define primitive, semantic, and component tokens with light/dark and interaction states.
3. Specify component anatomy, variants, constraints, keyboard behavior, responsive rules, and content limits.
4. Produce original SVG assets and CSS/JSON token exports that can be imported or recreated in Penpot.
5. Supply a Penpot handoff checklist: pages, boards, components, variants, tokens, exports, and naming.

## Deliverables
- `DESIGN.md`, token files, component/state matrix, and SVG assets as needed.
- Penpot import/recreation notes; native Penpot files only when a supported export path is actually available.

## Guardrails
Penpot is a large application, not embedded by this skill. Never claim to have edited a remote Penpot workspace without credentials and a successful API/plugin operation. Keep third-party assets and licenses traceable.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
