---
name: shadcn-ui-engine
description: Design and implement accessible, composable UI components using the host project's own stack.
mode: arena-workspace
auto_activate: true
triggers:
  - "component"
  - "komponen"
  - "form"
  - "dialog"
  - "menu"
  - "table"
  - "tabel"
  - "button"
source: https://github.com/shadcn-ui/ui
source_license: MIT
audited_at: 2026-09-11
---

# shadcn-ui-engine

## Use when
Use for forms, dialogs, menus, tables, cards, command surfaces, and reusable application components.

## Workflow
1. Define semantics, states, events, focus order, keyboard behavior, loading/error/empty states, and content constraints.
2. Reuse the host project's primitives and tokens before introducing dependencies.
3. Keep public APIs small, typed, composable, and controlled where state ownership matters.
4. Implement semantic HTML and accessible names first; add visual treatment after behavior works.
5. Test keyboard navigation, reduced motion, high zoom, narrow screens, and automated accessibility.

## Deliverables
- Original component source, examples, and focused tests.

## Guardrails
The name describes a workflow, not an installed shadcn CLI. Add shadcn/Radix/Tailwind packages only with explicit user approval, exact version pinning, license review, and bundle-impact measurement. Do not paste a component merely because it looks familiar.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
