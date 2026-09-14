---
name: axe-core-a11y
description: Run automated and manual accessibility checks targeting WCAG 2.2 AA behavior.
mode: arena-workspace
auto_activate: true
triggers:
  - "accessibility"
  - "aksesibilitas"
  - "a11y"
  - "WCAG"
  - "ARIA"
  - "keyboard"
  - "contrast"
  - "kontras"
source: https://github.com/dequelabs/axe-core
source_license: MPL-2.0
audited_at: 2026-09-11
---

# axe-core-a11y

## Use when
Use for every meaningful web UI change and before release.

## Workflow
1. Exercise real states and routes in a real browser, not only static HTML.
2. Use an already-installed accessibility runner when available; otherwise request approval before adding an exact-pinned development dependency.
3. Run axe rules at relevant viewports and store machine-readable violations.
4. Manually test keyboard-only use, focus visibility/order, dialogs, skip paths, live updates, zoom/reflow, reduced motion, labels, errors, and contrast.
5. Fix root semantics before adding ARIA and add regression tests for verified failures.

## Deliverables
- Automated results, manual checklist, source fixes, and known limitations.

## Guardrails
A zero-violation axe report is not proof of WCAG conformance. Never suppress a rule without a documented, reviewed reason. Axe remains development/test tooling unless explicitly needed at runtime.

## Completion gate
Do not call the work complete until requested artifacts exist, relevant validation has run, and limitations are stated plainly.
