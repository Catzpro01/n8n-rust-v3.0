---
name: design-md-tokens
description: Create a plain-text DESIGN.md and synchronized CSS/JSON design tokens for agent and human use.
mode: arena-workspace
auto_activate: true
triggers:
  - "design system"
  - "sistem desain"
  - "design token"
  - "token desain"
  - "theme"
  - "tema"
  - "palette"
  - "palet"
  - "typography"
  - "tipografi"
source: https://github.com/VoltAgent/awesome-design-md
source_license: MIT
audited_at: 2026-09-11
---

# design-md-tokens

## Use when
Use to establish or normalize a product's visual language before broad UI work.

## Workflow
1. Audit existing interfaces and identify reusable intent instead of copying a famous brand.
2. Define primitive tokens, then semantic roles, then component aliases; avoid components reading raw palette values.
3. Cover color, typography, spacing, radius, border, elevation, motion, breakpoints, density, focus, disabled, success, warning, and danger.
4. Generate synchronized `DESIGN.md`, `tokens.css`, and `tokens.json` where useful.
5. Include do/don't rules, responsive behavior, contrast targets, and a migration map from existing values.

## Deliverables
- Plain-text design contract plus machine-readable token exports and validation notes.

## Guardrails
The user-supplied `kaushikgopal/awesome-design-md` URL returned 404 during installation; this wrapper uses the active VoltAgent collection only as a reference. Never clone another product's distinctive trade dress or claim extracted public CSS values are an official design system.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
